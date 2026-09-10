//! Zeitfunktionen. Alle persistierten Zeitstempel sind RFC 3339 in UTC.

use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn now_rfc3339() -> String {
    to_rfc3339(now())
}

pub fn to_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

pub fn parse_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

pub fn plus_minutes(t: OffsetDateTime, m: i64) -> OffsetDateTime {
    t + Duration::minutes(m)
}

pub fn plus_hours(t: OffsetDateTime, h: i64) -> OffsetDateTime {
    t + Duration::hours(h)
}

pub fn plus_seconds(t: OffsetDateTime, s: i64) -> OffsetDateTime {
    t + Duration::seconds(s)
}

/// Unix-Sekunden, u.a. fuer TOTP (RFC 6238).
pub fn unix_seconds() -> u64 {
    now().unix_timestamp().max(0) as u64
}

pub fn is_past(rfc3339: &str) -> bool {
    match parse_rfc3339(rfc3339) {
        Some(t) => t <= now(),
        None => true,
    }
}
