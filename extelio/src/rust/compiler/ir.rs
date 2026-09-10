//! Typisierte Zwischenrepraesentation (Kapitel 9).
//!
//! Der Compiler uebersetzt den Desired State zuerst in dieses IR und erst
//! danach in FreeSWITCH-XML. Damit ist die fachliche Uebersetzung unabhaengig
//! vom Zielformat testbar.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SofiaProfileIr {
    pub name: String,
    pub context: String,
    pub bind_ip: String,
    pub sip_port: u16,
    pub tls_port: u16,
    pub tls_enabled: bool,
    pub srtp_required: bool,
    pub auth_calls: bool,
    pub accept_blind_registration: bool,
    pub inbound_codec_prefs: Vec<String>,
    pub outbound_codec_prefs: Vec<String>,
    pub rtp_start: u16,
    pub rtp_end: u16,
    pub apply_inbound_acl: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryUserIr {
    pub id: String,
    pub extension_number: String,
    pub display_name: String,
    /// Referenz auf ein Secret. Der Klartext steht nie in einer Datei auf /data.
    pub password_secret_id: Option<String>,
    pub voicemail_enabled: bool,
    pub context: String,
    pub effective_caller_id_name: String,
    pub effective_caller_id_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayIr {
    pub id: String,
    pub name: String,
    pub proxy: String,
    pub register: bool,
    pub username: Option<String>,
    pub password_secret_id: Option<String>,
    pub from_user: Option<String>,
    pub from_domain: Option<String>,
    pub transport: String,
    pub retry_seconds: u32,
    pub caller_id_in_from: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionIr {
    Set {
        key: String,
        value: String,
    },
    Bridge {
        target: String,
    },
    Answer,
    Playback {
        file: String,
    },
    Voicemail {
        extension: String,
    },
    Transfer {
        target: String,
        context: String,
    },
    Hangup {
        cause: String,
    },
    RingGroup {
        targets: Vec<String>,
        timeout_s: u32,
        strategy: String,
    },
    Queue {
        name: String,
    },
    Ivr {
        name: String,
    },
    Log {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionIr {
    pub field: String,
    pub expression: String,
    pub actions: Vec<ActionIr>,
    pub anti_actions: Vec<ActionIr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialplanExtensionIr {
    pub name: String,
    pub conditions: Vec<ConditionIr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialplanContextIr {
    pub name: String,
    pub extensions: Vec<DialplanExtensionIr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclIr {
    pub name: String,
    pub default_policy: String,
    pub nodes: Vec<(String, String)>, // (allow|deny, cidr)
}

/// Vollstaendiges IR einer Konfigurationsgeneration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigIr {
    pub domain: String,
    pub profiles: Vec<SofiaProfileIr>,
    pub directory: Vec<DirectoryUserIr>,
    pub gateways: Vec<GatewayIr>,
    pub contexts: Vec<DialplanContextIr>,
    pub acls: Vec<AclIr>,
    pub esl_port: u16,
    pub modules: Vec<String>,
}

/// FreeSWITCH-Modul-Allowlist (Kapitel 22.4). Nur was hier steht, wird geladen.
pub const MODULE_ALLOWLIST: &[&str] = &[
    "mod_sofia",
    "mod_event_socket",
    "mod_xml_curl",
    "mod_commands",
    "mod_dptools",
    "mod_db",
    "mod_hash",
    "mod_loopback",
    "mod_tone_stream",
    "mod_sndfile",
    "mod_voicemail",
    "mod_callcenter",
    "mod_opus",
];

/// Module, die ausdruecklich nicht geladen werden (Kapitel 22.4).
pub const MODULE_DENYLIST: &[&str] = &[
    "mod_xml_rpc",
    "mod_verto",
    "mod_rtmp",
    "mod_lua",
    "mod_python",
    "mod_spidermonkey",
];

pub fn module_allowed(name: &str) -> bool {
    MODULE_ALLOWLIST.contains(&name) && !MODULE_DENYLIST.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_and_denylist_are_disjoint() {
        for m in MODULE_ALLOWLIST {
            assert!(!MODULE_DENYLIST.contains(m), "{m} steht auf beiden Listen");
        }
        assert!(module_allowed("mod_sofia"));
        assert!(!module_allowed("mod_verto"));
        assert!(!module_allowed("mod_unbekannt"));
    }
}
