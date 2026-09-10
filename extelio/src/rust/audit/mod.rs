//! Security Audit (Kapitel 14.4).
//!
//! Hash-chained append-only Ledger mit periodischen HMAC-Checkpoints.
//! Secretwerte werden nie protokolliert (Kapitel 14.5).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::ids;
use crate::infrastructure::db::Db;
use crate::infrastructure::time::now_rfc3339;
use crate::Result;

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub action: String,
    pub object_type: Option<String>,
    pub object_id: Option<String>,
    pub outcome: String,
    pub reason: Option<String>,
    pub detail: serde_json::Value,
    pub remote_ip: Option<String>,
}

impl Event {
    pub fn system(action: &str) -> Self {
        Self {
            actor_type: "system".into(),
            actor_id: None,
            action: action.into(),
            object_type: None,
            object_id: None,
            outcome: "success".into(),
            reason: None,
            detail: serde_json::json!({}),
            remote_ip: None,
        }
    }

    pub fn user(actor_id: &str, action: &str) -> Self {
        Self {
            actor_type: "user".into(),
            actor_id: Some(actor_id.into()),
            ..Self::system(action)
        }
    }

    pub fn object(mut self, t: &str, id: &str) -> Self {
        self.object_type = Some(t.into());
        self.object_id = Some(id.into());
        self
    }

    pub fn outcome(mut self, o: &str) -> Self {
        self.outcome = o.into();
        self
    }

    pub fn reason(mut self, r: &str) -> Self {
        self.reason = Some(r.into());
        self
    }

    pub fn detail(mut self, d: serde_json::Value) -> Self {
        self.detail = d;
        self
    }

    pub fn from_ip(mut self, ip: Option<String>) -> Self {
        self.remote_ip = ip.map(|s| crate::infrastructure::log::mask_ip(&s));
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: i64,
    pub id: String,
    pub ts: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub action: String,
    pub object_type: Option<String>,
    pub object_id: Option<String>,
    pub outcome: String,
    pub reason: Option<String>,
    pub detail: serde_json::Value,
    pub entry_hash: String,
}

#[derive(Clone, Default)]
pub struct Audit;

impl Audit {
    pub fn new() -> Self {
        Audit
    }

    fn hash_entry(prev: &str, id: &str, ts: &str, ev: &Event) -> String {
        let canonical = serde_json::json!({
            "id": id,
            "ts": ts,
            "actor_type": ev.actor_type,
            "actor_id": ev.actor_id,
            "action": ev.action,
            "object_type": ev.object_type,
            "object_id": ev.object_id,
            "outcome": ev.outcome,
            "reason": ev.reason,
            "detail": ev.detail,
        });
        let mut h = Sha256::new();
        h.update(prev.as_bytes());
        h.update(canonical.to_string().as_bytes());
        format!("{:x}", h.finalize())
    }

    /// Haengt ein Ereignis an das Ledger an. Der Schreibvorgang committet
    /// sofort, damit Sicherheitsereignisse auch dann erhalten bleiben, wenn
    /// der ausloesende Request mit einem Fehler endet.
    pub fn record(&self, db: &Db, ev: Event) -> Result<String> {
        let id = ids::new_prefixed("aud");
        let ts = now_rfc3339();
        db.commit_now(|tx| {
            let prev: String = tx
                .query_row(
                    "SELECT entry_hash FROM audit_log ORDER BY seq DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| GENESIS.to_string());
            let entry_hash = Self::hash_entry(&prev, &id, &ts, &ev);
            tx.execute(
                "INSERT INTO audit_log
                 (id,ts,actor_type,actor_id,action,object_type,object_id,outcome,reason,detail,remote_ip,prev_hash,entry_hash)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                rusqlite::params![
                    id, ts, ev.actor_type, ev.actor_id, ev.action, ev.object_type, ev.object_id,
                    ev.outcome, ev.reason, ev.detail.to_string(), ev.remote_ip, prev, entry_hash
                ],
            )?;
            Ok(entry_hash)
        })
    }

    /// Prueft die Hash-Kette vollstaendig.
    pub fn verify(&self, db: &Db) -> Result<AuditVerification> {
        let guard = db.lock();
        let mut stmt = guard.prepare(
            "SELECT seq,id,ts,actor_type,actor_id,action,object_type,object_id,outcome,reason,detail,prev_hash,entry_hash
             FROM audit_log ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                Event {
                    actor_type: r.get(3)?,
                    actor_id: r.get(4)?,
                    action: r.get(5)?,
                    object_type: r.get(6)?,
                    object_id: r.get(7)?,
                    outcome: r.get(8)?,
                    reason: r.get(9)?,
                    detail: serde_json::from_str(&r.get::<_, String>(10)?)
                        .unwrap_or(serde_json::Value::Null),
                    remote_ip: None,
                },
                r.get::<_, String>(11)?,
                r.get::<_, String>(12)?,
            ))
        })?;

        let mut prev = GENESIS.to_string();
        let mut count = 0i64;
        for row in rows {
            let (seq, id, ts, ev, stored_prev, stored_hash) = row?;
            count += 1;
            if stored_prev != prev {
                return Ok(AuditVerification {
                    valid: false,
                    entries: count,
                    broken_at: Some(seq),
                });
            }
            let expect = Self::hash_entry(&prev, &id, &ts, &ev);
            if expect != stored_hash {
                return Ok(AuditVerification {
                    valid: false,
                    entries: count,
                    broken_at: Some(seq),
                });
            }
            prev = stored_hash;
        }
        Ok(AuditVerification {
            valid: true,
            entries: count,
            broken_at: None,
        })
    }

    /// Periodischer Checkpoint mit HMAC ueber den letzten Kettenhash.
    pub fn checkpoint(&self, db: &Db, mac_key: &[u8]) -> Result<Option<String>> {
        use hmac::{Hmac, Mac};
        let guard = db.lock();
        let head: Option<(i64, String)> = guard
            .query_row(
                "SELECT seq,entry_hash FROM audit_log ORDER BY seq DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        let Some((seq, hash)) = head else {
            return Ok(None);
        };

        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(mac_key)
            .map_err(|_| crate::Error::Internal("HMAC-Schluessel ungueltig".into()))?;
        mac.update(hash.as_bytes());
        let tag = format!("{:x}", mac.finalize().into_bytes());
        guard.execute(
            "INSERT INTO audit_checkpoints (id,ts,upto_seq,mac) VALUES (?1,?2,?3,?4)",
            rusqlite::params![ids::new_prefixed("ckp"), now_rfc3339(), seq, tag],
        )?;
        Ok(Some(tag))
    }

    pub fn list(
        &self,
        db: &Db,
        limit: i64,
        action_filter: Option<&str>,
    ) -> Result<Vec<AuditEntry>> {
        let guard = db.lock();
        let sql = "SELECT seq,id,ts,actor_type,actor_id,action,object_type,object_id,outcome,reason,detail,entry_hash
                   FROM audit_log
                   WHERE (?2 IS NULL OR action LIKE ?2)
                   ORDER BY seq DESC LIMIT ?1";
        let mut stmt = guard.prepare(sql)?;
        let pattern = action_filter.map(|a| format!("{a}%"));
        let rows = stmt.query_map(rusqlite::params![limit, pattern], |r| {
            Ok(AuditEntry {
                seq: r.get(0)?,
                id: r.get(1)?,
                ts: r.get(2)?,
                actor_type: r.get(3)?,
                actor_id: r.get(4)?,
                action: r.get(5)?,
                object_type: r.get(6)?,
                object_id: r.get(7)?,
                outcome: r.get(8)?,
                reason: r.get(9)?,
                detail: serde_json::from_str(&r.get::<_, String>(10)?)
                    .unwrap_or(serde_json::Value::Null),
                entry_hash: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditVerification {
    pub valid: bool,
    pub entries: i64,
    pub broken_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    #[test]
    fn chain_is_verifiable_and_tamper_evident() {
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let audit = Audit::new();

        audit.record(&db, Event::system("system.start")).unwrap();
        audit
            .record(&db, Event::user("u1", "auth.login").outcome("success"))
            .unwrap();
        audit
            .record(
                &db,
                Event::user("u1", "extension.create").object("extension", "e1"),
            )
            .unwrap();

        let v = audit.verify(&db).unwrap();
        assert!(v.valid);
        assert_eq!(v.entries, 3);

        {
            let guard = db.lock();
            guard
                .execute("UPDATE audit_log SET action='auth.logout' WHERE seq=2", [])
                .unwrap();
        }
        let v2 = audit.verify(&db).unwrap();
        assert!(!v2.valid, "Manipulation muss auffallen");
        assert_eq!(v2.broken_at, Some(2));
    }

    #[test]
    fn checkpoint_produces_mac() {
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let audit = Audit::new();
        assert!(audit.checkpoint(&db, b"key").unwrap().is_none());
        audit.record(&db, Event::system("system.start")).unwrap();
        assert!(audit.checkpoint(&db, b"key").unwrap().is_some());
    }
}
