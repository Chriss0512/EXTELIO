//! SQLite-Zugriff (Kapitel 4.2: SQLite + WAL).

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::Result;

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::tune(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::tune(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn tune(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        Ok(())
    }

    /// Sperrt die Verbindung.
    ///
    /// Regel: Der Guard wird so kurz wie moeglich gehalten und niemals ueber
    /// einen weiteren Datenbankzugriff hinweg. Der Mutex ist nicht
    /// wiedereintrittsfaehig - wird bei gehaltener Sperre erneut `lock()`,
    /// `commit_now()`, `Audit::record()` oder eine KMS-Operation aufgerufen,
    /// blockiert der Aufruf dauerhaft. In Handlern deshalb immer erst die
    /// Abfrage in einem eigenen Block abschliessen und den Guard fallen
    /// lassen, bevor der naechste Zugriff erfolgt.
    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        match self.conn.lock() {
            Ok(g) => g,
            // Ein vergifteter Mutex bedeutet Panic in einem anderen Handler.
            // Der Zustand der Verbindung bleibt nutzbar, deshalb Wiederaufnahme.
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Fuehrt eine Aktion in einer eigenen, sofort committeten Transaktion aus.
    ///
    /// Wichtig fuer Seiteneffekte, die auch dann bestehen bleiben muessen,
    /// wenn der umgebende Request mit einem Fehler endet - etwa das Hochzaehlen
    /// von Fehlversuchen (Kapitel 6.8). Wuerde der Zaehler in derselben
    /// Transaktion wie die Fehlerantwort liegen, wuerde er zurueckgerollt und
    /// ein Lockout niemals ausgeloest.
    pub fn commit_now<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
    {
        let mut guard = self.lock();
        let tx = guard.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    pub fn checkpoint(&self) -> Result<()> {
        let guard = self.lock();
        guard.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        Ok(())
    }

    pub fn integrity_ok(&self) -> bool {
        let guard = self.lock();
        guard
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .map(|s| s == "ok")
            .unwrap_or(false)
    }
}
