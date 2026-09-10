//! Passwortsicherheit (Kapitel 6.2).
//!
//! Argon2id, individuelles Salt, versionierte Parameter. Ein hardware-gestuetzter
//! Pepper wird nur verwendet, wenn TPM/HSM verfuegbar ist; einen softwarebasierten
//! Ersatz-Pepper gibt es bewusst nicht.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::{Error, Result};

/// Kalibrierte Parameter. Aenderungen erhoehen die Profil-ID, damit vorhandene
/// Hashes weiterhin verifizierbar bleiben und bei Login migriert werden koennen.
pub const PROFILE: &str = "argon2id-v1";
const MEMORY_KIB: u32 = 65536; // 64 MiB
const ITERATIONS: u32 = 3;
const PARALLELISM: u32 = 4;

fn hasher() -> Result<Argon2<'static>> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, PARALLELISM, None)
        .map_err(|e| Error::Internal(format!("Argon2-Parameter ungueltig: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Mindestanforderungen an ein lokales Passwort (OWASP ASVS 5.0, Level 2).
pub fn validate_strength(pw: &str) -> Result<()> {
    let len = pw.chars().count();
    if len < 12 {
        return Err(Error::Validation(
            "Das Passwort muss mindestens 12 Zeichen lang sein.".into(),
        ));
    }
    if len > 256 {
        return Err(Error::Validation(
            "Das Passwort darf hoechstens 256 Zeichen lang sein.".into(),
        ));
    }
    let classes = [
        pw.chars().any(|c| c.is_lowercase()),
        pw.chars().any(|c| c.is_uppercase()),
        pw.chars().any(|c| c.is_ascii_digit()),
        pw.chars().any(|c| !c.is_alphanumeric()),
    ]
    .iter()
    .filter(|x| **x)
    .count();
    if classes < 3 {
        return Err(Error::Validation(
            "Das Passwort muss mindestens drei der vier Zeichenarten enthalten: \
             Kleinbuchstaben, Grossbuchstaben, Ziffern, Sonderzeichen."
                .into(),
        ));
    }
    Ok(())
}

pub fn hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let out = hasher()?
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Internal(format!("Hashing fehlgeschlagen: {e}")))?;
    Ok(out.to_string())
}

pub fn verify(password: &str, stored: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored) else {
        return false;
    };
    hasher()
        .map(|h| h.verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

/// Muss der gespeicherte Hash auf das aktuelle Profil migriert werden?
pub fn needs_rehash(stored: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored) else {
        return true;
    };
    if parsed.algorithm.as_str() != "argon2id" {
        return true;
    }
    let m = parsed.params.get_decimal("m").unwrap_or(0);
    let t = parsed.params.get_decimal("t").unwrap_or(0);
    let p = parsed.params.get_decimal("p").unwrap_or(0);
    m < MEMORY_KIB || t < ITERATIONS || p < PARALLELISM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies() {
        let h = hash("Korrekt-Pferd-Batterie-9").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify("Korrekt-Pferd-Batterie-9", &h));
        assert!(!verify("falsch", &h));
        assert!(!needs_rehash(&h));
    }

    #[test]
    fn salts_are_individual() {
        let a = hash("Korrekt-Pferd-Batterie-9").unwrap();
        let b = hash("Korrekt-Pferd-Batterie-9").unwrap();
        assert_ne!(a, b, "Kapitel 6.2: individual salt");
    }

    #[test]
    fn strength_rules() {
        assert!(validate_strength("kurz").is_err());
        assert!(validate_strength("alleskleinbuchstaben").is_err());
        validate_strength("Korrekt-Pferd-9").unwrap();
    }

    #[test]
    fn detects_weak_legacy_hashes() {
        let weak = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(8, 1, 1, None).unwrap(),
        )
        .hash_password(b"x", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
        assert!(needs_rehash(&weak));
        assert!(needs_rehash("nonsense"));
    }
}
