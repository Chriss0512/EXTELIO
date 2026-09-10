//! Identifikatoren. Kapitel 8.9: UUIDv7.

use uuid::Uuid;

/// Neue zeitsortierte ID (UUIDv7).
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// Praefigierte ID fuer bessere Lesbarkeit in Logs und Audit.
pub fn new_prefixed(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7().simple())
}

pub fn is_valid(id: &str) -> bool {
    let raw = id.split_once('_').map(|(_, r)| r).unwrap_or(id);
    Uuid::parse_str(raw).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sortable_and_valid() {
        let a = new_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = new_id();
        assert!(is_valid(&a) && is_valid(&b));
        assert!(a < b, "UUIDv7 ist zeitsortiert");
        assert!(is_valid(&new_prefixed("ext")));
    }
}
