//! FreeSWITCH Event Socket Adapter (Kapitel 9.3).
//!
//! Dauerhafte ESL-Verbindung, normalisierte Domain Events, kontrollierter
//! Reconnect (Kapitel 18.1). Die Verbindung geht ausschliesslich auf
//! 127.0.0.1 (Kapitel 5.6).

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::{Error, Result};

pub struct EslConnection {
    stream: BufReader<TcpStream>,
}

impl EslConnection {
    /// Verbindet sich und authentifiziert. Das Passwort kommt aus dem Secret
    /// Broker und wird nie geloggt.
    pub async fn connect(port: u16, password: &str) -> Result<Self> {
        let stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .map_err(|e| Error::Internal(format!("ESL nicht erreichbar: {e}")))?;
        let mut conn = Self {
            stream: BufReader::new(stream),
        };

        let hello = conn.read_block().await?;
        if !hello.contains("auth/request") {
            return Err(Error::Internal(
                "ESL meldete keine Authentifizierungsanfrage".into(),
            ));
        }
        conn.send(&format!("auth {password}\n\n")).await?;
        let reply = conn.read_block().await?;
        if !reply.contains("+OK") {
            return Err(Error::Unauthorized(
                "ESL-Authentifizierung abgelehnt".into(),
            ));
        }
        Ok(conn)
    }

    async fn send(&mut self, raw: &str) -> Result<()> {
        self.stream.get_mut().write_all(raw.as_bytes()).await?;
        self.stream.get_mut().flush().await?;
        Ok(())
    }

    /// Liest einen Block bis zur Leerzeile.
    async fn read_block(&mut self) -> Result<String> {
        let mut out = String::new();
        loop {
            let mut line = String::new();
            let n = self.stream.read_line(&mut line).await?;
            if n == 0 {
                if out.is_empty() {
                    return Err(Error::Internal("ESL-Verbindung wurde geschlossen".into()));
                }
                break;
            }
            if line.trim().is_empty() && !out.is_empty() {
                break;
            }
            out.push_str(&line);
        }
        Ok(out)
    }

    /// Fuehrt ein `api`-Kommando aus.
    pub async fn api(&mut self, command: &str) -> Result<String> {
        self.send(&format!("api {command}\n\n")).await?;
        self.read_block().await
    }

    /// Abonniert Kanalereignisse im Plain-Format.
    pub async fn subscribe_channel_events(&mut self) -> Result<()> {
        self.api("").await.ok();
        self.send("event plain CHANNEL_CREATE CHANNEL_ANSWER CHANNEL_HANGUP_COMPLETE\n\n")
            .await?;
        let reply = self.read_block().await?;
        if reply.contains("+OK") {
            Ok(())
        } else {
            Err(Error::Internal("Event-Abonnement abgelehnt".into()))
        }
    }

    pub async fn next_event(&mut self) -> Result<EslEvent> {
        let block = self.read_block().await?;
        Ok(EslEvent::parse(&block))
    }

    /// Sofia-Profilstatus, u.a. fuer die Health-Checks.
    pub async fn sofia_status(&mut self) -> Result<String> {
        self.api("sofia status").await
    }

    /// Sanfter Reload nach einer neuen Generation (Kapitel 9.2).
    pub async fn reload_xml(&mut self) -> Result<()> {
        self.api("reloadxml").await?;
        Ok(())
    }

    pub async fn rescan_profile(&mut self, profile: &str) -> Result<()> {
        self.api(&format!("sofia profile {profile} rescan")).await?;
        Ok(())
    }
}

/// Rohereignis aus FreeSWITCH. Kapitel 8.7: Diese Ereignisse sind nicht die
/// fachliche Wahrheit, sondern werden zu Domain Events normalisiert.
#[derive(Debug, Clone, Default)]
pub struct EslEvent {
    pub name: String,
    pub headers: Vec<(String, String)>,
}

impl EslEvent {
    pub fn parse(block: &str) -> Self {
        let mut headers = Vec::new();
        for line in block.lines() {
            if let Some((k, v)) = line.split_once(':') {
                headers.push((k.trim().to_string(), urldecode(v.trim())));
            }
        }
        let name = headers
            .iter()
            .find(|(k, _)| k == "Event-Name")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        Self { name, headers }
    }

    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Normalisiert auf ein Domain-Event fuer das Call Event Ledger.
    /// Rufnummern werden fuer Logs maskiert, im Ledger jedoch vollstaendig
    /// gespeichert, weil sie dort einer Retention Policy unterliegen.
    pub fn to_domain_event(&self) -> Option<(String, String, serde_json::Value)> {
        let call_id = self.header("Unique-ID")?.to_string();
        let kind = match self.name.as_str() {
            "CHANNEL_CREATE" => "CHANNEL_CREATE",
            "CHANNEL_ANSWER" => "CHANNEL_ANSWER",
            "CHANNEL_HANGUP_COMPLETE" => "CHANNEL_HANGUP",
            _ => return None,
        };
        let payload = serde_json::json!({
            "direction": self.header("Call-Direction"),
            "from": self.header("Caller-Caller-ID-Number"),
            "to": self.header("Caller-Destination-Number"),
            "cause": self.header("Hangup-Cause"),
            "profile": self.header("variable_sofia_profile_name"),
        });
        Some((call_id, kind.to_string(), payload))
    }
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalises_events() {
        let raw = "Event-Name: CHANNEL_ANSWER\n\
                   Unique-ID: 1234-abcd\n\
                   Call-Direction: inbound\n\
                   Caller-Caller-ID-Number: %2B4951112345\n\
                   Caller-Destination-Number: 201\n";
        let ev = EslEvent::parse(raw);
        assert_eq!(ev.name, "CHANNEL_ANSWER");
        assert_eq!(ev.header("Caller-Caller-ID-Number"), Some("+4951112345"));

        let (call_id, kind, payload) = ev.to_domain_event().unwrap();
        assert_eq!(call_id, "1234-abcd");
        assert_eq!(kind, "CHANNEL_ANSWER");
        assert_eq!(payload["to"], "201");
    }

    #[test]
    fn ignores_unknown_events() {
        let ev = EslEvent::parse("Event-Name: HEARTBEAT\nUnique-ID: x\n");
        assert!(ev.to_domain_event().is_none());
    }
}
