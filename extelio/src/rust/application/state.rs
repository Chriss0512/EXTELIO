//! Geteilter Anwendungszustand.

use std::sync::{Arc, Mutex};

use crate::audit::Audit;
use crate::auth::session::{Session, Transport};
use crate::infrastructure::config::{Options, Paths};
use crate::infrastructure::db::Db;
use crate::kms::Kms;
use crate::{Error, Result};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub kms: Arc<Mutex<Kms>>,
    pub audit: Audit,
    pub paths: Paths,
    pub options: Arc<Options>,
}

impl AppState {
    pub fn new(db: Db, kms: Kms, paths: Paths, options: Options) -> Self {
        Self {
            db,
            kms: Arc::new(Mutex::new(kms)),
            audit: Audit::new(),
            paths,
            options: Arc::new(options),
        }
    }

    pub fn with_kms<T>(&self, f: impl FnOnce(&Kms) -> Result<T>) -> Result<T> {
        let guard = self
            .kms
            .lock()
            .map_err(|_| Error::Internal("KMS-Sperre".into()))?;
        f(&guard)
    }

    pub fn with_kms_mut<T>(&self, f: impl FnOnce(&mut Kms) -> Result<T>) -> Result<T> {
        let mut guard = self
            .kms
            .lock()
            .map_err(|_| Error::Internal("KMS-Sperre".into()))?;
        f(&mut guard)
    }

    /// Ist bereits ein Benutzer angelegt? Steuert den First-Boot-Wizard.
    pub fn has_users(&self) -> bool {
        let guard = self.db.lock();
        guard
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))
            .map(|n| n > 0)
            .unwrap_or(false)
    }
}

/// Aufloesung des angemeldeten Nutzers inklusive effektiver Berechtigungen.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role_key: String,
    pub permissions: Vec<String>,
    pub session: Session,
    pub transport: Transport,
}

impl AuthContext {
    pub fn require(&self, permission: &str) -> Result<()> {
        crate::auth::rbac::require(&self.permissions, permission)
    }

    /// Kapitel 6.6: kritische Aktionen verlangen eine frische Bestaetigung.
    pub fn require_step_up(&self, action: &str) -> Result<()> {
        if crate::auth::rbac::requires_step_up(action)
            && !crate::auth::session::step_up_fresh(&self.session)
        {
            return Err(Error::Forbidden(
                "Diese Aktion erfordert eine erneute Bestätigung des Passworts.".into(),
            ));
        }
        Ok(())
    }
}
