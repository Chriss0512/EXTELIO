//! Home-Assistant-Integration (Kapitel 13).
//!
//! Privacy Boundary (Kapitel 13.1): An Home Assistant gehen ausschliesslich
//! nicht-personenbezogene Betriebs- und Gesundheitszustaende. Rufnummern,
//! Namen, personenbezogene Extensions, Call IDs, CDR sowie Voicemail- und
//! Recordinginhalte werden nie uebertragen.
//!
//! Der Zugriff laeuft ueber die Supervisor-Proxy-URL `http://supervisor/core/api`
//! mit dem `SUPERVISOR_TOKEN` der Home-Assistant-App.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::health::HealthReport;
use crate::{Error, Result};

/// Genau die Kennzahlen, die Home Assistant sehen darf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicState {
    pub pbx_online: bool,
    pub health_state: String,
    pub active_calls: i64,
    pub trunks_total: i64,
    pub trunks_registered: i64,
    pub extensions_total: i64,
    pub devices_online: i64,
    pub active_generation: Option<i64>,
}

/// Prueft, dass ein Payload keine personenbezogenen Felder enthaelt.
/// Wird vor jedem Versand aufgerufen und ist die technische Umsetzung der
/// Privacy Boundary.
pub fn assert_non_personal(v: &serde_json::Value) -> Result<()> {
    const FORBIDDEN: &[&str] = &[
        "number",
        "caller",
        "callee",
        "name",
        "display_name",
        "username",
        "email",
        "extension",
        "call_id",
        "cdr",
        "recording",
        "voicemail",
        "ip",
        "mac",
        "contact",
    ];
    fn walk(v: &serde_json::Value, forbidden: &[&str]) -> Option<String> {
        match v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    let lk = k.to_lowercase();
                    if forbidden
                        .iter()
                        .any(|f| lk == *f || lk.ends_with(&format!("_{f}")))
                    {
                        return Some(k.clone());
                    }
                    if let Some(hit) = walk(val, forbidden) {
                        return Some(hit);
                    }
                }
                None
            }
            serde_json::Value::Array(items) => items.iter().find_map(|i| walk(i, forbidden)),
            _ => None,
        }
    }
    match walk(v, FORBIDDEN) {
        Some(field) => Err(Error::Forbidden(format!(
            "Feld '{field}' ist personenbezogen und darf nicht an Home Assistant gehen"
        ))),
        None => Ok(()),
    }
}

pub struct HaClient {
    base_host: String,
    base_port: u16,
    token: String,
}

impl HaClient {
    /// Baut den Client aus der Umgebung der Home-Assistant-App.
    /// Ohne Supervisor-Token ist die Integration inaktiv.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("SUPERVISOR_TOKEN").ok()?;
        Some(Self {
            base_host: std::env::var("EXTELIO_HA_HOST").unwrap_or_else(|_| "supervisor".into()),
            base_port: std::env::var("EXTELIO_HA_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(80),
            token,
        })
    }

    /// Meldet die erlaubten Zustaende als Home-Assistant-Sensoren.
    pub async fn publish(&self, state: &PublicState) -> Result<()> {
        let sensors: Vec<(&str, serde_json::Value, &str, &str)> = vec![
            (
                "sensor.extelio_status",
                serde_json::json!(state.health_state),
                "EXTELIO Status",
                "mdi:phone-settings",
            ),
            (
                "sensor.extelio_active_calls",
                serde_json::json!(state.active_calls),
                "EXTELIO Aktive Gespraeche",
                "mdi:phone-in-talk",
            ),
            (
                "sensor.extelio_trunks_registered",
                serde_json::json!(state.trunks_registered),
                "EXTELIO Registrierte Trunks",
                "mdi:transit-connection-variant",
            ),
            (
                "sensor.extelio_devices_online",
                serde_json::json!(state.devices_online),
                "EXTELIO Geraete online",
                "mdi:deskphone",
            ),
            (
                "binary_sensor.extelio_online",
                serde_json::json!(if state.pbx_online { "on" } else { "off" }),
                "EXTELIO erreichbar",
                "mdi:server-network",
            ),
        ];

        for (entity, value, friendly, icon) in sensors {
            let body = serde_json::json!({
                "state": value,
                "attributes": {
                    "friendly_name": friendly,
                    "icon": icon,
                    "source": "EXTELIO",
                    "generation": state.active_generation,
                }
            });
            assert_non_personal(&body)?;
            self.post(&format!("/core/api/states/{entity}"), &body)
                .await?;
        }
        Ok(())
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<String> {
        let payload = serde_json::to_vec(body)?;
        let mut stream = TcpStream::connect((self.base_host.as_str(), self.base_port))
            .await
            .map_err(|e| Error::Internal(format!("Home Assistant nicht erreichbar: {e}")))?;

        let head = format!(
            "POST {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.base_host,
            self.token,
            payload.len()
        );
        stream.write_all(head.as_bytes()).await?;
        stream.write_all(&payload).await?;
        stream.flush().await?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        let text = String::from_utf8_lossy(&buf).to_string();
        let status = text
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|c| c.parse::<u16>().ok())
            .unwrap_or(0);
        if !(200..300).contains(&status) {
            // Der Token wird bewusst nicht mitgeloggt (Kapitel 14.5).
            return Err(Error::Internal(format!(
                "Home Assistant antwortete mit HTTP {status}"
            )));
        }
        Ok(text)
    }
}

/// Leitet den erlaubten Zustand aus dem Health Report und der Datenbank ab.
pub fn public_state(db: &crate::infrastructure::db::Db, report: &HealthReport) -> PublicState {
    let guard = db.lock();
    let count = |sql: &str| -> i64 { guard.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    PublicState {
        pbx_online: report.state != crate::health::HealthState::Unhealthy,
        health_state: report.state.as_str().to_string(),
        active_calls: count(
            "SELECT COUNT(DISTINCT call_id) FROM call_events WHERE event_type='CHANNEL_ANSWER'
             AND call_id NOT IN (SELECT call_id FROM call_events WHERE event_type='CHANNEL_HANGUP')",
        ),
        trunks_total: count("SELECT COUNT(*) FROM trunks WHERE enabled=1"),
        trunks_registered: count("SELECT COUNT(*) FROM trunks WHERE status='registered'"),
        extensions_total: count("SELECT COUNT(*) FROM extensions"),
        devices_online: count("SELECT COUNT(*) FROM devices WHERE status='online'"),
        active_generation: report.active_generation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_personal_payloads() {
        let ok = serde_json::json!({"state":"HEALTHY","attributes":{"active_calls":3}});
        assert_non_personal(&ok).unwrap();

        for bad in [
            serde_json::json!({"state":"ok","attributes":{"caller":"+4951112345"}}),
            serde_json::json!({"state":"ok","attributes":{"nested":{"display_name":"Anna"}}}),
            serde_json::json!({"state":"ok","attributes":{"call_id":"abc"}}),
            serde_json::json!({"items":[{"extension":"201"}]}),
        ] {
            assert!(
                assert_non_personal(&bad).is_err(),
                "Kapitel 13.1 verletzt: {bad}"
            );
        }
    }
}
