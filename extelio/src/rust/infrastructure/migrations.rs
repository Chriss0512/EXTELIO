//! Migrationen. Die SQL-Dateien in `migrations/` sind die Quelle der Wahrheit
//! und werden zur Buildzeit eingebettet.

use crate::infrastructure::db::Db;
use crate::infrastructure::time::now_rfc3339;
use crate::Result;

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../../../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "seed",
        sql: include_str!("../../../migrations/0002_seed.sql"),
    },
];

pub fn applied_version(db: &Db) -> Result<i64> {
    let guard = db.lock();
    guard.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL)",
    )?;
    let v: i64 = guard.query_row(
        "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?;
    Ok(v)
}

/// Wendet alle ausstehenden Migrationen an. Jede Migration laeuft in einer
/// eigenen Transaktion; ein Fehler laesst die Datenbank auf dem letzten
/// vollstaendig angewendeten Stand.
pub fn run(db: &Db) -> Result<i64> {
    let current = applied_version(db)?;
    let mut last = current;
    for m in MIGRATIONS {
        if m.version <= current {
            continue;
        }
        let mut guard = db.lock();
        let tx = guard.transaction()?;
        tx.execute_batch(m.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version,name,applied_at) VALUES (?1,?2,?3)",
            rusqlite::params![m.version, m.name, now_rfc3339()],
        )?;
        tx.commit()?;
        last = m.version;
    }
    Ok(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let db = Db::open_memory().unwrap();
        let v1 = run(&db).unwrap();
        assert_eq!(v1, MIGRATIONS.last().unwrap().version);
        let v2 = run(&db).unwrap();
        assert_eq!(v1, v2);

        let guard = db.lock();
        let roles: i64 = guard
            .query_row("SELECT COUNT(*) FROM roles WHERE builtin=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(roles, 5, "Kapitel 6.5: fuenf Rollen-Templates");
    }
}
