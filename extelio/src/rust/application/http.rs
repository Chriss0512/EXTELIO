//! HTTP-Schicht: Router, Sicherheitsheader, statische Auslieferung.
//!
//! Content Security Policy (Kapitel 12): keine Inline-Styles, keine
//! Inline-Skripte, keine externen Quellen. Das Frontend wird ausschliesslich
//! aus lokalen Dateien geladen.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use std::path::{Component, Path, PathBuf};

use super::state::{AppState, AuthContext};
use crate::auth::session::{self, Transport};
use crate::{Error, Result};

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Error::Validation(_) => (StatusCode::BAD_REQUEST, "validation"),
            Error::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Error::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            Error::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Error::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Error::Throttled(_) => (StatusCode::TOO_MANY_REQUESTS, "throttled"),
            Error::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        // Interne Details bleiben im Log, der Client bekommt eine neutrale Meldung.
        let message = match &self {
            Error::Internal(detail) => {
                crate::log_error!("http", "{detail}");
                "Es ist ein interner Fehler aufgetreten.".to_string()
            }
            other => other
                .to_string()
                .split_once(": ")
                .map(|(_, rest)| rest.to_string())
                .unwrap_or_default(),
        };
        (
            status,
            Json(serde_json::json!({ "error": code, "message": message })),
        )
            .into_response()
    }
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        // Oeffentlich
        .route("/health", get(super::api_system::public_health))
        .route("/setup/status", get(super::api_auth::setup_status))
        .route("/setup/admin", post(super::api_auth::setup_admin))
        .route(
            "/setup/totp/confirm",
            post(super::api_auth::setup_totp_confirm),
        )
        .route("/auth/login", post(super::api_auth::login))
        // Angemeldet
        .route("/auth/session", get(super::api_auth::session_info))
        .route("/auth/logout", post(super::api_auth::logout))
        .route("/auth/step-up", post(super::api_auth::step_up))
        .route("/dashboard", get(super::api_system::dashboard))
        .route("/system/health", get(super::api_system::system_health))
        .route("/system/settings", get(super::api_system::get_settings))
        .route("/system/settings", put(super::api_system::put_settings))
        .route("/audit", get(super::api_system::audit_list))
        .route("/audit/verify", get(super::api_system::audit_verify))
        .route("/config/generations", get(super::api_system::generations))
        .route("/config/compile", post(super::api_system::compile))
        .route("/config/activate", post(super::api_system::activate))
        .route("/config/rollback", post(super::api_system::rollback))
        .route("/backup", get(super::api_system::backup_list))
        .route("/backup", post(super::api_system::backup_create))
        .route("/sites", get(super::api_telephony::list_sites))
        .route("/sites", post(super::api_telephony::create_site))
        .route("/extensions", get(super::api_telephony::list_extensions))
        .route("/extensions", post(super::api_telephony::create_extension))
        .route(
            "/extensions/{id}",
            put(super::api_telephony::update_extension),
        )
        .route(
            "/extensions/{id}",
            delete(super::api_telephony::delete_extension),
        )
        .route("/devices", get(super::api_telephony::list_devices))
        .route("/devices", post(super::api_telephony::create_device))
        .route("/devices/{id}", put(super::api_telephony::update_device))
        .route("/devices/{id}", delete(super::api_telephony::delete_device))
        .route("/numbers", get(super::api_telephony::list_numbers))
        .route("/numbers", post(super::api_telephony::create_number))
        .route("/numbers/{id}", put(super::api_telephony::update_number))
        .route("/numbers/{id}", delete(super::api_telephony::delete_number))
        .route("/providers", get(super::api_telephony::list_providers))
        .route("/trunks", get(super::api_telephony::list_trunks))
        .route("/trunks", post(super::api_telephony::create_trunk))
        .route("/trunks/{id}", put(super::api_telephony::update_trunk))
        .route("/trunks/{id}", delete(super::api_telephony::delete_trunk))
        .route("/groups", get(super::api_telephony::list_groups))
        .route("/groups", post(super::api_telephony::create_group))
        .route("/flows", get(super::api_telephony::list_flows))
        .route("/flows", post(super::api_telephony::create_flow))
        .route("/flows/{id}", put(super::api_telephony::update_flow))
        .route(
            "/flows/{id}/validate",
            post(super::api_telephony::validate_flow),
        )
        .route(
            "/flows/{id}/publish",
            post(super::api_telephony::publish_flow),
        )
        .route("/users", get(super::api_auth::list_users))
        .route("/users", post(super::api_auth::create_user))
        .route("/roles", get(super::api_auth::list_roles));

    Router::new()
        .nest("/api", api)
        .fallback(serve_static)
        .layer(axum::middleware::from_fn(security_headers))
        .with_state(state)
}

/// Sicherheitsheader fuer jede Antwort (Kapitel 12).
async fn security_headers(req: Request, next: Next) -> Response {
    let is_https = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false);

    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
             font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; \
             frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    h.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    h.insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    if is_https {
        h.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    res
}

/// Bestimmt den effektiven Transport fuer die Session-Trennung (Kapitel 6.4).
pub fn transport_of(state: &AppState, headers: &HeaderMap) -> Transport {
    if state.options.web_enabled_https() {
        return Transport::Https;
    }
    if state.options.behind_trusted_proxy() {
        let proto = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if proto.eq_ignore_ascii_case("https") {
            return Transport::Https;
        }
    }
    Transport::Http
}

pub fn client_ip(state: &AppState, headers: &HeaderMap) -> Option<String> {
    if state.options.behind_trusted_proxy() {
        if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = xff.split(',').next() {
                return Some(first.trim().to_string());
            }
        }
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Loest die aktuelle Sitzung auf und laedt die effektiven Berechtigungen.
pub fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<AuthContext> {
    let transport = transport_of(state, headers);
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let token = session::read_cookie(cookie, transport.cookie_name())
        .ok_or_else(|| Error::Unauthorized("Nicht angemeldet.".into()))?;
    let s = session::resolve(&state.db, &token, transport)?;

    let guard = state.db.lock();
    let (username, display_name, role_key, permissions_raw, status): (
        String,
        String,
        String,
        String,
        String,
    ) = guard
        .query_row(
            "SELECT u.username, u.display_name, r.key, r.permissions, u.status
             FROM users u JOIN roles r ON r.id = u.role_id WHERE u.id = ?1",
            rusqlite::params![s.user_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|_| Error::Unauthorized("Benutzerkonto existiert nicht mehr.".into()))?;
    drop(guard);

    if status != "active" {
        return Err(Error::Forbidden(
            "Das Benutzerkonto ist deaktiviert.".into(),
        ));
    }

    Ok(AuthContext {
        user_id: s.user_id.clone(),
        username,
        display_name,
        role_key,
        permissions: serde_json::from_str(&permissions_raw).unwrap_or_default(),
        session: s,
        transport,
    })
}

/// Statische Auslieferung des Frontends mit SPA-Fallback.
async fn serve_static(State(state): State<AppState>, req: Request) -> Response {
    let path = req.uri().path();
    if path.starts_with("/api/") {
        return Error::NotFound("Unbekannter Endpunkt".into()).into_response();
    }

    let root = state.paths.web_root.clone();
    let candidate = match safe_join(&root, path.trim_start_matches('/')) {
        Some(p) if p.is_file() => p,
        _ => root.join("index.html"),
    };

    match tokio::fs::read(&candidate).await {
        Ok(bytes) => {
            let ct = content_type(&candidate);
            let cache = if candidate.ends_with("index.html") {
                "no-store"
            } else {
                "public, max-age=31536000, immutable"
            };
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, ct)
                .header(header::CACHE_CONTROL, cache)
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Die Weboberfläche ist nicht installiert.",
        )
            .into_response(),
    }
}

/// Verhindert Pfad-Traversal.
fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    let mut out = root.to_path_buf();
    for c in PathBuf::from(rel).components() {
        match c {
            Component::Normal(part) => out.push(part),
            _ => return None,
        }
    }
    Some(out)
}

fn content_type(p: &std::path::Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        let root = PathBuf::from("/opt/extelio/web");
        assert!(safe_join(&root, "../../etc/passwd").is_none());
        assert!(safe_join(&root, "assets/app.js").is_some());
    }

    #[test]
    fn content_types() {
        assert_eq!(
            content_type(std::path::Path::new("a.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type(std::path::Path::new("a.bin")),
            "application/octet-stream"
        );
    }
}
