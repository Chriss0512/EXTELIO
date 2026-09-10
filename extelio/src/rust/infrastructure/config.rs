//! Laufzeitkonfiguration.
//!
//! Quelle ist ausschliesslich `/data/options.json` der Home-Assistant-App
//! (Kapitel 19.1). Langlebige Secrets stehen niemals in `options.json`
//! (Kapitel 19.1, letzter Absatz) - sie liegen im KMS (Kapitel 7).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

fn d_web_mode() -> String {
    "http_only".into()
}
fn d_web_http_port() -> u16 {
    8080
}
fn d_web_https_port() -> u16 {
    8443
}
fn d_sip_bind_mode() -> String {
    "single".into()
}
fn d_sip_bind_ip() -> String {
    "0.0.0.0".into()
}
fn d_local_sip() -> u16 {
    5060
}
fn d_local_sips() -> u16 {
    5061
}
fn d_public_sip() -> u16 {
    5080
}
fn d_public_sips() -> u16 {
    5081
}
fn d_trunk_sip() -> u16 {
    5090
}
fn d_trunk_sips() -> u16 {
    5091
}
fn d_rtp_start() -> u16 {
    16384
}
fn d_rtp_end() -> u16 {
    32768
}
fn d_log_level() -> String {
    "info".into()
}

/// Bootstrap Options aus `/data/options.json` (Kapitel 19.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    /// `https_only` | `http_only` | `http_https` | `http_behind_proxy` (Kapitel 5.3)
    #[serde(default = "d_web_mode")]
    pub web_mode: String,
    #[serde(default = "d_web_http_port")]
    pub web_http_port: u16,
    #[serde(default = "d_web_https_port")]
    pub web_https_port: u16,

    /// `single` | `per_profile` (Kapitel 5.2 Advanced)
    #[serde(default = "d_sip_bind_mode")]
    pub sip_bind_mode: String,
    #[serde(default = "d_sip_bind_ip")]
    pub sip_bind_ip: String,

    #[serde(default = "d_local_sip")]
    pub local_sip_port: u16,
    #[serde(default = "d_local_sips")]
    pub local_sips_port: u16,
    #[serde(default = "d_public_sip")]
    pub public_sip_port: u16,
    #[serde(default = "d_public_sips")]
    pub public_sips_port: u16,
    #[serde(default = "d_trunk_sip")]
    pub trunk_sip_port: u16,
    #[serde(default = "d_trunk_sips")]
    pub trunk_sips_port: u16,

    #[serde(default = "d_rtp_start")]
    pub rtp_start_port: u16,
    #[serde(default = "d_rtp_end")]
    pub rtp_end_port: u16,

    #[serde(default)]
    pub ipv6_enabled: bool,
    #[serde(default)]
    pub public_push_enabled: bool,

    /// Kanonischer Hostname; bindet WebAuthn RP-ID und ACME (Kapitel 5.3/5.5).
    #[serde(default)]
    pub canonical_hostname: String,

    /// Vertrauenswuerdige Reverse-Proxy-Quellen fuer `X-Forwarded-For`.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,

    #[serde(default = "d_log_level")]
    pub log_level: String,
}

impl Default for Options {
    fn default() -> Self {
        serde_json::from_str("{}").expect("Defaults sind vollstaendig")
    }
}

impl Options {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        let opts: Options = serde_json::from_str(&raw)
            .map_err(|e| Error::Validation(format!("options.json ungueltig: {e}")))?;
        opts.validate()?;
        Ok(opts)
    }

    /// Kapitel 5.2: "Ports sind frei konfigurierbar, aber Konflikte werden validiert."
    pub fn validate(&self) -> Result<()> {
        let mut ports: Vec<(&str, u16)> = vec![
            ("local_sip_port", self.local_sip_port),
            ("local_sips_port", self.local_sips_port),
            ("public_sip_port", self.public_sip_port),
            ("public_sips_port", self.public_sips_port),
            ("trunk_sip_port", self.trunk_sip_port),
            ("trunk_sips_port", self.trunk_sips_port),
        ];
        if self.web_enabled_http() {
            ports.push(("web_http_port", self.web_http_port));
        }
        if self.web_enabled_https() {
            ports.push(("web_https_port", self.web_https_port));
        }

        // Im Modus `per_profile` duerfen SIP-Ports pro Bind-IP gleich sein.
        let check_sip_collisions = self.sip_bind_mode != "per_profile";

        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                let (an, a) = ports[i];
                let (bn, b) = ports[j];
                if a != b {
                    continue;
                }
                let both_sip = an.contains("sip") && bn.contains("sip");
                if both_sip && !check_sip_collisions {
                    continue;
                }
                return Err(Error::Validation(format!(
                    "Portkonflikt: {an} und {bn} belegen beide {a}"
                )));
            }
        }

        if self.rtp_start_port >= self.rtp_end_port {
            return Err(Error::Validation(
                "rtp_start_port muss kleiner als rtp_end_port sein".into(),
            ));
        }
        if (self.rtp_end_port - self.rtp_start_port) < 100 {
            return Err(Error::Validation(
                "RTP-Range muss mindestens 100 Ports umfassen".into(),
            ));
        }
        for (name, p) in &ports {
            if *p < 1024 {
                return Err(Error::Validation(format!(
                    "{name}={p}: privilegierte Ports unter 1024 sind nicht zugelassen"
                )));
            }
        }
        if !matches!(
            self.web_mode.as_str(),
            "https_only" | "http_only" | "http_https" | "http_behind_proxy"
        ) {
            return Err(Error::Validation(format!(
                "web_mode '{}' unbekannt",
                self.web_mode
            )));
        }
        Ok(())
    }

    pub fn web_enabled_http(&self) -> bool {
        matches!(
            self.web_mode.as_str(),
            "http_only" | "http_https" | "http_behind_proxy"
        )
    }

    pub fn web_enabled_https(&self) -> bool {
        matches!(self.web_mode.as_str(), "https_only" | "http_https")
    }

    /// Kapitel 6.4: HTTP-Modus benutzt getrennte Session-/Cookie-Namen.
    pub fn behind_trusted_proxy(&self) -> bool {
        self.web_mode == "http_behind_proxy"
    }

    /// Kapitel 5.3: Passkeys/WebAuthn nur in gueltigem Secure Context.
    pub fn secure_context_available(&self) -> bool {
        self.web_enabled_https() || self.behind_trusted_proxy()
    }
}

/// Pfadlayout gemaess Kapitel 22.5.
#[derive(Debug, Clone)]
pub struct Paths {
    pub data: PathBuf,
    pub run: PathBuf,
    pub web_root: PathBuf,
}

impl Paths {
    pub fn from_env() -> Self {
        let data = std::env::var("EXTELIO_DATA_DIR").unwrap_or_else(|_| "/data".into());
        let run = std::env::var("EXTELIO_RUN_DIR").unwrap_or_else(|_| "/run/extelio".into());
        let web_root =
            std::env::var("EXTELIO_WEB_ROOT").unwrap_or_else(|_| "/opt/extelio/web".into());
        Self {
            data: data.into(),
            run: run.into(),
            web_root: web_root.into(),
        }
    }

    pub fn options_file(&self) -> PathBuf {
        std::env::var("EXTELIO_OPTIONS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.data.join("options.json"))
    }

    pub fn db_file(&self) -> PathBuf {
        self.data.join("db/pbx.sqlite3")
    }
    pub fn config_dir(&self) -> PathBuf {
        self.data.join("config")
    }
    pub fn current_config(&self) -> PathBuf {
        self.data.join("config/current")
    }
    pub fn media_dir(&self) -> PathBuf {
        self.data.join("media/sha256")
    }
    pub fn audit_dir(&self) -> PathBuf {
        self.data.join("audit")
    }
    pub fn backup_dir(&self) -> PathBuf {
        self.data.join("backup")
    }
    pub fn certs_dir(&self) -> PathBuf {
        self.data.join("certs")
    }
    pub fn kms_dir(&self) -> PathBuf {
        self.data.join("kms")
    }
    pub fn catalogs_dir(&self) -> PathBuf {
        self.data.join("catalogs")
    }
    pub fn state_dir(&self) -> PathBuf {
        self.data.join("state")
    }
    pub fn diagnostics_dir(&self) -> PathBuf {
        self.data.join("diagnostics")
    }
    pub fn broker_socket(&self) -> PathBuf {
        self.run.join("secret-broker.sock")
    }

    /// Legt das persistente Layout an (Kapitel 22.5).
    pub fn ensure(&self) -> Result<()> {
        for d in [
            self.data.join("db"),
            self.config_dir(),
            self.media_dir(),
            self.audit_dir(),
            self.backup_dir(),
            self.certs_dir(),
            self.kms_dir(),
            self.catalogs_dir(),
            self.state_dir(),
            self.diagnostics_dir(),
            self.run.clone(),
        ] {
            std::fs::create_dir_all(&d)?;
        }
        restrict(&self.kms_dir())?;
        restrict(&self.certs_dir())?;
        restrict(&self.backup_dir())?;
        Ok(())
    }
}

#[cfg(unix)]
fn restrict(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(p)?.permissions();
    perm.set_mode(0o700);
    std::fs::set_permissions(p, perm)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict(_p: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_specification() {
        let o = Options::default();
        assert_eq!(o.local_sip_port, 5060);
        assert_eq!(o.public_sip_port, 5080);
        assert_eq!(o.trunk_sip_port, 5090);
        assert_eq!(o.rtp_start_port, 16384);
        assert_eq!(o.rtp_end_port, 32768);
        assert!(!o.ipv6_enabled, "Kapitel 5.5: IPv6 Default false");
        assert!(
            !o.public_push_enabled,
            "Kapitel 5.5: Public SIP Default OFF"
        );
        o.validate().unwrap();
    }

    #[test]
    fn detects_port_conflicts() {
        let o = Options {
            public_sip_port: 5060,
            ..Options::default()
        };
        assert!(o.validate().is_err());
    }

    #[test]
    fn allows_shared_sip_ports_per_profile_binding() {
        let o = Options {
            sip_bind_mode: "per_profile".into(),
            public_sip_port: 5060,
            trunk_sip_port: 5060,
            ..Options::default()
        };
        o.validate().unwrap();
    }
}
