//! Errors as the UI sees them: a stable `code` and `params` the UI translates, plus the
//! developer `message` (English, from `thiserror`) kept for the technical details.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

/// Implemented by every error enum. `code` is an exhaustive `match` with no `_` arm, so a new
/// variant cannot ship without a code (and a code without a locale key fails a test, see
/// `src-tauri/src/error.rs`).
pub trait ErrorInfo: std::error::Error {
    /// Stable `domain.name` code; the UI looks up the text under `errors.<code>`.
    fn code(&self) -> &'static str;

    /// Values for the `{{placeholders}}` of the text.
    fn params(&self) -> Params {
        Params::new()
    }

    /// The error behind this one (a bad character inside an import file...).
    fn cause_info(&self) -> Option<&dyn ErrorInfo> {
        None
    }
}

pub type Params = BTreeMap<String, String>;

/// Builds `Params` from `key => value` pairs (values are shown with `Display`).
#[macro_export]
macro_rules! params {
    ($($key:literal => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut p = $crate::error::Params::new();
        $(p.insert($key.to_string(), $value.to_string());)*
        p
    }};
}

/// Error sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AppError {
    pub code: String,
    pub params: Params,
    /// Developer text (English), shown only as technical details.
    pub message: String,
    pub cause: Option<Box<AppError>>,
}

impl AppError {
    pub fn new(code: &str, params: Params, message: impl Into<String>) -> Self {
        Self { code: code.to_string(), params, message: message.into(), cause: None }
    }

    /// An error with a code but no parameters.
    pub fn coded(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, Params::new(), message)
    }

    /// Anything that has no code of its own (an OS error, a poisoned lock...).
    pub fn internal(message: impl Into<String>) -> Self {
        Self::coded("internal.unknown", message)
    }

    pub fn from_info(error: &dyn ErrorInfo) -> Self {
        Self {
            code: error.code().to_string(),
            params: error.params(),
            message: error.to_string(),
            cause: error.cause_info().map(|c| Box::new(Self::from_info(c))),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::CharacterError;
    use crate::transfer::TransferError;

    #[test]
    fn a_nested_error_keeps_its_own_code_as_the_cause() {
        let e = TransferError::Invalid { index: 1, name: "Jo".into(), source: CharacterError::EmptyName };
        let app = AppError::from_info(&e);
        assert_eq!(app.code, "transfer.invalid");
        assert_eq!(app.params["index"], "2");
        assert_eq!(app.cause.as_ref().map(|c| c.code.as_str()), Some("character.empty_name"));
    }

    #[test]
    fn a_transparent_variant_reports_the_code_of_the_inner_error() {
        let e = CharacterError::Role(crate::roles::RoleError::UnknownRole("x".into()));
        assert_eq!(AppError::from_info(&e).code, "role.unknown_role");
    }
}
