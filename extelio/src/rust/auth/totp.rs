//! TOTP nach RFC 6238 (Kapitel 6.3): 6 Ziffern, 30 s, +/-1 Fenster.

use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;
use subtle::ConstantTimeEq;

use crate::{Error, Result};

pub const DIGITS: u32 = 6;
pub const PERIOD: u64 = 30;
pub const WINDOW: i64 = 1;

/// Erzeugt ein neues Base32-Secret mit 160 Bit Entropie.
pub fn generate_secret() -> String {
    let mut raw = [0u8; 20];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    BASE32_NOPAD.encode(&raw)
}

fn decode(secret: &str) -> Result<Vec<u8>> {
    BASE32_NOPAD
        .decode(secret.trim().replace(' ', "").to_uppercase().as_bytes())
        .map_err(|_| Error::Validation("TOTP-Secret ist kein gueltiges Base32".into()))
}

fn code_at(secret: &[u8], counter: u64) -> Result<String> {
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(secret)
        .map_err(|_| Error::Internal("HMAC-Schluessel ungueltig".into()))?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();

    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let bin = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    let modulo = 10u32.pow(DIGITS);
    Ok(format!("{:0width$}", bin % modulo, width = DIGITS as usize))
}

/// Aktueller Code zu einem Zeitpunkt (Unix-Sekunden).
pub fn code(secret: &str, unix_seconds: u64) -> Result<String> {
    code_at(&decode(secret)?, unix_seconds / PERIOD)
}

/// Prueft einen Code inklusive +/-1 Zeitfenster in konstanter Zeit.
pub fn verify(secret: &str, input: &str, unix_seconds: u64) -> Result<bool> {
    let input = input.trim().replace(' ', "");
    if input.len() != DIGITS as usize || !input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }
    let key = decode(secret)?;
    let counter = (unix_seconds / PERIOD) as i64;
    let mut ok = false;
    for delta in -WINDOW..=WINDOW {
        let c = (counter + delta).max(0) as u64;
        let candidate = code_at(&key, c)?;
        ok |= bool::from(candidate.as_bytes().ct_eq(input.as_bytes()));
    }
    Ok(ok)
}

/// otpauth-URI fuer Authenticator-Apps. Enthaelt das Secret und darf nur
/// waehrend des Enrollments an den Client gehen.
pub fn provisioning_uri(secret: &str, account: &str, issuer: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        urlencode(issuer),
        urlencode(account),
        secret,
        urlencode(issuer),
        DIGITS,
        PERIOD
    )
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".into(),
            other => other
                .to_string()
                .bytes()
                .map(|b| format!("%{b:02X}"))
                .collect::<String>(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 Appendix B, Secret "12345678901234567890" (ASCII) als Base32.
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn matches_rfc6238_vectors() {
        assert_eq!(code(RFC_SECRET, 59).unwrap(), "287082");
        assert_eq!(code(RFC_SECRET, 1111111109).unwrap(), "081804");
        assert_eq!(code(RFC_SECRET, 1234567890).unwrap(), "005924");
    }

    #[test]
    fn accepts_neighbouring_windows_only() {
        let now = 1111111109u64;
        let c = code(RFC_SECRET, now).unwrap();
        assert!(verify(RFC_SECRET, &c, now).unwrap());
        assert!(verify(RFC_SECRET, &c, now + PERIOD).unwrap(), "+1 Fenster");
        assert!(
            verify(RFC_SECRET, &c, now.saturating_sub(PERIOD)).unwrap(),
            "-1 Fenster"
        );
        assert!(!verify(RFC_SECRET, &c, now + 3 * PERIOD).unwrap());
        assert!(!verify(RFC_SECRET, "000000x", now).unwrap());
    }

    #[test]
    fn generated_secrets_work() {
        let s = generate_secret();
        let c = code(&s, 1700000000).unwrap();
        assert_eq!(c.len(), 6);
        assert!(verify(&s, &c, 1700000000).unwrap());
        assert!(
            provisioning_uri(&s, "admin", "EXTELIO").starts_with("otpauth://totp/EXTELIO:admin")
        );
    }
}
