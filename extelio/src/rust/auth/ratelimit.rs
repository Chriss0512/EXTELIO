//! Brute-Force-Schutz (Kapitel 6.8).
//!
//! Adaptive, exponentielle Limits nach Account, IP, Subnetz und Auth-Methode.
//!
//! Wichtig: Fehlversuche werden in einer eigenen, sofort committeten
//! Transaktion gezaehlt. Laege der Zaehler in derselben Transaktion wie die
//! Fehlerantwort, wuerde er beim Abbruch zurueckgerollt und ein Lockout
//! niemals ausgeloest.

use crate::domain::ids;
use crate::infrastructure::db::Db;
use crate::infrastructure::time::{is_past, now, now_rfc3339, plus_seconds, to_rfc3339};
use crate::{Error, Result};

/// Ab wie vielen Fehlversuchen gesperrt wird und wie lange.
const FREE_ATTEMPTS: i64 = 3;
const BASE_LOCK_SECONDS: i64 = 15;
const MAX_LOCK_SECONDS: i64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Account,
    Ip,
    Subnet,
    Method,
}

impl Scope {
    fn as_str(self) -> &'static str {
        match self {
            Scope::Account => "account",
            Scope::Ip => "ip",
            Scope::Subnet => "subnet",
            Scope::Method => "method",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockState {
    pub locked: bool,
    pub locked_until: Option<String>,
    pub failures: i64,
}

/// /24 bzw. /64 als Subnetz-Schluessel.
pub fn subnet_of(ip: &str) -> String {
    if let Some(idx) = ip.rfind('.') {
        return format!("{}.0/24", &ip[..idx]);
    }
    let parts: Vec<&str> = ip.split(':').collect();
    if parts.len() >= 4 {
        return format!("{}::/64", parts[..4].join(":"));
    }
    ip.to_string()
}

fn lock_seconds(failures: i64) -> i64 {
    if failures <= FREE_ATTEMPTS {
        return 0;
    }
    let exponent = (failures - FREE_ATTEMPTS - 1).min(10) as u32;
    (BASE_LOCK_SECONDS * 2i64.pow(exponent)).min(MAX_LOCK_SECONDS)
}

/// Prueft alle relevanten Scopes vor einem Loginversuch.
pub fn check(db: &Db, account: &str, ip: Option<&str>, method: &str) -> Result<()> {
    let mut scopes: Vec<(Scope, String)> = vec![(Scope::Account, account.to_lowercase())];
    if let Some(ip) = ip {
        scopes.push((Scope::Ip, ip.to_string()));
        scopes.push((Scope::Subnet, subnet_of(ip)));
    }

    let guard = db.lock();
    for (scope, value) in scopes {
        let locked_until: Option<String> = guard
            .query_row(
                "SELECT locked_until FROM auth_throttle
                 WHERE scope_type=?1 AND scope_value=?2 AND auth_method=?3",
                rusqlite::params![scope.as_str(), value, method],
                |r| r.get(0),
            )
            .unwrap_or(None);
        if let Some(until) = locked_until {
            if !is_past(&until) {
                return Err(Error::Throttled(format!(
                    "Zu viele Fehlversuche. Nächster Versuch ist ab {until} möglich."
                )));
            }
        }
    }
    Ok(())
}

/// Zaehlt einen Fehlversuch und committet sofort.
pub fn record_failure(db: &Db, account: &str, ip: Option<&str>, method: &str) -> Result<LockState> {
    let mut scopes: Vec<(Scope, String)> = vec![(Scope::Account, account.to_lowercase())];
    if let Some(ip) = ip {
        scopes.push((Scope::Ip, ip.to_string()));
        scopes.push((Scope::Subnet, subnet_of(ip)));
    }
    scopes.push((Scope::Method, method.to_string()));

    db.commit_now(|tx| {
        let mut account_state = LockState { locked: false, locked_until: None, failures: 0 };
        for (scope, value) in &scopes {
            tx.execute(
                "INSERT INTO auth_throttle (id,scope_type,scope_value,auth_method,failures,first_failure_at,last_failure_at)
                 VALUES (?1,?2,?3,?4,1,?5,?5)
                 ON CONFLICT(scope_type,scope_value,auth_method) DO UPDATE SET
                   failures = failures + 1,
                   last_failure_at = ?5",
                rusqlite::params![
                    ids::new_prefixed("thr"), scope.as_str(), value, method, now_rfc3339()
                ],
            )?;
            let failures: i64 = tx.query_row(
                "SELECT failures FROM auth_throttle
                 WHERE scope_type=?1 AND scope_value=?2 AND auth_method=?3",
                rusqlite::params![scope.as_str(), value, method],
                |r| r.get(0),
            )?;
            let secs = lock_seconds(failures);
            let until = if secs > 0 {
                let u = to_rfc3339(plus_seconds(now(), secs));
                tx.execute(
                    "UPDATE auth_throttle SET locked_until=?4
                     WHERE scope_type=?1 AND scope_value=?2 AND auth_method=?3",
                    rusqlite::params![scope.as_str(), value, method, u],
                )?;
                Some(u)
            } else {
                None
            };
            if *scope == Scope::Account {
                account_state = LockState { locked: secs > 0, locked_until: until, failures };
            }
        }
        Ok(account_state)
    })
}

/// Setzt die Zaehler nach erfolgreicher Authentifizierung zurueck.
pub fn record_success(db: &Db, account: &str, ip: Option<&str>, method: &str) -> Result<()> {
    let mut scopes: Vec<(Scope, String)> = vec![(Scope::Account, account.to_lowercase())];
    if let Some(ip) = ip {
        scopes.push((Scope::Ip, ip.to_string()));
        scopes.push((Scope::Subnet, subnet_of(ip)));
    }
    db.commit_now(|tx| {
        for (scope, value) in &scopes {
            tx.execute(
                "DELETE FROM auth_throttle
                 WHERE scope_type=?1 AND scope_value=?2 AND auth_method=?3",
                rusqlite::params![scope.as_str(), value, method],
            )?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    fn db() -> Db {
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        db
    }

    #[test]
    fn locks_after_repeated_failures_and_survives_error_paths() {
        let db = db();
        for _ in 0..FREE_ATTEMPTS {
            let s = record_failure(&db, "admin", Some("10.0.0.5"), "password").unwrap();
            assert!(!s.locked);
        }
        check(&db, "admin", Some("10.0.0.5"), "password").unwrap();

        let s = record_failure(&db, "admin", Some("10.0.0.5"), "password").unwrap();
        assert!(s.locked, "Der Zaehler darf nicht zurueckgerollt werden");
        assert!(check(&db, "admin", Some("10.0.0.5"), "password").is_err());
    }

    #[test]
    fn lock_duration_grows_exponentially() {
        assert_eq!(lock_seconds(3), 0);
        assert_eq!(lock_seconds(4), 15);
        assert_eq!(lock_seconds(5), 30);
        assert_eq!(lock_seconds(6), 60);
        assert_eq!(lock_seconds(99), MAX_LOCK_SECONDS);
    }

    #[test]
    fn success_clears_counters() {
        let db = db();
        for _ in 0..4 {
            let _ = record_failure(&db, "admin", Some("10.0.0.5"), "password").unwrap();
        }
        assert!(check(&db, "admin", Some("10.0.0.5"), "password").is_err());
        record_success(&db, "admin", Some("10.0.0.5"), "password").unwrap();
        check(&db, "admin", Some("10.0.0.5"), "password").unwrap();
    }

    #[test]
    fn subnet_keys() {
        assert_eq!(subnet_of("10.10.20.43"), "10.10.20.0/24");
        assert_eq!(subnet_of("2001:db8:1:2:3:4:5:6"), "2001:db8:1:2::/64");
    }
}
