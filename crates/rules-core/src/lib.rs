//! Rules engine for Cyberpunk RED.
//!
//! Pure Rust: must not depend on Tauri so it can be tested alone and
//! later reused in WASM or on the server.

pub mod catalog;
pub mod character;
pub mod damage;
pub mod damage_event;
pub mod data_source;
pub mod dice;
pub mod encounter;
pub mod initiative;
pub mod scene;
pub mod skill_check;
pub mod transfer;
pub mod vitals;

use serde::Serialize;
use ts_rs::TS;

/// Crate version, exposed so the app can show which rules build it uses.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Information about the rules engine build. Also proves the Rust -> TS
/// type pipeline end to end.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RulesInfo {
    pub version: String,
}

pub fn rules_info() -> RulesInfo {
    RulesInfo { version: VERSION.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!rules_info().version.is_empty());
    }
}
