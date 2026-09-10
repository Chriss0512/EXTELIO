//! Identity-Domaene (Kapitel 6.5).

use serde::{Deserialize, Serialize};

/// Rollen-Templates gemaess Kapitel 6.5.
pub const ROLE_SYSTEMADMIN: &str = "systemadministrator";
pub const ROLE_TELEPHONY_ADMIN: &str = "telefonieadministrator";
pub const ROLE_SUPERVISOR: &str = "supervisor";
pub const ROLE_USER: &str = "benutzer";
pub const ROLE_READONLY: &str = "nur_lesen";

/// Scope-Hierarchie: Organization -> Site -> Department -> Object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeType {
    Organization,
    Site,
    Department,
    Object,
}

impl ScopeType {
    pub fn as_str(self) -> &'static str {
        match self {
            ScopeType::Organization => "organization",
            ScopeType::Site => "site",
            ScopeType::Department => "department",
            ScopeType::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role_id: String,
    pub role_key: String,
    pub scope_type: String,
    pub scope_value: Option<String>,
    pub totp_enabled: bool,
    pub status: String,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

/// Benutzername-Regeln. Bewusst eng, damit SIP-, Directory- und
/// Dateisystemkontexte keine Sonderbehandlung brauchen.
pub fn validate_username(u: &str) -> crate::Result<()> {
    if u.len() < 3 || u.len() > 64 {
        return Err(crate::Error::Validation(
            "Benutzername muss 3 bis 64 Zeichen lang sein".into(),
        ));
    }
    if !u
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(crate::Error::Validation(
            "Benutzername erlaubt nur a-z, A-Z, 0-9, Punkt, Bindestrich und Unterstrich".into(),
        ));
    }
    if !u
        .chars()
        .next()
        .map(|c| c.is_ascii_alphanumeric())
        .unwrap_or(false)
    {
        return Err(crate::Error::Validation(
            "Benutzername muss mit einem Buchstaben oder einer Ziffer beginnen".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_rules() {
        validate_username("admin").unwrap();
        validate_username("c.faust").unwrap();
        assert!(validate_username("ab").is_err());
        assert!(validate_username(".admin").is_err());
        assert!(validate_username("admin;drop").is_err());
    }
}
