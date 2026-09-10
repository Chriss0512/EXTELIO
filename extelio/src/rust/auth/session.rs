//! Sessions (Kapitel 6.4).
//!
//! Serverseitig in SQLite, opaques Cookie, kein Auth-Token im localStorage,
//! widerrufbar, Idle- und Absolut-Timeout, Session Rotation.
//! HTTP- und HTTPS-Sessions sind strikt getrennt.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::ids;
use crate::infrastructure::db::Db;
use crate::infrastructure::time::{
    is_past, now, now_rfc3339, plus_hours, plus_minutes, to_rfc3339,
};
use crate::{Error, Result};

pub const COOKIE_HTTPS: &str = "extelio_session_s";
pub const COOKIE_HTTP: &str = "extelio_session_h";

pub const DEFAULT_IDLE_MINUTES: i64 = 30;
pub const DEFAULT_ABSOLUTE_HOURS: i64 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Http,
    Https,
}

impl Transport {
    pub fn as_str(self) -> &'static str {
        match self {
            Transport::Http => "http",
            Transport::Https => "https",
        }
    }

    pub fn cookie_name(self) -> &'static str {
        match self {
            Transport::Http => COOKIE_HTTP,
            Transport::Https => COOKIE_HTTPS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub transport: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub idle_expires_at: String,
    pub absolute_expires_at: String,
    pub step_up_at: Option<String>,
}

fn random_token() -> String {
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

fn token_hash(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    format!("{:x}", h.finalize())
}

/// Erzeugt eine Session und liefert das Klartext-Token, das ausschliesslich
/// als Cookie zurueckgegeben wird.
pub fn create(
    db: &Db,
    user_id: &str,
    transport: Transport,
    remote_ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(String, Session)> {
    let token = random_token();
    let t = now();
    let session = Session {
        id: ids::new_prefixed("ses"),
        user_id: user_id.to_string(),
        transport: transport.as_str().into(),
        created_at: to_rfc3339(t),
        last_seen_at: to_rfc3339(t),
        idle_expires_at: to_rfc3339(plus_minutes(t, DEFAULT_IDLE_MINUTES)),
        absolute_expires_at: to_rfc3339(plus_hours(t, DEFAULT_ABSOLUTE_HOURS)),
        step_up_at: None,
    };

    let guard = db.lock();
    guard.execute(
        "INSERT INTO sessions
         (id,user_id,token_hash,transport,created_at,last_seen_at,idle_expires_at,absolute_expires_at,remote_ip,user_agent)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            session.id, session.user_id, token_hash(&token), session.transport,
            session.created_at, session.last_seen_at, session.idle_expires_at,
            session.absolute_expires_at,
            remote_ip.map(crate::infrastructure::log::mask_ip),
            user_agent.map(|u| u.chars().take(180).collect::<String>())
        ],
    )?;
    Ok((token, session))
}

/// Loest ein Cookie-Token auf und verlaengert das Idle-Fenster.
/// Ein Token, das zu einem anderen Transport gehoert, wird abgelehnt.
pub fn resolve(db: &Db, token: &str, transport: Transport) -> Result<Session> {
    let hash = token_hash(token);
    let s: Session = {
        let guard = db.lock();
        guard
            .query_row(
                "SELECT id,user_id,transport,created_at,last_seen_at,idle_expires_at,absolute_expires_at,step_up_at
                 FROM sessions WHERE token_hash=?1 AND revoked_at IS NULL",
                rusqlite::params![hash],
                |r| {
                    Ok(Session {
                        id: r.get(0)?,
                        user_id: r.get(1)?,
                        transport: r.get(2)?,
                        created_at: r.get(3)?,
                        last_seen_at: r.get(4)?,
                        idle_expires_at: r.get(5)?,
                        absolute_expires_at: r.get(6)?,
                        step_up_at: r.get(7)?,
                    })
                },
            )
            .map_err(|_| Error::Unauthorized("Sitzung ist nicht gültig.".into()))?
    };

    if s.transport != transport.as_str() {
        return Err(Error::Unauthorized(
            "Sitzung gehört zu einem anderen Transport.".into(),
        ));
    }
    if is_past(&s.idle_expires_at) {
        revoke(db, &s.id)?;
        return Err(Error::Unauthorized(
            "Sitzung ist wegen Inaktivität abgelaufen.".into(),
        ));
    }
    if is_past(&s.absolute_expires_at) {
        revoke(db, &s.id)?;
        return Err(Error::Unauthorized(
            "Sitzung hat ihre maximale Laufzeit erreicht.".into(),
        ));
    }

    let t = now();
    let guard = db.lock();
    guard.execute(
        "UPDATE sessions SET last_seen_at=?2, idle_expires_at=?3 WHERE id=?1",
        rusqlite::params![
            s.id,
            to_rfc3339(t),
            to_rfc3339(plus_minutes(t, DEFAULT_IDLE_MINUTES))
        ],
    )?;
    Ok(s)
}

/// Session Rotation: neues Token, gleiche Sitzung. Wird nach Login und nach
/// jeder Rechteaenderung aufgerufen.
pub fn rotate(db: &Db, session_id: &str) -> Result<String> {
    let token = random_token();
    let guard = db.lock();
    let n = guard.execute(
        "UPDATE sessions SET token_hash=?2, last_seen_at=?3 WHERE id=?1 AND revoked_at IS NULL",
        rusqlite::params![session_id, token_hash(&token), now_rfc3339()],
    )?;
    if n == 0 {
        return Err(Error::Unauthorized("Sitzung existiert nicht mehr.".into()));
    }
    Ok(token)
}

pub fn mark_step_up(db: &Db, session_id: &str) -> Result<()> {
    let guard = db.lock();
    guard.execute(
        "UPDATE sessions SET step_up_at=?2 WHERE id=?1",
        rusqlite::params![session_id, now_rfc3339()],
    )?;
    Ok(())
}

/// Ist eine Step-up-Bestaetigung noch frisch genug (5 Minuten)?
pub fn step_up_fresh(s: &Session) -> bool {
    match &s.step_up_at {
        None => false,
        Some(ts) => !is_past(&to_rfc3339(plus_minutes(
            crate::infrastructure::time::parse_rfc3339(ts).unwrap_or_else(now),
            5,
        ))),
    }
}

pub fn revoke(db: &Db, session_id: &str) -> Result<()> {
    let guard = db.lock();
    guard.execute(
        "UPDATE sessions SET revoked_at=?2 WHERE id=?1",
        rusqlite::params![session_id, now_rfc3339()],
    )?;
    Ok(())
}

pub fn revoke_all_for_user(db: &Db, user_id: &str) -> Result<usize> {
    let guard = db.lock();
    Ok(guard.execute(
        "UPDATE sessions SET revoked_at=?2 WHERE user_id=?1 AND revoked_at IS NULL",
        rusqlite::params![user_id, now_rfc3339()],
    )?)
}

/// Entfernt abgelaufene Sitzungen; wird vom Worker periodisch aufgerufen.
pub fn purge_expired(db: &Db) -> Result<usize> {
    let guard = db.lock();
    Ok(guard.execute(
        "DELETE FROM sessions WHERE absolute_expires_at < ?1 OR (revoked_at IS NOT NULL AND revoked_at < ?1)",
        rusqlite::params![now_rfc3339()],
    )?)
}

/// Baut den `Set-Cookie`-Header (Kapitel 6.4).
pub fn cookie_header(transport: Transport, token: &str, max_age_seconds: i64) -> String {
    let base = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        transport.cookie_name(),
        token,
        max_age_seconds
    );
    match transport {
        Transport::Https => format!("{base}; Secure"),
        Transport::Http => base,
    }
}

pub fn clear_cookie_header(transport: Transport) -> String {
    cookie_header(transport, "", 0)
}

/// Liest ein Cookie aus einem `Cookie`-Header.
pub fn read_cookie(header: &str, name: &str) -> Option<String> {
    header
        .split(';')
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    fn setup() -> (Db, String) {
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let guard = db.lock();
        guard
            .execute(
                "INSERT INTO users (id,username,display_name,role_id,password_hash,password_algo,created_at,updated_at)
                 VALUES ('u1','admin','Admin','role-sysadmin','x','argon2id',?1,?1)",
                rusqlite::params![now_rfc3339()],
            )
            .unwrap();
        drop(guard);
        (db, "u1".to_string())
    }

    #[test]
    fn creates_resolves_and_rotates() {
        let (db, uid) = setup();
        let (token, s) =
            create(&db, &uid, Transport::Https, Some("10.0.0.1"), Some("test")).unwrap();
        let resolved = resolve(&db, &token, Transport::Https).unwrap();
        assert_eq!(resolved.id, s.id);

        let new_token = rotate(&db, &s.id).unwrap();
        assert_ne!(new_token, token);
        assert!(
            resolve(&db, &token, Transport::Https).is_err(),
            "altes Token ist ungueltig"
        );
        assert!(resolve(&db, &new_token, Transport::Https).is_ok());
    }

    #[test]
    fn transports_are_separated() {
        let (db, uid) = setup();
        let (token, _) = create(&db, &uid, Transport::Https, None, None).unwrap();
        assert!(
            resolve(&db, &token, Transport::Http).is_err(),
            "Kapitel 6.4: HTTP uebernimmt keine HTTPS-Session"
        );
    }

    #[test]
    fn revocation_and_expiry() {
        let (db, uid) = setup();
        let (token, s) = create(&db, &uid, Transport::Https, None, None).unwrap();
        revoke(&db, &s.id).unwrap();
        assert!(resolve(&db, &token, Transport::Https).is_err());

        let (token2, s2) = create(&db, &uid, Transport::Https, None, None).unwrap();
        {
            let guard = db.lock();
            guard
                .execute(
                    "UPDATE sessions SET idle_expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
                    rusqlite::params![s2.id],
                )
                .unwrap();
        }
        assert!(resolve(&db, &token2, Transport::Https).is_err());
    }

    #[test]
    fn cookie_flags_match_specification() {
        let h = cookie_header(Transport::Https, "abc", 3600);
        assert!(
            h.contains("HttpOnly")
                && h.contains("SameSite=Strict")
                && h.contains("Path=/")
                && h.contains("Secure")
        );
        assert!(!cookie_header(Transport::Http, "abc", 3600).contains("Secure"));
        assert_eq!(
            read_cookie("a=1; extelio_session_s=xyz", COOKIE_HTTPS).as_deref(),
            Some("xyz")
        );
    }
}
