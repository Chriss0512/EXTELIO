//! Telephony-Domaene (Kapitel 8.1, 8.3).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub number: String,
    pub name: String,
    pub kind: String,
    pub site_id: Option<String>,
    pub department_id: Option<String>,
    pub user_id: Option<String>,
    pub voicemail: bool,
    pub dnd: bool,
    pub forward_target: Option<String>,
    pub outbound_caller_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub mac: Option<String>,
    pub device_type: String,
    pub provisioning_mode: String,
    pub enrollment_state: String,
    pub firmware: Option<String>,
    pub ip: Option<String>,
    pub status: String,
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub lines: Vec<DeviceLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLine {
    pub id: String,
    pub line_no: i64,
    pub extension_id: Option<String>,
    pub extension_number: Option<String>,
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub key: String,
    pub name: String,
    pub registrar: Option<String>,
    pub proxy: Option<String>,
    pub transport: String,
    pub codecs: Vec<String>,
    pub number_format: String,
    pub auth_capabilities: Vec<String>,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trunk {
    pub id: String,
    pub name: String,
    pub provider_profile_id: String,
    pub provider_name: Option<String>,
    pub mode: String,
    pub host: String,
    pub port: u16,
    pub transport: String,
    pub auth_username: Option<String>,
    pub from_user: Option<String>,
    pub from_domain: Option<String>,
    pub inbound_trust: String,
    pub security_profile: String,
    pub enabled: bool,
    pub status: String,
}

/// Sofia-Sicherheitsprofile (Kapitel 5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SofiaProfile {
    Local,
    Public,
    Trunk,
}

impl SofiaProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            SofiaProfile::Local => "local",
            SofiaProfile::Public => "public",
            SofiaProfile::Trunk => "trunk",
        }
    }

    pub fn context(self) -> &'static str {
        match self {
            SofiaProfile::Local => "extelio_local",
            SofiaProfile::Public => "extelio_public",
            SofiaProfile::Trunk => "extelio_trunk",
        }
    }

    pub fn all() -> [SofiaProfile; 3] {
        [
            SofiaProfile::Local,
            SofiaProfile::Public,
            SofiaProfile::Trunk,
        ]
    }
}

/// Security-Modi gemaess Kapitel 5.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityProfile {
    Strict,
    Compatible,
    Legacy,
}

impl SecurityProfile {
    pub fn parse(s: &str) -> Self {
        match s {
            "compatible" => SecurityProfile::Compatible,
            "legacy" => SecurityProfile::Legacy,
            _ => SecurityProfile::Strict,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SecurityProfile::Strict => "strict",
            SecurityProfile::Compatible => "compatible",
            SecurityProfile::Legacy => "legacy",
        }
    }

    /// STRICT: TLS wo anwendbar, SRTP fuer PUBLIC/PUSH, keine Guest Calls,
    /// starke Credentials, Rate Limits, kein stiller Downgrade.
    pub fn requires_srtp(self) -> bool {
        matches!(self, SecurityProfile::Strict)
    }

    pub fn allows_guest(self) -> bool {
        false
    }
}
