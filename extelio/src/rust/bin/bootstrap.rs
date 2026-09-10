//! `extelio-bootstrap` - erster Dienst der Startsequenz (Kapitel 21).
//!
//! Aufgaben: Pfadlayout anlegen, Optionen validieren, Datenbank migrieren,
//! Root KEK initialisieren. Der Dienst ist ein Oneshot und beendet sich danach.
//! Schlaegt er fehl, startet kein weiterer Dienst.

use extelio::infrastructure::config::{Options, Paths};
use extelio::infrastructure::db::Db;
use extelio::infrastructure::migrations;
use extelio::kms::Kms;
use extelio::{log_error, log_info};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            log_error!("bootstrap", "Start abgebrochen: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> extelio::Result<()> {
    let paths = Paths::from_env();
    paths.ensure()?;
    log_info!("bootstrap", "EXTELIO {} startet", extelio::VERSION);

    let options = Options::load(&paths.options_file())?;
    std::env::set_var("EXTELIO_LOG_LEVEL", &options.log_level);
    log_info!(
        "bootstrap",
        "Web-Modus {}, SIP local/public/trunk {}/{}/{}, RTP {}-{}",
        options.web_mode,
        options.local_sip_port,
        options.public_sip_port,
        options.trunk_sip_port,
        options.rtp_start_port,
        options.rtp_end_port
    );
    if !options.public_push_enabled {
        log_info!("bootstrap", "PUBLIC/PUSH ist deaktiviert (Voreinstellung)");
    }

    let db = Db::open(&paths.db_file())?;
    let version = migrations::run(&db)?;
    log_info!("bootstrap", "Datenbankschema auf Version {version}");

    let kms = Kms::open_or_init(&paths.kms_dir())?;
    log_info!(
        "bootstrap",
        "KMS bereit, Root KEK {} ({:?})",
        kms.key_id(),
        kms.backend()
    );

    // Startereignis ins Audit-Ledger (Kapitel 14.4).
    let audit = extelio::audit::Audit::new();
    audit.record(
        &db,
        extelio::audit::Event::system("system.start")
            .detail(serde_json::json!({ "version": extelio::VERSION, "schema": version })),
    )?;

    extelio::health::beat(&paths, "pbx-bootstrap")?;
    log_info!("bootstrap", "Bootstrap abgeschlossen");
    Ok(())
}
