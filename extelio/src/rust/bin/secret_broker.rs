//! `extelio-secret-broker` - lokaler Secret Broker (Kapitel 7.4).
//!
//! Hoert auf einem Unix-Socket, prueft die Policy jeder Secret Class gegen den
//! anfragenden Principal und gibt Klartext ausschliesslich an berechtigte
//! Prozesse heraus. Jeder Zugriff wird auditiert.

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use extelio::audit::Audit;
use extelio::infrastructure::config::Paths;
use extelio::infrastructure::db::Db;
use extelio::kms::broker::{handle, BrokerRequest};
use extelio::kms::Kms;
use extelio::{log_error, log_info, log_warn};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            log_error!("secret-broker", "Beendet: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> extelio::Result<()> {
    let paths = Paths::from_env();
    paths.ensure()?;
    let db = Db::open(&paths.db_file())?;
    let kms = Arc::new(Kms::open_or_init(&paths.kms_dir())?);
    let audit = Audit::new();

    let socket = paths.broker_socket();
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)
        .map_err(|e| extelio::Error::Internal(format!("Socket {socket:?}: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&socket)?.permissions();
        // Nur Prozesse der eigenen Gruppe duerfen den Broker ansprechen.
        p.set_mode(0o660);
        std::fs::set_permissions(&socket, p)?;
    }

    log_info!("secret-broker", "Bereit auf {socket:?}");
    extelio::health::beat(&paths, "secret-broker")?;

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                log_warn!("secret-broker", "Verbindung abgelehnt: {e}");
                continue;
            }
        };
        let db = db.clone();
        let kms = kms.clone();
        let audit = audit.clone();
        let paths = paths.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).await.is_err() {
                return;
            }
            let response = match serde_json::from_str::<BrokerRequest>(line.trim()) {
                Ok(req) => handle(&db, &kms, &audit, &req),
                Err(_) => extelio::kms::broker::BrokerResponse {
                    ok: false,
                    value: None,
                    error: Some("Ungueltige Anfrage".into()),
                },
            };
            let payload = serde_json::to_string(&response).unwrap_or_default();
            let stream = reader.get_mut();
            let _ = stream.write_all(payload.as_bytes()).await;
            let _ = stream.write_all(b"\n").await;
            let _ = stream.flush().await;
            let _ = extelio::health::beat(&paths, "secret-broker");
        });
    }
}
