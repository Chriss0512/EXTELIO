//! API: Telephony-, Routing- und Organisationsobjekte (Kapitel 8).

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use rand::RngCore;
use serde_json::{json, Value};
use std::collections::HashSet;

use super::http::require_auth;
use super::state::{AppState, AuthContext};
use crate::audit::Event;
use crate::domain::ids;
use crate::domain::numbers;
use crate::domain::routing::RouteGraph;
use crate::infrastructure::time::now_rfc3339;
use crate::kms::SecretClass;
use crate::{Error, Result};

fn field<'a>(body: &'a Value, name: &str) -> Result<&'a str> {
    body.get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| Error::Validation(format!("Feld '{name}' fehlt.")))
}

fn opt<'a>(body: &'a Value, name: &str) -> Option<&'a str> {
    body.get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
}

/// Erzeugt ein starkes SIP-Passwort (Kapitel 5.4: keine schwachen Credentials).
fn strong_secret() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut raw = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    raw.iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

fn audit(
    state: &AppState,
    ctx: &AuthContext,
    action: &str,
    object: (&str, &str),
    detail: Value,
) -> Result<()> {
    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, action)
            .object(object.0, object.1)
            .detail(detail),
    )?;
    Ok(())
}

// ---------------------------------------------------------------- Sites

pub async fn list_sites(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare("SELECT id,name,timezone,created_at FROM sites ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "timezone": r.get::<_, String>(2)?,
            "created_at": r.get::<_, String>(3)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_site(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("telephony.site.create")?;
    let name = field(&body, "name")?;
    let tz = opt(&body, "timezone").unwrap_or("Europe/Berlin");
    let id = ids::new_prefixed("site");
    {
        let guard = state.db.lock();
        guard.execute(
            "INSERT INTO sites (id,name,timezone,created_at) VALUES (?1,?2,?3,?4)",
            rusqlite::params![id, name, tz, now_rfc3339()],
        )?;
    }
    audit(
        &state,
        &ctx,
        "site.create",
        ("site", &id),
        json!({ "name": name }),
    )?;
    Ok(Json(json!({ "id": id })))
}

// ----------------------------------------------------------- Extensions

pub async fn list_extensions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT e.id,e.number,e.name,e.kind,e.site_id,e.user_id,e.voicemail,e.dnd,
                e.forward_target,e.outbound_caller_id,
                (SELECT COUNT(*) FROM device_lines dl WHERE dl.extension_id=e.id) AS devices
         FROM extensions e ORDER BY e.number",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "number": r.get::<_, String>(1)?,
            "name": r.get::<_, String>(2)?,
            "kind": r.get::<_, String>(3)?,
            "site_id": r.get::<_, Option<String>>(4)?,
            "user_id": r.get::<_, Option<String>>(5)?,
            "voicemail": r.get::<_, i64>(6)? == 1,
            "dnd": r.get::<_, i64>(7)? == 1,
            "forward_target": r.get::<_, Option<String>>(8)?,
            "outbound_caller_id": r.get::<_, Option<String>>(9)?,
            "device_count": r.get::<_, i64>(10)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

/// Legt eine Nebenstelle an und erzeugt dabei ein SIP-Credential.
/// Das Passwort wird sofort im KMS versiegelt und nur einmal zurueckgegeben.
pub async fn create_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("telephony.extension.create")?;

    let number = numbers::parse_extension(field(&body, "number")?)?;
    let name = field(&body, "name")?;
    let kind = opt(&body, "kind").unwrap_or("user");
    let voicemail = body
        .get("voicemail")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let id = ids::new_prefixed("ext");
    let sip_password = strong_secret();
    let realm = if state.options.canonical_hostname.is_empty() {
        "pbx.local".to_string()
    } else {
        state.options.canonical_hostname.clone()
    };
    let secret_id = state.with_kms(|k| {
        k.store(
            &state.db,
            SecretClass::SipCredential,
            &format!("sip:{number}"),
            "extension",
            sip_password.as_bytes(),
        )
    })?;

    {
        let guard = state.db.lock();
        guard
            .execute(
                "INSERT INTO extensions (id,number,name,kind,site_id,user_id,voicemail,outbound_caller_id,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
                rusqlite::params![
                    id, number, name, kind,
                    opt(&body, "site_id"), opt(&body, "user_id"),
                    voicemail as i64, opt(&body, "outbound_caller_id"), now_rfc3339()
                ],
            )
            .map_err(|_| Error::Conflict(format!("Die Nebenstelle {number} ist bereits vergeben.")))?;
        guard.execute(
            "INSERT INTO sip_credentials (id,username,realm,secret_id,extension_id,profile,created_at)
             VALUES (?1,?2,?3,?4,?5,'local',?6)",
            rusqlite::params![
                ids::new_prefixed("cred"), number, realm, secret_id, id, now_rfc3339()
            ],
        )?;
        if voicemail {
            guard.execute(
                "INSERT INTO voicemail_boxes (id,extension_id,created_at) VALUES (?1,?2,?3)",
                rusqlite::params![ids::new_prefixed("vm"), id, now_rfc3339()],
            )?;
        }
    }

    audit(
        &state,
        &ctx,
        "extension.create",
        ("extension", &id),
        json!({ "number": number }),
    )?;
    audit(
        &state,
        &ctx,
        "secret.create",
        ("secret", &secret_id),
        json!({ "class": "SIP_CREDENTIAL" }),
    )?;

    Ok(Json(json!({
        "id": id,
        "number": number,
        "sip_username": number,
        "sip_realm": realm,
        // Einmalige Anzeige zur Geraeteeinrichtung; danach nur ueber Rotation.
        "sip_password": sip_password,
    })))
}

pub async fn update_extension(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("telephony.extension.update")?;
    {
        let guard = state.db.lock();
        let n = guard.execute(
            "UPDATE extensions SET
               name = COALESCE(?2,name),
               voicemail = COALESCE(?3,voicemail),
               dnd = COALESCE(?4,dnd),
               forward_target = ?5,
               outbound_caller_id = COALESCE(?6,outbound_caller_id),
               updated_at = ?7
             WHERE id = ?1",
            rusqlite::params![
                id,
                opt(&body, "name"),
                body.get("voicemail")
                    .and_then(|v| v.as_bool())
                    .map(|b| b as i64),
                body.get("dnd").and_then(|v| v.as_bool()).map(|b| b as i64),
                opt(&body, "forward_target"),
                opt(&body, "outbound_caller_id"),
                now_rfc3339()
            ],
        )?;
        if n == 0 {
            return Err(Error::NotFound("Nebenstelle nicht gefunden.".into()));
        }
    }
    audit(
        &state,
        &ctx,
        "extension.update",
        ("extension", &id),
        body.clone(),
    )?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_extension(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("telephony.extension.delete")?;

    // Zugehoerige Secrets kryptografisch loeschen (Kapitel 7.5).
    let secret_ids: Vec<String> = {
        let guard = state.db.lock();
        let mut stmt =
            guard.prepare("SELECT secret_id FROM sip_credentials WHERE extension_id=?1")?;
        let rows = stmt.query_map(rusqlite::params![id], |r| r.get::<_, String>(0))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    for s in &secret_ids {
        state.with_kms(|k| k.crypto_erase(&state.db, s))?;
        audit(&state, &ctx, "secret.delete", ("secret", s), json!({}))?;
    }
    {
        let guard = state.db.lock();
        guard.execute("DELETE FROM extensions WHERE id=?1", rusqlite::params![id])?;
    }
    audit(
        &state,
        &ctx,
        "extension.delete",
        ("extension", &id),
        json!({}),
    )?;
    Ok(Json(json!({ "ok": true })))
}

// -------------------------------------------------------------- Devices

pub async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT id,name,vendor,model,mac,device_type,provisioning_mode,enrollment_state,
                firmware,ip,status,last_seen_at FROM devices ORDER BY name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "vendor": r.get::<_, String>(2)?,
            "model": r.get::<_, String>(3)?,
            "mac": r.get::<_, Option<String>>(4)?,
            "device_type": r.get::<_, String>(5)?,
            "provisioning_mode": r.get::<_, String>(6)?,
            "enrollment_state": r.get::<_, String>(7)?,
            "firmware": r.get::<_, Option<String>>(8)?,
            "ip": r.get::<_, Option<String>>(9)?,
            "status": r.get::<_, String>(10)?,
            "last_seen_at": r.get::<_, Option<String>>(11)?,
        }))
    })?;
    let mut items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;

    let mut stmt = guard.prepare(
        "SELECT dl.device_id, dl.id, dl.line_no, dl.extension_id, e.number
         FROM device_lines dl LEFT JOIN extensions e ON e.id=dl.extension_id ORDER BY dl.line_no",
    )?;
    let lines = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            json!({
                "id": r.get::<_, String>(1)?,
                "line_no": r.get::<_, i64>(2)?,
                "extension_id": r.get::<_, Option<String>>(3)?,
                "extension_number": r.get::<_, Option<String>>(4)?,
            }),
        ))
    })?;
    let mut by_device: std::collections::HashMap<String, Vec<Value>> = Default::default();
    for row in lines {
        let (dev, line) = row?;
        by_device.entry(dev).or_default().push(line);
    }
    for item in &mut items {
        let id = item["id"].as_str().unwrap_or_default().to_string();
        item["lines"] = json!(by_device.remove(&id).unwrap_or_default());
    }
    Ok(Json(json!({ "items": items })))
}

pub async fn create_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("device.create")?;
    let name = field(&body, "name")?;
    let id = ids::new_prefixed("dev");
    let mac = opt(&body, "mac").map(|m| m.to_uppercase().replace([':', '-'], ""));
    if let Some(m) = &mac {
        if m.len() != 12 || !m.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Validation(
                "Die MAC-Adresse ist nicht gültig.".into(),
            ));
        }
    }
    {
        let guard = state.db.lock();
        guard
            .execute(
                "INSERT INTO devices (id,name,vendor,model,mac,device_type,provisioning_mode,site_id,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
                rusqlite::params![
                    id, name,
                    opt(&body, "vendor").unwrap_or("generic"),
                    opt(&body, "model").unwrap_or("generic-sip"),
                    mac,
                    opt(&body, "device_type").unwrap_or("desk_phone"),
                    opt(&body, "provisioning_mode").unwrap_or("manual"),
                    opt(&body, "site_id"),
                    now_rfc3339()
                ],
            )
            .map_err(|_| Error::Conflict("Ein Gerät mit dieser MAC-Adresse existiert bereits.".into()))?;

        if let Some(ext_id) = opt(&body, "extension_id") {
            let cred: Option<String> = guard
                .query_row(
                    "SELECT id FROM sip_credentials WHERE extension_id=?1 AND profile='local'",
                    rusqlite::params![ext_id],
                    |r| r.get(0),
                )
                .ok();
            guard.execute(
                "INSERT INTO device_lines (id,device_id,line_no,extension_id,credential_id)
                 VALUES (?1,?2,1,?3,?4)",
                rusqlite::params![ids::new_prefixed("line"), id, ext_id, cred],
            )?;
        }
    }
    audit(
        &state,
        &ctx,
        "device.create",
        ("device", &id),
        json!({ "name": name }),
    )?;
    Ok(Json(json!({ "id": id })))
}

pub async fn update_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("device.update")?;
    {
        let guard = state.db.lock();
        let n = guard.execute(
            "UPDATE devices SET name=COALESCE(?2,name), vendor=COALESCE(?3,vendor),
                model=COALESCE(?4,model), provisioning_mode=COALESCE(?5,provisioning_mode),
                enrollment_state=COALESCE(?6,enrollment_state), updated_at=?7 WHERE id=?1",
            rusqlite::params![
                id,
                opt(&body, "name"),
                opt(&body, "vendor"),
                opt(&body, "model"),
                opt(&body, "provisioning_mode"),
                opt(&body, "enrollment_state"),
                now_rfc3339()
            ],
        )?;
        if n == 0 {
            return Err(Error::NotFound("Gerät nicht gefunden.".into()));
        }
    }
    audit(&state, &ctx, "device.update", ("device", &id), body.clone())?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("device.delete")?;
    {
        let guard = state.db.lock();
        guard.execute("DELETE FROM devices WHERE id=?1", rusqlite::params![id])?;
    }
    audit(&state, &ctx, "device.delete", ("device", &id), json!({}))?;
    Ok(Json(json!({ "ok": true })))
}

// -------------------------------------------------------------- Numbers

pub async fn list_numbers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT n.id,n.canonical,n.display,n.source_repr,n.number_type,n.country,
                n.trunk_id,t.name,n.route_graph_id,g.name
         FROM numbers n
         LEFT JOIN trunks t ON t.id=n.trunk_id
         LEFT JOIN route_graphs g ON g.id=n.route_graph_id
         ORDER BY n.canonical",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "canonical": r.get::<_, String>(1)?,
            "display": r.get::<_, String>(2)?,
            "source_repr": r.get::<_, String>(3)?,
            "number_type": r.get::<_, String>(4)?,
            "country": r.get::<_, Option<String>>(5)?,
            "trunk_id": r.get::<_, Option<String>>(6)?,
            "trunk_name": r.get::<_, Option<String>>(7)?,
            "route_graph_id": r.get::<_, Option<String>>(8)?,
            "flow_name": r.get::<_, Option<String>>(9)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_number(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("number.create")?;
    let parsed = numbers::parse_external(
        field(&body, "number")?,
        opt(&body, "country").unwrap_or("DE"),
    )?;
    let id = ids::new_prefixed("num");
    {
        let guard = state.db.lock();
        guard
            .execute(
                "INSERT INTO numbers (id,canonical,display,source_repr,number_type,country,trunk_id,route_graph_id,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![
                    id, parsed.canonical, parsed.display, parsed.source_repr, parsed.number_type,
                    parsed.country, opt(&body, "trunk_id"), opt(&body, "route_graph_id"), now_rfc3339()
                ],
            )
            .map_err(|_| Error::Conflict("Diese Rufnummer ist bereits angelegt.".into()))?;
    }
    audit(
        &state,
        &ctx,
        "number.create",
        ("number", &id),
        json!({ "canonical": crate::infrastructure::log::mask_number(&parsed.canonical) }),
    )?;
    Ok(Json(
        json!({ "id": id, "canonical": parsed.canonical, "display": parsed.display }),
    ))
}

pub async fn update_number(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("number.update")?;
    {
        let guard = state.db.lock();
        let n = guard.execute(
            "UPDATE numbers SET trunk_id=?2, route_graph_id=?3 WHERE id=?1",
            rusqlite::params![id, opt(&body, "trunk_id"), opt(&body, "route_graph_id")],
        )?;
        if n == 0 {
            return Err(Error::NotFound("Rufnummer nicht gefunden.".into()));
        }
    }
    audit(&state, &ctx, "number.update", ("number", &id), json!({}))?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_number(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("number.delete")?;
    {
        let guard = state.db.lock();
        guard.execute("DELETE FROM numbers WHERE id=?1", rusqlite::params![id])?;
    }
    audit(&state, &ctx, "number.delete", ("number", &id), json!({}))?;
    Ok(Json(json!({ "ok": true })))
}

// ------------------------------------------------------ Provider/Trunks

pub async fn list_providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT id,key,name,registrar,transport,codecs,number_format,auth_capabilities,builtin
         FROM provider_profiles ORDER BY name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "key": r.get::<_, String>(1)?,
            "name": r.get::<_, String>(2)?,
            "registrar": r.get::<_, Option<String>>(3)?,
            "transport": r.get::<_, String>(4)?,
            "codecs": serde_json::from_str::<Value>(&r.get::<_, String>(5)?).unwrap_or(Value::Null),
            "number_format": r.get::<_, String>(6)?,
            "auth_capabilities": serde_json::from_str::<Value>(&r.get::<_, String>(7)?).unwrap_or(Value::Null),
            "builtin": r.get::<_, i64>(8)? == 1,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn list_trunks(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT t.id,t.name,t.provider_profile_id,p.name,t.mode,t.host,t.port,t.transport,
                t.auth_username,t.enabled,t.status,t.last_status_at,t.security_profile
         FROM trunks t JOIN provider_profiles p ON p.id=t.provider_profile_id ORDER BY t.name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "provider_profile_id": r.get::<_, String>(2)?,
            "provider_name": r.get::<_, String>(3)?,
            "mode": r.get::<_, String>(4)?,
            "host": r.get::<_, String>(5)?,
            "port": r.get::<_, i64>(6)?,
            "transport": r.get::<_, String>(7)?,
            "auth_username": r.get::<_, Option<String>>(8)?,
            "enabled": r.get::<_, i64>(9)? == 1,
            "status": r.get::<_, String>(10)?,
            "last_status_at": r.get::<_, Option<String>>(11)?,
            "security_profile": r.get::<_, String>(12)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_trunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("trunk.create")?;
    let name = field(&body, "name")?;
    let provider = field(&body, "provider_profile_id")?;
    let host = field(&body, "host")?;
    let mode = opt(&body, "mode").unwrap_or("register");

    let secret_id = match opt(&body, "password") {
        Some(pw) => Some(state.with_kms(|k| {
            k.store(
                &state.db,
                SecretClass::TrunkCredential,
                &format!("trunk:{name}"),
                "trunk",
                pw.as_bytes(),
            )
        })?),
        None => None,
    };
    if mode == "register" && (opt(&body, "auth_username").is_none() || secret_id.is_none()) {
        return Err(Error::Validation(
            "Für einen Trunk mit Registrierung werden Benutzername und Passwort benötigt.".into(),
        ));
    }

    let id = ids::new_prefixed("trk");
    {
        let guard = state.db.lock();
        guard
            .execute(
                "INSERT INTO trunks (id,name,provider_profile_id,mode,host,port,transport,auth_username,
                                     from_user,from_domain,secret_id,security_profile,enabled,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,1,?13,?13)",
                rusqlite::params![
                    id, name, provider, mode, host,
                    body.get("port").and_then(|v| v.as_i64()).unwrap_or(5060),
                    opt(&body, "transport").unwrap_or("udp"),
                    opt(&body, "auth_username"), opt(&body, "from_user"), opt(&body, "from_domain"),
                    secret_id, opt(&body, "security_profile").unwrap_or("strict"), now_rfc3339()
                ],
            )
            .map_err(|_| Error::Conflict("Ein Trunk mit diesem Namen existiert bereits.".into()))?;
    }
    audit(
        &state,
        &ctx,
        "trunk.create",
        ("trunk", &id),
        json!({ "name": name, "host": host }),
    )?;
    Ok(Json(json!({ "id": id })))
}

pub async fn update_trunk(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("trunk.update")?;

    if let Some(pw) = opt(&body, "password") {
        ctx.require_step_up("trunk.credential.update")?;
        let secret_id = state.with_kms(|k| {
            k.store(
                &state.db,
                SecretClass::TrunkCredential,
                &format!("trunk:{id}"),
                "trunk",
                pw.as_bytes(),
            )
        })?;
        let guard = state.db.lock();
        guard.execute(
            "UPDATE trunks SET secret_id=?2, updated_at=?3 WHERE id=?1",
            rusqlite::params![id, secret_id, now_rfc3339()],
        )?;
        drop(guard);
        audit(
            &state,
            &ctx,
            "secret.rotate",
            ("trunk", &id),
            json!({ "class": "TRUNK_CREDENTIAL" }),
        )?;
    }

    {
        let guard = state.db.lock();
        let n = guard.execute(
            "UPDATE trunks SET name=COALESCE(?2,name), host=COALESCE(?3,host),
               transport=COALESCE(?4,transport), auth_username=COALESCE(?5,auth_username),
               enabled=COALESCE(?6,enabled), updated_at=?7 WHERE id=?1",
            rusqlite::params![
                id,
                opt(&body, "name"),
                opt(&body, "host"),
                opt(&body, "transport"),
                opt(&body, "auth_username"),
                body.get("enabled")
                    .and_then(|v| v.as_bool())
                    .map(|b| b as i64),
                now_rfc3339()
            ],
        )?;
        if n == 0 {
            return Err(Error::NotFound("Trunk nicht gefunden.".into()));
        }
    }
    audit(&state, &ctx, "trunk.update", ("trunk", &id), json!({}))?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_trunk(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("trunk.delete")?;
    let secret: Option<String> = {
        let guard = state.db.lock();
        guard
            .query_row(
                "SELECT secret_id FROM trunks WHERE id=?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .ok()
            .flatten()
    };
    if let Some(s) = secret {
        state.with_kms(|k| k.crypto_erase(&state.db, &s))?;
    }
    {
        let guard = state.db.lock();
        guard.execute("DELETE FROM trunks WHERE id=?1", rusqlite::params![id])?;
    }
    audit(&state, &ctx, "trunk.delete", ("trunk", &id), json!({}))?;
    Ok(Json(json!({ "ok": true })))
}

// --------------------------------------------------------------- Groups

pub async fn list_groups(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard
        .prepare("SELECT id,name,strategy,members,timeout_s FROM ring_groups ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "strategy": r.get::<_, String>(2)?,
            "members": serde_json::from_str::<Value>(&r.get::<_, String>(3)?).unwrap_or(Value::Null),
            "timeout_s": r.get::<_, i64>(4)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("telephony.group.create")?;
    let name = field(&body, "name")?;
    let members = body.get("members").cloned().unwrap_or(json!([]));
    let id = ids::new_prefixed("grp");
    {
        let guard = state.db.lock();
        guard.execute(
            "INSERT INTO ring_groups (id,name,strategy,members,timeout_s,created_at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![
                id,
                name,
                opt(&body, "strategy").unwrap_or("simultaneous"),
                members.to_string(),
                body.get("timeout_s").and_then(|v| v.as_i64()).unwrap_or(25),
                now_rfc3339()
            ],
        )?;
    }
    audit(
        &state,
        &ctx,
        "group.create",
        ("ring_group", &id),
        json!({ "name": name }),
    )?;
    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------- Flows

pub async fn list_flows(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT id,name,graph,version,status,updated_at FROM route_graphs ORDER BY name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "graph": serde_json::from_str::<Value>(&r.get::<_, String>(2)?).unwrap_or(Value::Null),
            "version": r.get::<_, i64>(3)?,
            "status": r.get::<_, String>(4)?,
            "updated_at": r.get::<_, String>(5)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_flow(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("routing.flow.create")?;
    let name = field(&body, "name")?;
    let graph = body
        .get("graph")
        .cloned()
        .unwrap_or(json!({"nodes":[],"edges":[]}));
    let id = ids::new_prefixed("flw");
    {
        let guard = state.db.lock();
        guard.execute(
            "INSERT INTO route_graphs (id,name,graph,version,status,created_at,updated_at)
             VALUES (?1,?2,?3,1,'draft',?4,?4)",
            rusqlite::params![id, name, graph.to_string(), now_rfc3339()],
        )?;
    }
    audit(
        &state,
        &ctx,
        "flow.create",
        ("route_graph", &id),
        json!({ "name": name }),
    )?;
    Ok(Json(json!({ "id": id })))
}

pub async fn update_flow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("routing.flow.update")?;
    let graph = body
        .get("graph")
        .cloned()
        .ok_or_else(|| Error::Validation("Feld 'graph' fehlt.".into()))?;
    // Schemapruefung vor dem Speichern.
    let _: RouteGraph = serde_json::from_value(graph.clone())
        .map_err(|e| Error::Validation(format!("Der Flow ist strukturell ungültig: {e}")))?;
    {
        let guard = state.db.lock();
        let n = guard.execute(
            "UPDATE route_graphs SET name=COALESCE(?2,name), graph=?3, version=version+1,
                status='draft', updated_at=?4 WHERE id=?1",
            rusqlite::params![id, opt(&body, "name"), graph.to_string(), now_rfc3339()],
        )?;
        if n == 0 {
            return Err(Error::NotFound("Flow nicht gefunden.".into()));
        }
    }
    audit(&state, &ctx, "flow.update", ("route_graph", &id), json!({}))?;
    Ok(Json(json!({ "ok": true })))
}

fn known_refs(state: &AppState) -> Result<HashSet<String>> {
    let guard = state.db.lock();
    let mut set = HashSet::new();
    for sql in [
        "SELECT id FROM extensions",
        "SELECT id FROM numbers",
        "SELECT id FROM ring_groups",
        "SELECT id FROM queues",
        "SELECT id FROM ivrs",
        "SELECT id FROM schedules",
    ] {
        let mut stmt = guard.prepare(sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            set.insert(row?);
        }
    }
    Ok(set)
}

fn load_flow(state: &AppState, id: &str) -> Result<RouteGraph> {
    let raw: String = {
        let guard = state.db.lock();
        guard
            .query_row(
                "SELECT graph FROM route_graphs WHERE id=?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .map_err(|_| Error::NotFound("Flow nicht gefunden.".into()))?
    };
    Ok(serde_json::from_str(&raw)?)
}

pub async fn validate_flow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let graph = load_flow(&state, &id)?;
    let report = graph.validate(&known_refs(&state)?);
    let simulation = graph.simulate(&Default::default()).unwrap_or_default();
    Ok(Json(json!({
        "valid": report.valid,
        "errors": report.errors,
        "warnings": report.warnings,
        "simulation": simulation,
    })))
}

pub async fn publish_flow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("routing.flow.publish")?;
    let graph = load_flow(&state, &id)?;
    let report = graph.validate(&known_refs(&state)?);
    if !report.valid {
        return Err(Error::Validation(format!(
            "Der Flow kann nicht veröffentlicht werden: {}",
            report.errors.join("; ")
        )));
    }
    {
        let guard = state.db.lock();
        guard.execute(
            "UPDATE route_graphs SET status='published', updated_at=?2 WHERE id=?1",
            rusqlite::params![id, now_rfc3339()],
        )?;
    }
    audit(
        &state,
        &ctx,
        "flow.publish",
        ("route_graph", &id),
        json!({}),
    )?;
    Ok(Json(json!({ "ok": true, "warnings": report.warnings })))
}
