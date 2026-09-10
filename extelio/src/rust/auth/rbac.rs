//! Rollen und Berechtigungen (Kapitel 6.5).
//!
//! Permissions sind Punkt-getrennt und unterstuetzen `*` als Platzhalter,
//! sowohl am Ende (`telephony.*`) als auch als Suffix (`*.read`).

use crate::{Error, Result};

/// Prueft eine konkrete Permission gegen die Liste einer Rolle.
pub fn allows(granted: &[String], required: &str) -> bool {
    granted.iter().any(|g| matches_pattern(g, required))
}

fn matches_pattern(pattern: &str, required: &str) -> bool {
    if pattern == "*" || pattern == required {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return required == prefix || required.starts_with(&format!("{prefix}."));
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return required.ends_with(&format!(".{suffix}"));
    }
    false
}

pub fn require(granted: &[String], required: &str) -> Result<()> {
    if allows(granted, required) {
        Ok(())
    } else {
        Err(Error::Forbidden(format!(
            "Für diese Aktion fehlt die Berechtigung '{required}'."
        )))
    }
}

/// Aktionen, die eine erneute Authentifizierung verlangen (Kapitel 6.6).
pub fn requires_step_up(action: &str) -> bool {
    const CRITICAL: &[&str] = &[
        "user.create",
        "user.delete",
        "role.update",
        "secret.reveal",
        "secret.rotate",
        "kms.rotate",
        "security.mode.change",
        "trunk.credential.update",
        "device.admin_password.reveal",
        "backup.restore",
        "override.hard_error",
        "recording.export",
        "voicemail.admin_access",
    ];
    CRITICAL.contains(&action)
}

/// Aktionen, die Home Assistant ueber den Service Principal ausloesen darf
/// (Kapitel 13, Allowlist). Alles andere ist unzulaessig.
pub fn ha_allowed(action: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "dnd.set",
        "forward.set",
        "daynight.set",
        "queue.login",
        "queue.logout",
        "presence.set",
        "routing_profile.set",
        "announcement.set",
    ];
    ALLOWED.contains(&action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_behave_as_specified() {
        let sysadmin = vec!["*".to_string()];
        assert!(allows(&sysadmin, "user.delete"));

        let telephony: Vec<String> = ["telephony.*", "routing.*", "health.read"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(allows(&telephony, "telephony.extension.create"));
        assert!(allows(&telephony, "health.read"));
        assert!(!allows(&telephony, "user.delete"));

        let readonly = vec!["*.read".to_string()];
        assert!(allows(&readonly, "telephony.read"));
        assert!(!allows(&readonly, "telephony.create"));
    }

    #[test]
    fn step_up_and_ha_allowlist() {
        assert!(requires_step_up("secret.reveal"));
        assert!(!requires_step_up("extension.read"));
        assert!(ha_allowed("dnd.set"));
        assert!(!ha_allowed("user.delete"));
        assert!(!ha_allowed("secret.reveal"));
    }

    #[test]
    fn require_reports_missing_permission() {
        let err = require(&["health.read".into()], "user.delete").unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }
}
