//! EXTELIO - Software-defined Communications
//!
//! Leitprinzip: "Simple outside, rigorous inside."
//!
//! Modulschnitt folgt Kapitel 22.2 der Implementierungsspezifikation.
//! Domaenengrenzen folgen Kapitel 8: Identity | Organization | Telephony |
//! Routing | Scheduling | Media | Audit.

pub mod adapters;
pub mod application;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod compiler;
pub mod domain;
pub mod health;
pub mod infrastructure;
pub mod kms;

/// Produktversion. Wird von `config.yaml` und `build-lock.json` gespiegelt.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Einheitlicher Fehlertyp der Anwendung.
#[derive(Debug)]
pub enum Error {
    /// Eingabe verletzt eine Domaenen- oder Schemaregel.
    Validation(String),
    /// Authentifizierung fehlgeschlagen oder fehlend.
    Unauthorized(String),
    /// Authentifiziert, aber nicht berechtigt (RBAC).
    Forbidden(String),
    /// Objekt existiert nicht.
    NotFound(String),
    /// Konflikt mit bestehendem Zustand.
    Conflict(String),
    /// Rate Limit / Lockout aktiv.
    Throttled(String),
    /// Interner Fehler; Details gehen ins Log, nicht an den Client.
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Validation(m) => write!(f, "validation: {m}"),
            Error::Unauthorized(m) => write!(f, "unauthorized: {m}"),
            Error::Forbidden(m) => write!(f, "forbidden: {m}"),
            Error::NotFound(m) => write!(f, "not_found: {m}"),
            Error::Conflict(m) => write!(f, "conflict: {m}"),
            Error::Throttled(m) => write!(f, "throttled: {m}"),
            Error::Internal(m) => write!(f, "internal: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Internal(format!("sqlite: {e}"))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Internal(format!("io: {e}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Validation(format!("json: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
