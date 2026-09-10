//! Secret Broker (Kapitel 7.4).
//!
//! Secrets werden nicht global ueber Environment Variables verteilt. Consumer
//! fragen ueber einen lokalen Unix-Socket an, der Broker prueft die Policy der
//! Secret Class gegen den anfragenden Principal und auditiert jeden Zugriff.
//!
//! Protokoll (zeilenbasiert, UTF-8):
//!   Anfrage:  {"principal":"pbx-core","op":"get","id":"sec_..."}
//!   Antwort:  {"ok":true,"value":"..."} | {"ok":false,"error":"..."}

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::audit::Audit;
use crate::infrastructure::db::Db;
use crate::kms::{Kms, SecretClass};
use crate::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerRequest {
    pub principal: String,
    pub op: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Prueft die Policy einer Secret Class gegen einen Prozess-Principal.
pub fn policy_allows(class: SecretClass, principal: &str) -> bool {
    class.allowed_principals().contains(&principal)
}

/// Beantwortet eine Broker-Anfrage. Getrennt vom Transport, damit die Policy
/// unabhaengig testbar bleibt.
pub fn handle(db: &Db, kms: &Kms, audit: &Audit, req: &BrokerRequest) -> BrokerResponse {
    match handle_inner(db, kms, audit, req) {
        Ok(v) => BrokerResponse {
            ok: true,
            value: Some(v),
            error: None,
        },
        Err(e) => {
            let _ = audit.record(
                db,
                crate::audit::Event::system("secret.failed_access")
                    .object("secret", &req.id)
                    .outcome("denied")
                    .reason(&e.to_string())
                    .detail(serde_json::json!({ "principal": req.principal })),
            );
            BrokerResponse {
                ok: false,
                value: None,
                error: Some(e.to_string()),
            }
        }
    }
}

fn handle_inner(db: &Db, kms: &Kms, audit: &Audit, req: &BrokerRequest) -> Result<String> {
    if req.op != "get" {
        return Err(Error::Validation(format!(
            "Unbekannte Operation '{}'",
            req.op
        )));
    }
    let class_raw: String = {
        let guard = db.lock();
        guard
            .query_row(
                "SELECT class FROM secrets WHERE id=?1 AND revoked_at IS NULL",
                rusqlite::params![req.id],
                |r| r.get(0),
            )
            .map_err(|_| Error::NotFound("Secret nicht gefunden".into()))?
    };
    let class = SecretClass::parse(&class_raw)
        .ok_or_else(|| Error::Internal("Unbekannte Secret Class".into()))?;

    if !policy_allows(class, &req.principal) {
        return Err(Error::Forbidden(format!(
            "Principal '{}' darf {} nicht beziehen",
            req.principal,
            class.as_str()
        )));
    }

    let plain = kms.load(db, &req.id)?;
    let value = String::from_utf8(plain.to_vec())
        .map_err(|_| Error::Internal("Secret ist kein UTF-8".into()))?;

    // Kapitel 14.4: USE wird auditiert, der Wert selbst nie.
    audit.record(
        db,
        crate::audit::Event::system("secret.use")
            .object("secret", &req.id)
            .detail(serde_json::json!({ "principal": req.principal, "class": class.as_str() })),
    )?;
    Ok(value)
}

/// Client fuer Consumer-Prozesse.
pub struct BrokerClient {
    socket: std::path::PathBuf,
    principal: String,
}

impl BrokerClient {
    pub fn new(socket: &Path, principal: &str) -> Self {
        Self {
            socket: socket.to_path_buf(),
            principal: principal.to_string(),
        }
    }

    pub fn get(&self, id: &str) -> Result<String> {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(&self.socket)
            .map_err(|e| Error::Internal(format!("Secret Broker nicht erreichbar: {e}")))?;
        let req = BrokerRequest {
            principal: self.principal.clone(),
            op: "get".into(),
            id: id.to_string(),
        };
        writeln!(stream, "{}", serde_json::to_string(&req)?)?;
        stream.flush()?;

        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line)?;
        let res: BrokerResponse = serde_json::from_str(line.trim())?;
        if res.ok {
            res.value
                .ok_or_else(|| Error::Internal("Broker lieferte keinen Wert".into()))
        } else {
            Err(Error::Forbidden(
                res.error.unwrap_or_else(|| "abgelehnt".into()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    #[test]
    fn policy_matches_specification() {
        assert!(policy_allows(SecretClass::TotpSecret, "pbx-web"));
        assert!(!policy_allows(SecretClass::TotpSecret, "pbx-worker"));
        assert!(policy_allows(SecretClass::SipCredential, "pbx-xml-adapter"));
        assert!(!policy_allows(SecretClass::AcmeSecret, "freeswitch"));
    }

    #[test]
    fn broker_enforces_policy_and_audits() {
        let dir = std::env::temp_dir().join(format!("extelio-broker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let kms = Kms::open_or_init(&dir).unwrap();
        let audit = Audit::new();

        let id = kms
            .store(&db, SecretClass::TotpSecret, "totp", "user", b"JBSWY3DP")
            .unwrap();

        let ok = handle(
            &db,
            &kms,
            &audit,
            &BrokerRequest {
                principal: "pbx-web".into(),
                op: "get".into(),
                id: id.clone(),
            },
        );
        assert!(ok.ok);

        let denied = handle(
            &db,
            &kms,
            &audit,
            &BrokerRequest {
                principal: "freeswitch".into(),
                op: "get".into(),
                id: id.clone(),
            },
        );
        assert!(!denied.ok);

        let guard = db.lock();
        let n: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action LIKE 'secret.%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 2, "USE und FAILED_ACCESS werden auditiert");
    }
}
