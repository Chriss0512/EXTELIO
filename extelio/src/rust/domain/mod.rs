//! Fachliches Domain Model (Kapitel 8).
//!
//! Das Domain Model ist die fachliche Source of Truth. FreeSWITCH-Konfiguration
//! wird ausschliesslich aus dem Desired State erzeugt (Kapitel 1).

pub mod identity;
pub mod ids;
pub mod numbers;
pub mod routing;
pub mod scheduling;
pub mod telephony;
