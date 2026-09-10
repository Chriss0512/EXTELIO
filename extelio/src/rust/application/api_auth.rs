//! API: Erstinbetriebnahme, Anmeldung, Sitzung, Benutzerverwaltung (Kapitel 6).

use axum::extract::State;
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use super::http::{client_ip, require_auth, transport_of};
use super::state::AppState;
use crate::audit::Event;
use crate::auth::{password, ratelimit, rbac, session, totp};
use crate::domain::identity::validate_username;
use crate::domain::ids;
use crate::infrastructure::time::now_rfc3339;
use crate::kms::SecretClass;
use crate::{Error, Result};

fn field<'a>(body: &'a Value, name: &str) -> Result<&'a str> {
    body.get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| Error::Validation(format!("Feld '{name}' fehlt.")))
}

/// Steuert den First-Boot-Wizard (Kapitel 20).
pub async fn setup_status(State(state): State<AppState>) -> Result<Json<Value>> {
    let needs_setup = !state.has_users();
    Ok(Json(json!({
        "needs_setup": needs_setup,
        "version": crate::VERSION,
        "secure_context": state.options.secure_context_available(),
        "web_mode": state.options.web_mode,
    })))
}

/// Legt den ersten Systemadministrator an. Nur moeglich, solange kein Benutzer
/// existiert (Kapitel 20: First Admin, danach kein offener Zugang).
pub async fn setup_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    if state.has_users() {
        return Err(Error::Conflict(
            "Die Erstinbetriebnahme ist bereits abgeschlossen.".into(),
        ));
    }
    let username = field(&body, "username")?.to_lowercase();
    let display_name = field(&body, "display_name")?;
    let pw = field(&body, "password")?;
    validate_username(&username)?;
    password::validate_strength(pw)?;

    let hash = password::hash(pw)?;
    let secret = totp::generate_secret();
    let secret_id = state.with_kms(|k| {
        k.store(
            &state.db,
            SecretClass::TotpSecret,
            &format!("totp:{username}"),
            "user",
            secret.as_bytes(),
        )
    })?;

    let user_id = ids::new_prefixed("usr");
    {
        let guard = state.db.lock();
        guard.execute(
            "INSERT INTO users
             (id,username,display_name,role_id,password_hash,password_algo,totp_secret_id,totp_enabled,status,created_at,updated_at)
             VALUES (?1,?2,?3,'role-sysadmin',?4,?5,?6,0,'pending_totp',?7,?7)",
            rusqlite::params![
                user_id, username, display_name, hash, password::PROFILE, secret_id, now_rfc3339()
            ],
        )?;
    }

    state.audit.record(
        &state.db,
        Event::system("setup.first_admin_created")
            .object("user", &user_id)
            .from_ip(client_ip(&state, &headers))
            .detail(json!({ "username": username })),
    )?;

    // Das Secret verlaesst den Server ausschliesslich hier, waehrend des
    // Enrollments, und wird danach nie wieder ausgegeben (Kapitel 7.3).
    Ok(Json(json!({
        "user_id": user_id,
        "totp_secret": secret,
        "provisioning_uri": totp::provisioning_uri(&secret, &username, "EXTELIO"),
    })))
}

/// Bestaetigt das TOTP-Enrollment und schliesst die Erstinbetriebnahme ab.
pub async fn setup_totp_confirm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let user_id = field(&body, "user_id")?;
    let code = field(&body, "code")?;

    let (secret_id, status): (String, String) = {
        let guard = state.db.lock();
        guard
            .query_row(
                "SELECT totp_secret_id, status FROM users WHERE id=?1",
                rusqlite::params![user_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| Error::NotFound("Benutzer nicht gefunden.".into()))?
    };
    if status != "pending_totp" {
        return Err(Error::Conflict(
            "Die Zwei-Faktor-Einrichtung ist bereits abgeschlossen.".into(),
        ));
    }

    let secret = state.with_kms(|k| k.load(&state.db, &secret_id))?;
    let secret = String::from_utf8(secret.to_vec())
        .map_err(|_| Error::Internal("TOTP-Secret unlesbar".into()))?;

    if !totp::verify(&secret, code, crate::infrastructure::time::unix_seconds())? {
        state.audit.record(
            &state.db,
            Event::system("setup.totp_failed")
                .object("user", user_id)
                .outcome("failure")
                .from_ip(client_ip(&state, &headers)),
        )?;
        return Err(Error::Unauthorized(
            "Der Code stimmt nicht. Bitte erneut versuchen.".into(),
        ));
    }

    {
        let guard = state.db.lock();
        guard.execute(
            "UPDATE users SET totp_enabled=1, status='active', updated_at=?2 WHERE id=?1",
            rusqlite::params![user_id, now_rfc3339()],
        )?;
    }
    state.audit.record(
        &state.db,
        Event::system("setup.totp_enrolled").object("user", user_id),
    )?;
    Ok(Json(json!({ "ok": true })))
}

/// Anmeldung mit Passwort und TOTP (Kapitel 6.1 Fallback-Pfad).
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    let username = field(&body, "username")?.to_lowercase();
    let pw = field(&body, "password")?;
    let code = body
        .get("totp")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let ip = client_ip(&state, &headers);
    let transport = transport_of(&state, &headers);

    ratelimit::check(&state.db, &username, ip.as_deref(), "password")?;

    let row: Option<(String, String, String, Option<String>, i64, String)> = {
        let guard = state.db.lock();
        guard
            .query_row(
                "SELECT id,password_hash,display_name,totp_secret_id,totp_enabled,status
                 FROM users WHERE username=?1",
                rusqlite::params![username],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .ok()
    };

    // Einheitliche Fehlermeldung, damit keine Account-Enumeration moeglich ist.
    let deny = || Error::Unauthorized("Anmeldung fehlgeschlagen.".into());

    let Some((user_id, hash, display_name, totp_secret_id, totp_enabled, status)) = row else {
        ratelimit::record_failure(&state.db, &username, ip.as_deref(), "password")?;
        state.audit.record(
            &state.db,
            Event::system("auth.login")
                .outcome("failure")
                .reason("unbekannter Benutzer")
                .from_ip(ip.clone()),
        )?;
        return Err(deny());
    };

    if !password::verify(pw, &hash) {
        let lock = ratelimit::record_failure(&state.db, &username, ip.as_deref(), "password")?;
        state.audit.record(
            &state.db,
            Event::user(&user_id, "auth.login")
                .outcome("failure")
                .reason("falsches Passwort")
                .detail(json!({ "failures": lock.failures, "locked": lock.locked }))
                .from_ip(ip.clone()),
        )?;
        return Err(deny());
    }

    if status != "active" {
        return Err(Error::Forbidden(
            "Das Benutzerkonto ist nicht aktiv.".into(),
        ));
    }

    if totp_enabled == 1 {
        let Some(secret_id) = totp_secret_id else {
            return Err(Error::Internal(
                "TOTP aktiviert, aber kein Secret hinterlegt".into(),
            ));
        };
        let secret = state.with_kms(|k| k.load(&state.db, &secret_id))?;
        let secret = String::from_utf8(secret.to_vec())
            .map_err(|_| Error::Internal("TOTP-Secret unlesbar".into()))?;
        if !totp::verify(&secret, code, crate::infrastructure::time::unix_seconds())? {
            let lock = ratelimit::record_failure(&state.db, &username, ip.as_deref(), "totp")?;
            state.audit.record(
                &state.db,
                Event::user(&user_id, "auth.login")
                    .outcome("failure")
                    .reason("falscher zweiter Faktor")
                    .detail(json!({ "failures": lock.failures }))
                    .from_ip(ip.clone()),
            )?;
            return Err(deny());
        }
    }

    // Passwortprofil bei Bedarf still migrieren.
    if password::needs_rehash(&hash) {
        if let Ok(new_hash) = password::hash(pw) {
            let guard = state.db.lock();
            let _ = guard.execute(
                "UPDATE users SET password_hash=?2, password_algo=?3 WHERE id=?1",
                rusqlite::params![user_id, new_hash, password::PROFILE],
            );
        }
    }

    ratelimit::record_success(&state.db, &username, ip.as_deref(), "password")?;
    let (token, _s) = session::create(
        &state.db,
        &user_id,
        transport,
        ip.as_deref(),
        headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok()),
    )?;
    {
        let guard = state.db.lock();
        guard.execute(
            "UPDATE users SET last_login_at=?2 WHERE id=?1",
            rusqlite::params![user_id, now_rfc3339()],
        )?;
    }
    state.audit.record(
        &state.db,
        Event::user(&user_id, "auth.login")
            .outcome("success")
            .from_ip(ip),
    )?;

    let cookie = session::cookie_header(transport, &token, session::DEFAULT_ABSOLUTE_HOURS * 3600);
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(json!({ "ok": true, "display_name": display_name })),
    )
        .into_response())
}

pub async fn session_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    Ok(Json(json!({
        "user_id": ctx.user_id,
        "username": ctx.username,
        "display_name": ctx.display_name,
        "role": ctx.role_key,
        "permissions": ctx.permissions,
        "step_up_fresh": session::step_up_fresh(&ctx.session),
        "idle_expires_at": ctx.session.idle_expires_at,
        "absolute_expires_at": ctx.session.absolute_expires_at,
    })))
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    let ctx = require_auth(&state, &headers)?;
    session::revoke(&state.db, &ctx.session.id)?;
    state
        .audit
        .record(&state.db, Event::user(&ctx.user_id, "auth.logout"))?;
    Ok((
        [(
            header::SET_COOKIE,
            session::clear_cookie_header(ctx.transport),
        )],
        Json(json!({ "ok": true })),
    )
        .into_response())
}

/// Step-up-Authentifizierung fuer kritische Aktionen (Kapitel 6.6).
pub async fn step_up(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    let ctx = require_auth(&state, &headers)?;
    let pw = field(&body, "password")?;
    let ip = client_ip(&state, &headers);

    ratelimit::check(&state.db, &ctx.username, ip.as_deref(), "step_up")?;
    let hash: String = {
        let guard = state.db.lock();
        guard.query_row(
            "SELECT password_hash FROM users WHERE id=?1",
            rusqlite::params![ctx.user_id],
            |r| r.get(0),
        )?
    };
    if !password::verify(pw, &hash) {
        ratelimit::record_failure(&state.db, &ctx.username, ip.as_deref(), "step_up")?;
        state.audit.record(
            &state.db,
            Event::user(&ctx.user_id, "auth.step_up")
                .outcome("failure")
                .from_ip(ip),
        )?;
        return Err(Error::Unauthorized(
            "Das Passwort ist nicht korrekt.".into(),
        ));
    }

    ratelimit::record_success(&state.db, &ctx.username, ip.as_deref(), "step_up")?;
    session::mark_step_up(&state.db, &ctx.session.id)?;
    // Session Rotation nach erfolgreicher Rechteerhoehung (Kapitel 6.4).
    let token = session::rotate(&state.db, &ctx.session.id)?;
    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "auth.step_up")
            .outcome("success")
            .from_ip(ip),
    )?;

    Ok((
        [(
            header::SET_COOKIE,
            session::cookie_header(
                ctx.transport,
                &token,
                session::DEFAULT_ABSOLUTE_HOURS * 3600,
            ),
        )],
        Json(json!({ "ok": true })),
    )
        .into_response())
}

pub async fn list_users(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("user.read")
        .or_else(|_| ctx.require("user.list"))?;

    let guard = state.db.lock();
    let mut stmt = guard.prepare(
        "SELECT u.id,u.username,u.display_name,u.email,r.key,u.status,u.totp_enabled,u.created_at,u.last_login_at
         FROM users u JOIN roles r ON r.id=u.role_id ORDER BY u.username",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "username": r.get::<_, String>(1)?,
            "display_name": r.get::<_, String>(2)?,
            "email": r.get::<_, Option<String>>(3)?,
            "role": r.get::<_, String>(4)?,
            "status": r.get::<_, String>(5)?,
            "totp_enabled": r.get::<_, i64>(6)? == 1,
            "created_at": r.get::<_, String>(7)?,
            "last_login_at": r.get::<_, Option<String>>(8)?,
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    ctx.require("user.create")?;
    ctx.require_step_up("user.create")?;

    let username = field(&body, "username")?.to_lowercase();
    let display_name = field(&body, "display_name")?;
    let role_key = field(&body, "role")?;
    let pw = field(&body, "password")?;
    validate_username(&username)?;
    password::validate_strength(pw)?;

    let role_id: String = {
        let guard = state.db.lock();
        guard
            .query_row(
                "SELECT id FROM roles WHERE key=?1",
                rusqlite::params![role_key],
                |r| r.get(0),
            )
            .map_err(|_| Error::Validation(format!("Rolle '{role_key}' existiert nicht.")))?
    };

    let secret = totp::generate_secret();
    let secret_id = state.with_kms(|k| {
        k.store(
            &state.db,
            SecretClass::TotpSecret,
            &format!("totp:{username}"),
            "user",
            secret.as_bytes(),
        )
    })?;
    let user_id = ids::new_prefixed("usr");
    {
        let guard = state.db.lock();
        guard
            .execute(
                "INSERT INTO users
                 (id,username,display_name,email,role_id,password_hash,password_algo,totp_secret_id,totp_enabled,status,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0,'pending_totp',?9,?9)",
                rusqlite::params![
                    user_id, username, display_name,
                    body.get("email").and_then(|v| v.as_str()),
                    role_id, password::hash(pw)?, password::PROFILE, secret_id, now_rfc3339()
                ],
            )
            .map_err(|_| Error::Conflict("Dieser Benutzername ist bereits vergeben.".into()))?;
    }

    state.audit.record(
        &state.db,
        Event::user(&ctx.user_id, "user.create")
            .object("user", &user_id)
            .detail(json!({ "username": username, "role": role_key })),
    )?;

    Ok(Json(json!({
        "id": user_id,
        "totp_secret": secret,
        "provisioning_uri": totp::provisioning_uri(&secret, &username, "EXTELIO"),
    })))
}

pub async fn list_roles(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let ctx = require_auth(&state, &headers)?;
    let _ = ctx;
    let guard = state.db.lock();
    let mut stmt =
        guard.prepare("SELECT id,key,name,permissions,builtin FROM roles ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "key": r.get::<_, String>(1)?,
            "name": r.get::<_, String>(2)?,
            "permissions": serde_json::from_str::<Value>(&r.get::<_, String>(3)?).unwrap_or(Value::Null),
            "builtin": r.get::<_, i64>(4)? == 1,
            "step_up_actions": rbac::requires_step_up("user.create"),
        }))
    })?;
    let items: Vec<Value> = rows.collect::<std::result::Result<_, _>>()?;
    Ok(Json(json!({ "items": items })))
}
