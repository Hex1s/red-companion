//! Errors of the content sync, as the UI sees them (`ErrorInfo`): a code to translate and values for it.

use std::collections::BTreeMap;

use rules_core::error::{ErrorInfo, Params};
use rules_core::manifest::ManifestError;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// No answer: no network, DNS, TLS, a timeout.
    #[error("could not reach the server: {0}")]
    Unreachable(String),
    /// An answer the app cannot use (not JSON, not the expected shape).
    #[error("unexpected server response: {0}")]
    BadResponse(String),
    /// The server answered with its error body `{ code, params, message }`.
    #[error("server error {code} (HTTP {status})")]
    Server { code: String, status: u16, params: BTreeMap<String, String> },
    /// The manifest or a collection did not pass the signature or hash check.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The server offers an older content release than one already accepted: a replayed old answer.
    #[error("the server offers release {offered}, older than the accepted {known}")]
    ReleaseRollback { offered: i64, known: i64 },
    /// The new reference data needs a newer app.
    #[error("the content needs app version {needed}, this is {current}")]
    AppTooOld { needed: String, current: String },
}

/// The codes of the server's errors the app can meet while syncing, as static strings the UI
/// translates under `errors.server.*`; anything else is `server.unknown`.
fn server_code(code: &str) -> &'static str {
    match code {
        "server.bad_language" => "server.bad_language",
        "server.unknown_collection" => "server.unknown_collection",
        "server.unknown_pack" => "server.unknown_pack",
        "server.unknown_release" => "server.unknown_release",
        "server.invalid_request" => "server.invalid_request",
        "server.unauthorized" => "server.unauthorized",
        "server.revoked" => "server.revoked",
        "server.rate_limited" => "server.rate_limited",
        "server.unavailable" => "server.unavailable",
        _ => "server.unknown",
    }
}

impl ErrorInfo for SyncError {
    fn code(&self) -> &'static str {
        match self {
            Self::Unreachable(_) => "sync.unreachable",
            Self::BadResponse(_) => "sync.bad_response",
            Self::Server { code, .. } => server_code(code),
            Self::Manifest(e) => e.code(),
            Self::ReleaseRollback { .. } => "sync.release_rollback",
            Self::AppTooOld { .. } => "sync.app_too_old",
        }
    }

    fn params(&self) -> Params {
        match self {
            Self::Server { params, .. } => params.clone(),
            Self::Manifest(e) => e.params(),
            Self::AppTooOld { needed, .. } => Params::from([("needed".to_string(), needed.clone())]),
            Self::Unreachable(_) | Self::BadResponse(_) | Self::ReleaseRollback { .. } => Params::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_codes_pass_through_and_unknown_ones_fall_back() {
        let known = SyncError::Server { code: "server.rate_limited".into(), status: 429, params: BTreeMap::new() };
        assert_eq!(known.code(), "server.rate_limited");
        let other = SyncError::Server { code: "server.something_new".into(), status: 400, params: BTreeMap::new() };
        assert_eq!(other.code(), "server.unknown");
    }

    #[test]
    fn a_manifest_error_keeps_its_own_code() {
        assert_eq!(SyncError::from(ManifestError::UntrustedSignature).code(), "manifest.untrusted_signature");
    }
}
