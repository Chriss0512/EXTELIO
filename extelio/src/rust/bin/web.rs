//! `extelio-web` - HTTP-Schnittstelle und Auslieferung der Weboberflaeche.

use std::net::SocketAddr;

use extelio::application::{http, state::AppState};
use extelio::infrastructure::config::{Options, Paths};
use extelio::infrastructure::db::Db;
use extelio::infrastructure::migrations;
use extelio::kms::Kms;
use extelio::{log_error, log_info, log_warn};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            log_error!("pbx-web", "Beendet: {e}");
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
    // Bootstrap laeuft normalerweise vorher; im Entwicklungsmodus wird hier
    // nachgezogen, damit der Dienst allein startklar ist.
    migrations::run(&db)?;
    let kms = Kms::open_or_init(&paths.kms_dir())?;

    let state = AppState::new(db, kms, paths.clone(), options.clone());
    if !state.has_users() {
        log_info!(
            "pbx-web",
            "Kein Benutzer vorhanden - Erstinbetriebnahme ist offen"
        );
    }

    let app = http::router(state);

    let port = if options.web_enabled_http() {
        options.web_http_port
    } else {
        // Im Modus https_only terminiert der Reverse Proxy bzw. der TLS-Wrapper
        // vor diesem Dienst; intern wird weiterhin auf dem HTTP-Port gebunden.
        options.web_https_port
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| extelio::Error::Internal(format!("Port {port} nicht belegbar: {e}")))?;

    log_info!("pbx-web", "Weboberflaeche auf http://0.0.0.0:{port}");
    if options.canonical_hostname.is_empty() {
        log_warn!(
            "pbx-web",
            "Kein kanonischer Hostname gesetzt - Passkeys und Zertifikate bleiben deaktiviert"
        );
    }

    let beat_paths = paths.clone();
    tokio::spawn(async move {
        loop {
            let _ = extelio::health::beat(&beat_paths, "pbx-web");
            let _ = extelio::health::beat(&beat_paths, "pbx-core");
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| extelio::Error::Internal(format!("HTTP-Server: {e}")))?;
    log_info!("pbx-web", "Beendet");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
