//! Minimales strukturiertes Logging.
//!
//! Kapitel 14.5: Passwoerter, Tokens, TOTP-Seeds, Private Keys, vollstaendige
//! Authorization-Header und Audioinhalte werden nie geloggt. Rufnummern und
//! IP-Adressen werden maskiert.

use crate::infrastructure::time::now_rfc3339;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

fn threshold() -> Level {
    match std::env::var("EXTELIO_LOG_LEVEL")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "debug" => Level::Debug,
        "warn" => Level::Warn,
        "error" => Level::Error,
        _ => Level::Info,
    }
}

pub fn emit(level: Level, component: &str, message: &str) {
    if level < threshold() {
        return;
    }
    println!(
        "{} {} [{}] {}",
        now_rfc3339(),
        level.label(),
        component,
        message
    );
}

#[macro_export]
macro_rules! log_info {
    ($c:expr, $($a:tt)*) => {
        $crate::infrastructure::log::emit($crate::infrastructure::log::Level::Info, $c, &format!($($a)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($c:expr, $($a:tt)*) => {
        $crate::infrastructure::log::emit($crate::infrastructure::log::Level::Warn, $c, &format!($($a)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($c:expr, $($a:tt)*) => {
        $crate::infrastructure::log::emit($crate::infrastructure::log::Level::Error, $c, &format!($($a)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($c:expr, $($a:tt)*) => {
        $crate::infrastructure::log::emit($crate::infrastructure::log::Level::Debug, $c, &format!($($a)*))
    };
}

/// Maskiert eine Rufnummer fuer Diagnosezwecke: +4951112345 -> +4951****45
pub fn mask_number(n: &str) -> String {
    let chars: Vec<char> = n.chars().collect();
    if chars.len() <= 6 {
        return "*".repeat(chars.len());
    }
    let head: String = chars[..5].iter().collect();
    let tail: String = chars[chars.len() - 2..].iter().collect();
    format!("{head}{}{tail}", "*".repeat(chars.len() - 7))
}

/// Maskiert eine IP-Adresse: 10.10.20.43 -> 10.10.20.x
pub fn mask_ip(ip: &str) -> String {
    if let Some(idx) = ip.rfind('.') {
        return format!("{}.x", &ip[..idx]);
    }
    if let Some(idx) = ip.rfind(':') {
        return format!("{}:x", &ip[..idx]);
    }
    String::from("x")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_numbers_and_ips() {
        assert_eq!(mask_number("+4951112345"), "+4951****45");
        assert_eq!(mask_ip("10.10.20.43"), "10.10.20.x");
        assert_eq!(mask_number("123"), "***");
    }
}
