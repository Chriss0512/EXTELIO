//! API: Health, Konfigurationsgenerationen, Audit, Backup, Einstellungen.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

use super::http::require_auth;
use super::state::AppState;
use crate::audit::Event;
use crate::infrastructure::time::now_rfc3339;
use crate::{backup, compiler, health, Error, Result};

/// Oeffentlicher Liveness-Endpunkt. Gibt bewusst keine Details preis.
pub async fn public_health(State(state): State<AppState>) -> Json<Value> {
    let ok = state.db.integrity_ok();
    Json(json!({
        "status": if ok { "ok" } else { "error" },
        "version": crate::VERSION,
    }))
}

/// Vollstaendiger Health Report (Kapitel 18).
pub async fn system_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("health.read")
        .or_else(|_| ctx.require("system.read"))?;

    let expect_fs = std::env::var("EXTELIO_FREESWITCH_ENABLED")
        .map(|v| v == "1")
        .unwrap_or(false);
    let report = health::collect(&state.db, &state.paths, expect_fs);
    Ok(Json(json!({
        "state": report.state.as_str(),
        "ts": report.ts,
        "active_generation": report.active_generation,
        "checks": report.checks,
    })))
}

/// Kennzahlen fuer das Dashboard (Kapitel 2.13).
pub async fn dashboard(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_auth(&state, &headers)?;
    let expect_fs = std::env::var("EXTELIO_FREESWITCH_ENABLED")
        .map(|v| v == "1")
        .unwrap_or(false);
    let report = health::collect(&state.db, &state.paths, expect_fs);

    let guard = state.db.lock();
    let count = |sql: &str| -> i64 { guard.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    let extensions = count("SELECT COUNT(*) FROM extensions");
    let devices = count("SELECT COUNT(*) FROM devices");
    let devices_online = count("SELECT COUNT(*) FROM devices WHERE status='online'");
    let numbers = count("SELECT COUNT(*) FROM numbers");
    let trunks = count("SELECT COUNT(*) FROM trunks WHERE enabled=1");
    let trunks_registered = count("SELECT COUNT(*) FROM trunks WHERE status='registered'");
    let flows = count("SELECT COUNT(*) FROM route_graphs");
    let flows_draft = count("SELECT COUNT(*) FROM route_graphs WHERE status='draft'");
    let users = count("SELECT COUNT(*) FROM users WHERE status='active'");
    let calls_today = count(
        "SELECT COUNT(DISTINCT call_id) FROM call_events WHERE ts >= date('now') || 'T00:00:00Z'",
    );

    let mut recent = Vec::new();
    {
        let mut stmt = guard.prepare(
            "SELECT ts,action,outcome,actor_type FROM audit_log ORDER BY seq DESC LIMIT 8",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(json!({
                "ts": r.get::<_, String>(0)?,
                "action": r.get::<_, String>(1)?,
                "outcome": r.get::<_, String>(2)?,
                "actor_type": r.get::<_, String>(3)?,
            }))
        })?;
        for row in rows {
            recent.push(row?);
        }
    }
    drop(guard);

    Ok(Json(json!({
        "health": {
            "state": report.state.as_str(),
            "checks_total": report.checks.len(),
            "checks_failing": report.checks.iter()
                .filter(|c| c.state != health::HealthState::Healthy
                         && c.state != health::HealthState::Maintenance)
                .count(),
        },
        "counters": {
            "extensions": extensions,
            "devices": devices,
            "devices_online": devices_online,
            "numbers": numbers,
            "trunks": trunks,
            "trunks_registered": trunks_registered,
            "flows": flows,
            "flows_draft": flows_draft,
            "users": users,
            "calls_today": calls_today,
        },
        "active_generation": report.active_generation,
        "recent_events": recent,
    })))
}

#[derive(serde::Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    action: Option<String>,
}

pub async fn audit_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("audit.read")?;
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let items = state.audit.list(&state.db, limit, q.action.as_deref())?;
    Ok(Json(json!({ "items": items })))
}

pub async fn audit_verify(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("audit.read")?;
    let v = state.audit.verify(&state.db)?;
    Ok(Json(
        json!({ "valid": v.valid, "entries": v.entries, "broken_at": v.broken_at }),
    ))
}

// ------------------------------------------------- Konfigurationsgenerationen

pub async fn generations(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.read")
        .or_else(|_| ctx.require("health.read"))?;

    // Die Datenbanksperre wird bewusst vor dem naechsten Zugriff wieder
    // freigegeben: der Mutex ist nicht wiedereintrittsfaehig, ein zweiter
    // Zugriff bei gehaltener Sperre wuerde den Request dauerhaft blockieren.
    let items: Vec<Value> = {
        let guard = state.db.lock();
        let mut stmt = guard.prepare(
            "SELECT number,status,hash,created_by,created_at,activated_at,manifest
             FROM config_generations ORDER BY number DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |r| {
            let manifest: String = r.get(6)?;
            let parsed: Value = serde_json::from_str(&manifest).unwrap_or(Value::Null);
            Ok(json!({
                "number": r.get::<_, i64>(0)?,
                "status": r.get::<_, String>(1)?,
                "hash": r.get::<_, String>(2)?,
                "created_by": r.get::<_, Option<String>>(3)?,
                "created_at": r.get::<_, String>(4)?,
                "activated_at": r.get::<_, Option<String>>(5)?,
                "files": parsed.get("files").and_then(|f| f.as_array()).map(|a| a.len()).unwrap_or(0),
                "warnings": parsed.get("warnings").cloned().unwrap_or(json!([])),
            }))
        })?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    let active = compiler::active_generation(&state.db);
    Ok(Json(json!({ "items": items, "active": active })))
}

/// ESL-Secret. Wird beim ersten Kompilieren erzeugt und danach wiederverwendet.
fn esl_secret_id(state: &AppState) -> Result<String> {
    {
        let guard = state.db.lock();
        let existing: Option<String> = guard
            .query_row(
                "SELECT id FROM secrets WHERE class='SERVICE_TOKEN' AND label='esl' AND revoked_at IS NULL",
                [],
                |r| r.get(0),
            )
            .ok();
        if let Some(id) = existing {
            return Ok(id);
        }
    }
    use rand::RngCore;
    let mut raw = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let pw: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    state.with_kms(|k| {
        k.store(
            &state.db,
            crate::kms::SecretClass::ServiceToken,
            "esl",
            "system",
            pw.as_bytes(),
        )
    })
}

pub async fn compile(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.config.compile")
        .or_else(|_| ctx.require("routing.compile"))?;
    if backup::is_frozen() {
        return Err(Error::Conflict(
            "Während eines laufenden Backups wird nicht kompiliert.".into(),
        ));
    }

    let esl = esl_secret_id(&state)?;
    let adapter_url = std::env::var("EXTELIO_XML_ADAPTER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081/xml".into());

    let report = compiler::compile(
        &state.db,
        &state.options,
        &state.paths.config_dir(),
        &esl,
        &adapter_url,
        Some(&ctx.user_id),
    )?;

    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "config.compile")
            .object("generation", &report.generation.to_string())
            .detail(json!({ "hash": report.hash, "files": report.files })),
    )?;
    Ok(Json(json!({
        "generation": report.generation,
        "hash": report.hash,
        "files": report.files,
        "warnings": report.warnings,
    })))
}

pub async fn activate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.config.activate")
        .or_else(|_| ctx.require("routing.compile"))?;
    if backup::is_frozen() {
        return Err(Error::Conflict(
            "Während eines laufenden Backups wird nicht aktiviert.".into(),
        ));
    }

    let number = body
        .get("generation")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::Validation("Feld 'generation' fehlt.".into()))?;

    compiler::activate(&state.db, &state.paths.config_dir(), number)?;
    // Der Worker erkennt die neue Generation und stoesst den Reload an.
    std::fs::write(
        state.paths.state_dir().join("reload.request"),
        number.to_string(),
    )
    .ok();

    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "config.activate").object("generation", &number.to_string()),
    )?;
    Ok(Json(json!({ "ok": true, "generation": number })))
}

pub async fn rollback(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.config.activate")
        .or_else(|_| ctx.require("routing.compile"))?;
    let number = compiler::rollback(&state.db, &state.paths.config_dir())?;
    std::fs::write(
        state.paths.state_dir().join("reload.request"),
        number.to_string(),
    )
    .ok();
    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "config.rollback").object("generation", &number.to_string()),
    )?;
    Ok(Json(json!({ "ok": true, "generation": number })))
}

// -------------------------------------------------------------- Backup

pub async fn backup_list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.backup.read")
        .or_else(|_| ctx.require("system.read"))?;
    Ok(Json(json!({ "items": backup::list(&state.db)? })))
}

pub async fn backup_create(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.backup.create")
        .or_else(|_| ctx.require("system.backup"))?;
    let info =
        state.with_kms(|k| backup::create(&state.db, k, &state.paths.backup_dir(), "manual"))?;
    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "backup.create")
            .object("backup", &info.id)
            .detail(json!({ "size_bytes": info.size_bytes, "encrypted": info.encrypted })),
    )?;
    Ok(Json(json!({
        "id": info.id,
        "created_at": info.created_at,
        "size_bytes": info.size_bytes,
        "sha256": info.sha256,
    })))
}

// --------------------------------------------------------- Einstellungen

pub async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("system.read")
        .or_else(|_| ctx.require("settings.read"))?;

    let mut settings = serde_json::Map::new();
    let mut retention = Vec::new();
    {
        let guard = state.db.lock();
        {
            let mut stmt = guard.prepare("SELECT key,value FROM settings ORDER BY key")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows {
                let (k, v) = row?;
                settings.insert(k, serde_json::from_str(&v).unwrap_or(Value::String(v)));
            }
        }
        {
            let mut stmt =
                guard.prepare("SELECT key,purpose,days FROM retention_policies ORDER BY key")?;
            let rows = stmt.query_map([], |r| {
                Ok(json!({
                    "key": r.get::<_, String>(0)?,
                    "purpose": r.get::<_, String>(1)?,
                    "days": r.get::<_, i64>(2)?,
                }))
            })?;
            for row in rows {
                retention.push(row?);
            }
        }
    }

    Ok(Json(json!({
        "settings": settings,
        "retention_policies": retention,
        "runtime": {
            "version": crate::VERSION,
            "web_mode": state.options.web_mode,
            "canonical_hostname": state.options.canonical_hostname,
            "sip_ports": {
                "local": state.options.local_sip_port,
                "local_tls": state.options.local_sips_port,
                "public": state.options.public_sip_port,
                "public_tls": state.options.public_sips_port,
                "trunk": state.options.trunk_sip_port,
                "trunk_tls": state.options.trunk_sips_port,
            },
            "rtp_range": [state.options.rtp_start_port, state.options.rtp_end_port],
            "public_push_enabled": state.options.public_push_enabled,
            "ipv6_enabled": state.options.ipv6_enabled,
            "secure_context": state.options.secure_context_available(),
        }
    })))
}

pub async fn put_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("settings.update")
        .or_else(|_| ctx.require("system.update"))?;

    let map: HashMap<String, Value> =
        serde_json::from_value(body.get("settings").cloned().unwrap_or_else(|| json!({})))
            .map_err(|_| Error::Validation("Feld 'settings' muss ein Objekt sein.".into()))?;

    // Sicherheitsrelevante Umschaltungen verlangen Step-up (Kapitel 6.6).
    if map.contains_key("security.mode") {
        ctx.require_step_up("security.mode.change")?;
    }

    for (k, v) in &map {
        if k.starts_with("privacy.") && v.as_bool() == Some(true) && k.contains("ha_personal") {
            return Err(Error::Forbidden(
                "Personenbezogene Daten dürfen nicht an Home Assistant übertragen werden.".into(),
            ));
        }
        let guard = state.db.lock();
        guard.execute(
            "INSERT INTO settings (key,value,updated_at) VALUES (?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value=?2, updated_at=?3",
            rusqlite::params![k, v.to_string(), now_rfc3339()],
        )?;
    }

    if let Some(policies) = body.get("retention_policies").and_then(|v| v.as_array()) {
        for p in policies {
            let key = p.get("key").and_then(|v| v.as_str()).unwrap_or_default();
            let days = p.get("days").and_then(|v| v.as_i64()).unwrap_or(0);
            if key.is_empty() {
                continue;
            }
            if !(1..=3650).contains(&days) {
                return Err(Error::Validation(
                    "Die Aufbewahrungsdauer muss zwischen 1 und 3650 Tagen liegen.".into(),
                ));
            }
            let guard = state.db.lock();
            guard.execute(
                "UPDATE retention_policies SET days=?2, updated_at=?3 WHERE key=?1",
                rusqlite::params![key, days, now_rfc3339()],
            )?;
        }
    }

    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "settings.update")
            .detail(json!({ "keys": map.keys().collect::<Vec<_>>() })),
    )?;
    Ok(Json(json!({ "ok": true })))
}
