//! `extelio-worker` - Hintergrunddienste (Kapitel 18).
//!
//! Aufgaben: Health-Erhebung, Home-Assistant-Aktualisierung, Sitzungs- und
//! Retention-Pflege, Audit-Checkpoints, Reaktion auf neue Konfigurations-
//! generationen und der ESL-Reconnect mit begrenztem Self-Healing.

use std::time::Duration;

use extelio::adapters::{esl::EslConnection, ha};
use extelio::infrastructure::config::{Options, Paths};
use extelio::infrastructure::db::Db;
use extelio::kms::Kms;
use extelio::{health, log_error, log_info, log_warn};

const TICK: Duration = Duration::from_secs(30);

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            log_error!("pbx-worker", "Beendet: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> extelio::Result<()> {
    let paths = Paths::from_env();
    paths.ensure()?;
    let options = Options::load(&paths.options_file())?;
    std::env::set_var("EXTELIO_LOG_LEVEL", &options.log_level);

    let db = Db::open(&paths.db_file())?;
    let kms = Kms::open_or_init(&paths.kms_dir())?;
    let audit = extelio::audit::Audit::new();
    let ha_client = ha::HaClient::from_env();
    if ha_client.is_none() {
        log_info!(
            "pbx-worker",
            "Kein Supervisor-Token - Home-Assistant-Integration inaktiv"
        );
    }

    let freeswitch_enabled = std::env::var("EXTELIO_FREESWITCH_ENABLED")
        .map(|v| v == "1")
        .unwrap_or(false);
    let mut esl: Option<EslConnection> = None;
    let mut esl_backoff = 1u64;
    let mut ticks = 0u64;

    log_info!("pbx-worker", "Gestartet");
    loop {
        ticks += 1;
        health::beat(&paths, "pbx-worker")?;

        // 1. Health erheben und persistieren.
        let report = health::collect(&db, &paths, freeswitch_enabled);
        if let Err(e) = health::persist(&db, &report) {
            log_warn!("pbx-worker", "Health konnte nicht gespeichert werden: {e}");
        }

        // 2. Nur nicht-personenbezogene Kennzahlen an Home Assistant (Kapitel 13.1).
        if let Some(client) = &ha_client {
            let state = ha::public_state(&db, &report);
            if let Err(e) = client.publish(&state).await {
                log_warn!("pbx-worker", "Home Assistant nicht aktualisiert: {e}");
            }
        }

        // 3. Abgelaufene Sitzungen entfernen (Kapitel 6.4).
        match extelio::auth::session::purge_expired(&db) {
            Ok(n) if n > 0 => log_info!("pbx-worker", "{n} abgelaufene Sitzungen entfernt"),
            Err(e) => log_warn!("pbx-worker", "Sitzungspflege fehlgeschlagen: {e}"),
            _ => {}
        }

        // 4. Neue Generation aktivieren lassen (Kapitel 9.2).
        let reload_marker = paths.state_dir().join("reload.request");
        if reload_marker.exists() {
            let generation = std::fs::read_to_string(&reload_marker).unwrap_or_default();
            log_info!(
                "pbx-worker",
                "Konfigurationsgeneration {generation} wird uebernommen"
            );
            if let Some(conn) = esl.as_mut() {
                if let Err(e) = conn.reload_xml().await {
                    log_warn!("pbx-worker", "reloadxml fehlgeschlagen: {e}");
                } else {
                    for profile in ["local", "public", "trunk"] {
                        let _ = conn.rescan_profile(profile).await;
                    }
                }
            }
            let _ = std::fs::remove_file(&reload_marker);
        }

        // 5. Retention anwenden (Kapitel 14.1), stuendlich.
        if ticks % 120 == 1 {
            if let Err(e) = apply_retention(&db) {
                log_warn!("pbx-worker", "Retention fehlgeschlagen: {e}");
            }
            // Audit-Checkpoint mit einem aus dem KMS abgeleiteten Schluessel.
            let mac_key = kms.key_id().as_bytes().to_vec();
            if let Err(e) = audit.checkpoint(&db, &mac_key) {
                log_warn!("pbx-worker", "Audit-Checkpoint fehlgeschlagen: {e}");
            }
        }

        // 6. ESL-Verbindung halten (Kapitel 18.1: kontrollierter Reconnect).
        if freeswitch_enabled {
            if esl.is_none() {
                match connect_esl(&db, &kms).await {
                    Ok(conn) => {
                        log_info!("pbx-worker", "ESL verbunden");
                        esl = Some(conn);
                        esl_backoff = 1;
                    }
                    Err(e) => {
                        log_warn!("pbx-worker", "ESL nicht verbunden: {e}");
                        tokio::time::sleep(Duration::from_secs(esl_backoff)).await;
                        esl_backoff = (esl_backoff * 2).min(60);
                    }
                }
            }
            if let Some(conn) = esl.as_mut() {
                match conn.sofia_status().await {
                    Ok(status) => update_trunk_status(&db, &status),
                    Err(e) => {
                        log_warn!("pbx-worker", "ESL-Verbindung verloren: {e}");
                        esl = None;
                    }
                }
            }
        }

        tokio::time::sleep(TICK).await;
    }
}

async fn connect_esl(db: &Db, kms: &Kms) -> extelio::Result<EslConnection> {
    let secret_id: String = {
        let guard = db.lock();
        guard
            .query_row(
                "SELECT id FROM secrets WHERE class='SERVICE_TOKEN' AND label='esl' AND revoked_at IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(|_| extelio::Error::NotFound("ESL-Secret nicht vorhanden".into()))?
    };
    let secret = kms.load(db, &secret_id)?;
    let password = String::from_utf8(secret.to_vec())
        .map_err(|_| extelio::Error::Internal("ESL-Secret unlesbar".into()))?;
    EslConnection::connect(8021, &password).await
}

/// Wertet `sofia status` aus und pflegt den Registrierungszustand der Trunks.
fn update_trunk_status(db: &Db, status: &str) {
    let guard = db.lock();
    let Ok(mut stmt) = guard.prepare("SELECT id,name FROM trunks WHERE enabled=1") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    else {
        return;
    };
    let now = extelio::infrastructure::time::now_rfc3339();
    for row in rows.flatten() {
        let (id, name) = row;
        let registered = status
            .lines()
            .any(|l| l.contains(&name) && (l.contains("REGED") || l.contains("RUNNING")));
        let new_status = if registered {
            "registered"
        } else {
            "unregistered"
        };
        let _ = guard.execute(
            "UPDATE trunks SET status=?2, last_status_at=?3 WHERE id=?1",
            rusqlite::params![id, new_status, now],
        );
    }
}

/// Loescht Daten nach Ablauf der zweckgebundenen Aufbewahrungsfrist.
fn apply_retention(db: &Db) -> extelio::Result<()> {
    let policies: Vec<(String, i64)> = {
        let guard = db.lock();
        let mut stmt = guard.prepare("SELECT key,days FROM retention_policies")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<std::result::Result<_, _>>()?
    };

    for (key, days) in policies {
        let cutoff = extelio::infrastructure::time::to_rfc3339(
            extelio::infrastructure::time::now() - time::Duration::days(days),
        );
        let guard = db.lock();
        let removed = match key.as_str() {
            "call_events" => guard.execute(
                "DELETE FROM call_events WHERE ts < ?1",
                rusqlite::params![cutoff],
            )?,
            "security_audit" => guard.execute(
                "DELETE FROM audit_log WHERE ts < ?1",
                rusqlite::params![cutoff],
            )?,
            "system_logs" => guard.execute(
                "DELETE FROM health_snapshots WHERE ts < ?1",
                rusqlite::params![cutoff],
            )?,
            _ => 0,
        };
        if removed > 0 {
            log_info!(
                "pbx-worker",
                "Retention '{key}': {removed} Datensaetze entfernt"
            );
        }
    }
    Ok(())
}
