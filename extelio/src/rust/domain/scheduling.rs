//! Scheduling-Domaene (Kapitel 8.5).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyWindow {
    /// 1 = Montag ... 7 = Sonntag (ISO-8601)
    pub weekday: u8,
    /// "HH:MM"
    pub from: String,
    /// "HH:MM"
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateException {
    pub date: String,
    pub open: bool,
    pub from: Option<String>,
    pub to: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub timezone: String,
    #[serde(default)]
    pub weekly: Vec<WeeklyWindow>,
    #[serde(default)]
    pub holidays: Vec<DateException>,
    #[serde(default)]
    pub exceptions: Vec<DateException>,
    #[serde(default = "default_priority")]
    pub priority: i64,
}

fn default_priority() -> i64 {
    100
}

pub fn validate_hhmm(v: &str) -> crate::Result<(u8, u8)> {
    let (h, m) = v
        .split_once(':')
        .ok_or_else(|| crate::Error::Validation(format!("Zeit '{v}' erwartet Format HH:MM")))?;
    let h: u8 = h
        .parse()
        .map_err(|_| crate::Error::Validation(format!("Stunde '{h}' ungueltig")))?;
    let m: u8 = m
        .parse()
        .map_err(|_| crate::Error::Validation(format!("Minute '{m}' ungueltig")))?;
    if h > 23 || m > 59 {
        return Err(crate::Error::Validation(format!(
            "Zeit '{v}' liegt ausserhalb des Tages"
        )));
    }
    Ok((h, m))
}

impl Schedule {
    pub fn validate(&self) -> crate::Result<()> {
        for w in &self.weekly {
            if w.weekday < 1 || w.weekday > 7 {
                return Err(crate::Error::Validation(
                    "Wochentag muss zwischen 1 (Montag) und 7 (Sonntag) liegen".into(),
                ));
            }
            let from = validate_hhmm(&w.from)?;
            let to = validate_hhmm(&w.to)?;
            if from >= to {
                return Err(crate::Error::Validation(format!(
                    "Zeitfenster {}-{} ist leer oder invertiert",
                    w.from, w.to
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_windows() {
        let s = Schedule {
            id: "s".into(),
            name: "Buero".into(),
            timezone: "Europe/Berlin".into(),
            weekly: vec![WeeklyWindow {
                weekday: 1,
                from: "08:00".into(),
                to: "17:00".into(),
            }],
            holidays: vec![],
            exceptions: vec![],
            priority: 100,
        };
        s.validate().unwrap();

        let bad = Schedule {
            weekly: vec![WeeklyWindow {
                weekday: 9,
                from: "08:00".into(),
                to: "17:00".into(),
            }],
            ..s.clone()
        };
        assert!(bad.validate().is_err());
    }
}
