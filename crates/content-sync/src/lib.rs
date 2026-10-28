//! Client of the Red Companion content server (docs/SERVER-SPEC.md, sections 5, 6 and 11). No Tauri,
//! so a second app (the players' client) can use it as it is: registration, the signed manifest,
//! and the download of collections into the local cache (`storage::content`).

pub mod error;
pub mod server;
pub mod source;
pub mod sync;

pub use error::SyncError;
pub use source::{CacheReader, CachedSource};
pub use server::{ContentServer, DEFAULT_SERVER, HttpServer, InstallInfo, Registration};
pub use sync::{MANIFEST_PUBLIC_KEYS, Outcome, Run, Selection, SyncInput, apply_outcome, apply_registration, load_input, run};
