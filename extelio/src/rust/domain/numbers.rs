//! Rufnummern (Kapitel 8.2).
//!
//! Extern: canonical | display | source representation | type | country.
//! Canonical bevorzugt E.164, interne Extensions sind eine eigene Klasse.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumberType {
    External,
    Internal,
    Emergency,
    Service,
}

impl NumberType {
    pub fn as_str(self) -> &'static str {
        match self {
            NumberType::External => "external",
            NumberType::Internal => "internal",
            NumberType::Emergency => "emergency",
            NumberType::Service => "service",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNumber {
    pub canonical: String,
    pub display: String,
    pub source_repr: String,
    pub number_type: String,
    pub country: Option<String>,
}

/// Normalisiert eine externe Rufnummer nach E.164.
///
/// `default_country` ist der zweistellige ISO-Code (z. B. "DE") fuer Nummern
/// in nationaler Schreibweise.
pub fn parse_external(input: &str, default_country: &str) -> Result<ParsedNumber> {
    let source = input.trim().to_string();
    if source.is_empty() {
        return Err(Error::Validation("Rufnummer ist leer".into()));
    }

    let digits: String = source.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(Error::Validation("Rufnummer enthaelt keine Ziffern".into()));
    }

    let cc = country_code(default_country)?;

    let canonical = if source.starts_with('+') {
        format!("+{digits}")
    } else if source.starts_with("00") {
        format!("+{}", &digits[2..])
    } else if source.starts_with('0') {
        format!("+{}{}", cc, &digits[1..])
    } else {
        format!("+{cc}{digits}")
    };

    if canonical.len() < 8 || canonical.len() > 16 {
        return Err(Error::Validation(format!(
            "E.164-Nummer {canonical} hat eine unplausible Laenge"
        )));
    }

    let country = country_from_e164(&canonical);
    Ok(ParsedNumber {
        display: pretty(&canonical),
        canonical,
        source_repr: source,
        number_type: NumberType::External.as_str().into(),
        country,
    })
}

/// Interne Extensions sind eine eigene Klasse und werden nicht E.164-normalisiert.
pub fn parse_extension(input: &str) -> Result<String> {
    let n = input.trim();
    if n.len() < 2 || n.len() > 8 {
        return Err(Error::Validation(
            "Nebenstelle muss 2 bis 8 Ziffern lang sein".into(),
        ));
    }
    if !n.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::Validation(
            "Nebenstelle darf nur Ziffern enthalten".into(),
        ));
    }
    Ok(n.to_string())
}

fn country_code(iso: &str) -> Result<&'static str> {
    Ok(match iso.to_uppercase().as_str() {
        "DE" => "49",
        "AT" => "43",
        "CH" => "41",
        "NL" => "31",
        "FR" => "33",
        "IT" => "39",
        "GB" | "UK" => "44",
        "US" | "CA" => "1",
        other => {
            return Err(Error::Validation(format!(
                "Unbekanntes Land '{other}'. Bitte Nummer in E.164-Schreibweise angeben."
            )))
        }
    })
}

fn country_from_e164(e164: &str) -> Option<String> {
    let d = e164.trim_start_matches('+');
    for (code, iso) in [
        ("49", "DE"),
        ("43", "AT"),
        ("41", "CH"),
        ("31", "NL"),
        ("33", "FR"),
        ("39", "IT"),
        ("44", "GB"),
        ("1", "US"),
    ] {
        if d.starts_with(code) {
            return Some(iso.to_string());
        }
    }
    None
}

fn pretty(e164: &str) -> String {
    let d = e164.trim_start_matches('+');
    if d.len() > 5 {
        format!("+{} {}", &d[..2], &d[2..])
    } else {
        e164.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_german_numbers() {
        let a = parse_external("0511 123456", "DE").unwrap();
        assert_eq!(a.canonical, "+49511123456");
        assert_eq!(a.country.as_deref(), Some("DE"));
        assert_eq!(a.source_repr, "0511 123456");

        assert_eq!(
            parse_external("+49 511 123456", "DE").unwrap().canonical,
            "+49511123456"
        );
        assert_eq!(
            parse_external("0049511123456", "DE").unwrap().canonical,
            "+49511123456"
        );
    }

    #[test]
    fn rejects_garbage_and_short_numbers() {
        assert!(parse_external("abc", "DE").is_err());
        assert!(parse_external("12", "DE").is_err());
        assert!(parse_extension("1").is_err());
        assert_eq!(parse_extension("201").unwrap(), "201");
    }
}
