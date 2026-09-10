//! Monitoring, Health und Self-Healing (Kapitel 18).

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::compiler;
use crate::infrastructure::config::Paths;
use crate::infrastructure::db::Db;
use crate::infrastructure::time::{now_rfc3339, parse_rfc3339};
use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Maintenance,
    Degraded,
    Recovery,
    Unhealthy,
    UnsafeOverrideActive,
}

impl HealthState {
    pub fn as_str(self) -> &'static str {
        match self {
            HealthState::Healthy => "HEALTHY",
            HealthState::Degraded => "DEGRADED",
            HealthState::Unhealthy => "UNHEALTHY",
            HealthState::Recovery => "RECOVERY",
            HealthState::UnsafeOverrideActive => "UNSAFE_OVERRIDE_ACTIVE",
            HealthState::Maintenance => "MAINTENANCE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub state: HealthState,
    pub detail: String,
    /// Latenz in Millisekunden, sofern messbar.
    pub latency_ms: Option<u64>,
}

impl Check {
    fn ok(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: HealthState::Healthy,
            detail: detail.into(),
            latency_ms: None,
        }
    }
    fn degraded(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: HealthState::Degraded,
            detail: detail.into(),
            latency_ms: None,
        }
    }
    fn unhealthy(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: HealthState::Unhealthy,
            detail: detail.into(),
            latency_ms: None,
        }
    }
    fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub state: HealthState,
    pub ts: String,
    pub checks: Vec<Check>,
    pub active_generation: Option<i64>,
}

/// Schwellwerte aus Kapitel 18.
const STORAGE_WARN_PERCENT: f64 = 20.0;
const STORAGE_CRIT_PERCENT: f64 = 10.0;
const STORAGE_CRIT_BYTES: u64 = 1024 * 1024 * 1024;
const CERT_WARN_DAYS: i64 = 30;
const CERT_HIGH_DAYS: i64 = 14;
const CERT_CRIT_DAYS: i64 = 7;

/// Fuehrt alle Pflichtchecks aus (Kapitel 18).
pub fn collect(db: &Db, paths: &Paths, freeswitch_expected: bool) -> HealthReport {
    let mut checks = Vec::new();

    // pbx-web laeuft, sonst gaebe es diesen Aufruf nicht.
    checks.push(Check::ok("pbx-web", "Erreichbar"));

    // SQLite
    let t0 = std::time::Instant::now();
    if db.integrity_ok() {
        checks.push(
            Check::ok("sqlite", "integrity_check ok").with_latency(t0.elapsed().as_millis() as u64),
        );
    } else {
        checks.push(Check::unhealthy("sqlite", "integrity_check fehlgeschlagen"));
    }

    // KMS
    let kek = paths.kms_dir().join("root.key");
    if kek.exists() {
        let backend = if crate::kms::hardware_backing_available() {
            "hardwaregestuetzt"
        } else {
            "CSPRNG"
        };
        checks.push(Check::ok("kms", format!("Root KEK vorhanden ({backend})")));
    } else {
        checks.push(Check::unhealthy("kms", "Root KEK fehlt"));
    }

    // Secret Broker
    checks.push(if paths.broker_socket().exists() {
        Check::ok("secret-broker", "Socket vorhanden")
    } else {
        Check::degraded("secret-broker", "Socket nicht vorhanden")
    });

    // Prozesse ueber ihre Heartbeat-Dateien
    for (name, file) in [
        ("pbx-core", "pbx-core.heartbeat"),
        ("pbx-worker", "pbx-worker.heartbeat"),
        ("pbx-xml-adapter", "pbx-xml-adapter.heartbeat"),
    ] {
        checks.push(heartbeat_check(name, &paths.state_dir().join(file)));
    }

    // FreeSWITCH und Sofia-Profile
    if freeswitch_expected {
        checks.push(heartbeat_check(
            "freeswitch",
            &paths.state_dir().join("freeswitch.heartbeat"),
        ));
        for p in ["sofia-local", "sofia-public", "sofia-trunk"] {
            checks.push(heartbeat_check(
                p,
                &paths.state_dir().join(format!("{p}.heartbeat")),
            ));
        }
    } else {
        checks.push(Check {
            name: "freeswitch".into(),
            state: HealthState::Maintenance,
            detail: "Telephony Core ist in dieser Installation nicht aktiviert".into(),
            latency_ms: None,
        });
    }

    // TLS-Zertifikate
    checks.push(certificate_check(&paths.certs_dir()));

    // Media Store
    checks.push(if paths.media_dir().exists() {
        Check::ok("media-store", "Verfuegbar")
    } else {
        Check::degraded("media-store", "Verzeichnis fehlt")
    });

    // Storage
    checks.push(storage_check(&paths.data));

    // Audit Ledger
    let audit = crate::audit::Audit::new();
    checks.push(match audit.verify(db) {
        Ok(v) if v.valid => Check::ok(
            "audit-ledger",
            format!("{} Eintraege, Kette intakt", v.entries),
        ),
        Ok(v) => Check::unhealthy(
            "audit-ledger",
            format!("Kette gebrochen ab Eintrag {}", v.broken_at.unwrap_or(0)),
        ),
        Err(e) => Check::unhealthy("audit-ledger", format!("Pruefung fehlgeschlagen: {e}")),
    });

    // Backup
    checks.push(backup_check(db));

    // HA-Integration
    checks.push(if std::env::var("SUPERVISOR_TOKEN").is_ok() {
        Check::ok("ha-integration", "Supervisor-Token vorhanden")
    } else {
        Check::degraded(
            "ha-integration",
            "Kein Supervisor-Token; Integration inaktiv",
        )
    });

    // Hard-Error Override (Kapitel 18.2)
    let override_active = setting_flag(db, "override.unsafe_active");
    if override_active {
        checks.push(Check {
            name: "override".into(),
            state: HealthState::UnsafeOverrideActive,
            detail: "Ein Hard Error wurde bewusst uebersteuert".into(),
            latency_ms: None,
        });
    }

    let state = aggregate(&checks);
    HealthReport {
        state,
        ts: now_rfc3339(),
        checks,
        active_generation: compiler::active_generation(db),
    }
}

fn aggregate(checks: &[Check]) -> HealthState {
    if checks
        .iter()
        .any(|c| c.state == HealthState::UnsafeOverrideActive)
    {
        return HealthState::UnsafeOverrideActive;
    }
    if checks.iter().any(|c| c.state == HealthState::Unhealthy) {
        return HealthState::Unhealthy;
    }
    if checks.iter().any(|c| c.state == HealthState::Recovery) {
        return HealthState::Recovery;
    }
    if checks.iter().any(|c| c.state == HealthState::Degraded) {
        return HealthState::Degraded;
    }
    HealthState::Healthy
}

fn setting_flag(db: &Db, key: &str) -> bool {
    let guard = db.lock();
    guard
        .query_row(
            "SELECT value FROM settings WHERE key=?1",
            rusqlite::params![key],
            |r| r.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Ein Prozess gilt als gesund, wenn seine Heartbeat-Datei juenger als 90 s ist.
fn heartbeat_check(name: &str, path: &Path) -> Check {
    let Ok(meta) = std::fs::metadata(path) else {
        return Check::degraded(name, "Kein Heartbeat");
    };
    let age = meta
        .modified()
        .ok()
        .and_then(|m| m.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX);
    if age <= 90 {
        Check::ok(name, format!("Heartbeat vor {age} s"))
    } else if age <= 300 {
        Check::degraded(name, format!("Heartbeat ist {age} s alt"))
    } else {
        Check::unhealthy(name, format!("Heartbeat ist {age} s alt"))
    }
}

fn certificate_check(certs: &Path) -> Check {
    let marker = certs.join("expires_at");
    let Ok(raw) = std::fs::read_to_string(&marker) else {
        return Check {
            name: "tls".into(),
            state: HealthState::Maintenance,
            detail: "Kein Zertifikat konfiguriert".into(),
            latency_ms: None,
        };
    };
    let Some(expiry) = parse_rfc3339(raw.trim()) else {
        return Check::degraded("tls", "Ablaufdatum unlesbar");
    };
    let days = (expiry - crate::infrastructure::time::now()).whole_days();
    if days <= 0 {
        Check::unhealthy("tls", "Zertifikat ist abgelaufen")
    } else if days <= CERT_CRIT_DAYS {
        Check::unhealthy("tls", format!("Zertifikat laeuft in {days} Tagen ab"))
    } else if days <= CERT_HIGH_DAYS {
        Check::degraded(
            "tls",
            format!("Zertifikat laeuft in {days} Tagen ab (hoch)"),
        )
    } else if days <= CERT_WARN_DAYS {
        Check::degraded("tls", format!("Zertifikat laeuft in {days} Tagen ab"))
    } else {
        Check::ok("tls", format!("Zertifikat gueltig fuer {days} Tage"))
    }
}

fn storage_check(data: &Path) -> Check {
    match available_bytes(data) {
        None => Check::degraded("storage", "Belegung nicht ermittelbar"),
        Some((free, total)) => {
            let percent = if total == 0 {
                0.0
            } else {
                free as f64 * 100.0 / total as f64
            };
            let gib = free as f64 / 1024.0 / 1024.0 / 1024.0;
            let detail = format!("{gib:.1} GiB frei ({percent:.1} %)");
            if percent < STORAGE_CRIT_PERCENT || free < STORAGE_CRIT_BYTES {
                Check::unhealthy("storage", detail)
            } else if percent < STORAGE_WARN_PERCENT {
                Check::degraded("storage", detail)
            } else {
                Check::ok("storage", detail)
            }
        }
    }
}

fn backup_check(db: &Db) -> Check {
    let guard = db.lock();
    let last: Option<String> = guard
        .query_row(
            "SELECT created_at FROM backups ORDER BY created_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    match last {
        None => Check::degraded("backup", "Noch kein Backup erstellt"),
        Some(ts) => {
            let age_days = parse_rfc3339(&ts)
                .map(|t| (crate::infrastructure::time::now() - t).whole_days())
                .unwrap_or(999);
            if age_days > 7 {
                Check::degraded("backup", format!("Letztes Backup vor {age_days} Tagen"))
            } else {
                Check::ok("backup", format!("Letztes Backup vor {age_days} Tagen"))
            }
        }
    }
}

#[cfg(unix)]
fn available_bytes(path: &Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut st: libc_statvfs = std::mem::zeroed();
        if statvfs(c.as_ptr(), &mut st) != 0 {
            return None;
        }
        let bsize = if st.f_frsize > 0 {
            st.f_frsize
        } else {
            st.f_bsize
        };
        Some((
            st.f_bavail.saturating_mul(bsize),
            st.f_blocks.saturating_mul(bsize),
        ))
    }
}

#[cfg(not(unix))]
fn available_bytes(_p: &Path) -> Option<(u64, u64)> {
    None
}

// Minimale statvfs-Bindung, damit kein zusaetzlicher Crate noetig ist.
#[cfg(unix)]
#[repr(C)]
#[allow(non_camel_case_types)]
struct libc_statvfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    __reserved: [i32; 6],
}

#[cfg(unix)]
extern "C" {
    fn statvfs(path: *const std::os::raw::c_char, buf: *mut libc_statvfs) -> i32;
}

/// Schreibt einen Heartbeat fuer den eigenen Prozess.
pub fn beat(paths: &Paths, component: &str) -> Result<()> {
    std::fs::create_dir_all(paths.state_dir())?;
    std::fs::write(
        paths.state_dir().join(format!("{component}.heartbeat")),
        now_rfc3339(),
    )?;
    Ok(())
}

pub fn persist(db: &Db, report: &HealthReport) -> Result<()> {
    let guard = db.lock();
    guard.execute(
        "INSERT INTO health_snapshots (id,ts,state,checks) VALUES (?1,?2,?3,?4)",
        rusqlite::params![
            crate::domain::ids::new_prefixed("hlt"),
            report.ts,
            report.state.as_str(),
            serde_json::to_string(&report.checks)?
        ],
    )?;
    guard.execute(
        "DELETE FROM health_snapshots WHERE id NOT IN
         (SELECT id FROM health_snapshots ORDER BY ts DESC LIMIT 2880)",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    #[test]
    fn aggregates_worst_state() {
        assert_eq!(aggregate(&[Check::ok("a", "")]), HealthState::Healthy);
        assert_eq!(
            aggregate(&[Check::ok("a", ""), Check::degraded("b", "")]),
            HealthState::Degraded
        );
        assert_eq!(
            aggregate(&[Check::degraded("a", ""), Check::unhealthy("b", "")]),
            HealthState::Unhealthy
        );
        let mut o = Check::ok("o", "");
        o.state = HealthState::UnsafeOverrideActive;
        assert_eq!(
            aggregate(&[Check::unhealthy("a", ""), o]),
            HealthState::UnsafeOverrideActive
        );
    }

    #[test]
    fn collect_runs_all_mandatory_checks() {
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let dir = std::env::temp_dir().join(format!("extelio-hlt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = Paths {
            data: dir.clone(),
            run: dir.join("run"),
            web_root: dir.join("web"),
        };
        paths.ensure().unwrap();
        crate::kms::Kms::open_or_init(&paths.kms_dir()).unwrap();

        let r = collect(&db, &paths, false);
        let names: Vec<&str> = r.checks.iter().map(|c| c.name.as_str()).collect();
        for required in [
            "pbx-web",
            "pbx-core",
            "pbx-worker",
            "secret-broker",
            "sqlite",
            "kms",
            "freeswitch",
            "tls",
            "media-store",
            "storage",
            "audit-ledger",
            "backup",
            "ha-integration",
        ] {
            assert!(names.contains(&required), "Pflichtcheck '{required}' fehlt");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
