//! App updates: ask the server, check which of the two keys signed the release, download with progress, install.
//!
//! The plugin knows one `pubkey` (the main key, in `tauri.conf.json`). The backup key is built in here so that a
//! lost or replaced main key can be rotated by a normal update: the release is signed with the backup key, and
//! the key id inside the signature tells which public key must verify it.

use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use crate::error::CmdError;

/// Public key of the backup pair (the private half is kept offline). Same format as `pubkey` in the config.
const BACKUP_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDUyRDlGMTBEMkRGQzlCQTIKUldTaW0vd3REZkhaVXVTSSthS090V2JMQ3NJRXBWRkxIUWp2SWJEb0ltYU1GVk1DWkhONDVxazIK";

/// Event with the download progress of the update.
const PROGRESS_EVENT: &str = "update-progress";

/// The update found by the last check, waiting for the install command.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// The 8 key id bytes of a minisign public key or signature: the file text is base64 of
/// `untrusted comment` + newline + base64(`Ed` + key id + payload) + newline + ...
fn key_id(encoded: &str) -> Option<[u8; 8]> {
    let text = String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok()?;
    let line = text.lines().nth(1)?;
    let bytes = STANDARD.decode(line.trim()).ok()?;
    bytes.get(2..10)?.try_into().ok()
}

fn main_pubkey(app: &AppHandle) -> Option<String> {
    let config = app.config().plugins.0.get("updater")?.clone();
    config.get("pubkey")?.as_str().map(String::from)
}

fn build_updater(app: &AppHandle, pubkey: Option<&str>) -> Result<Updater, CmdError> {
    let mut builder = app
        .updater_builder()
        .header("X-App-Version", app.package_info().version.to_string())?;
    if let Some(pubkey) = pubkey {
        builder = builder.pubkey(pubkey);
    }
    Ok(builder.build()?)
}

/// Asks the server for a newer version. `None` = this is the latest one.
#[tauri::command]
pub async fn check_update(app: AppHandle, pending: State<'_, PendingUpdate>) -> Result<Option<UpdateInfo>, CmdError> {
    let mut found = build_updater(&app, None)?.check().await?;
    if let Some(update) = &found {
        let signed_by = key_id(&update.signature);
        let main = main_pubkey(&app);
        let is_main = main.as_deref().and_then(key_id) == signed_by;
        if !is_main {
            if key_id(BACKUP_PUBKEY) != signed_by {
                return Err(CmdError::update_untrusted());
            }
            found = build_updater(&app, Some(BACKUP_PUBKEY))?.check().await?;
        }
    }
    let info = found.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() });
    *pending.0.lock().map_err(|_| CmdError::locked("update"))? = found;
    Ok(info)
}

/// Downloads and installs the update found by `check_update`, reporting `update-progress` events.
/// On Windows the installer may close the app itself before this returns.
#[tauri::command]
pub async fn install_update(app: AppHandle, pending: State<'_, PendingUpdate>) -> Result<(), CmdError> {
    let update = pending
        .0
        .lock()
        .map_err(|_| CmdError::locked("update"))?
        .clone()
        .ok_or_else(CmdError::update_not_checked)?;
    let mut downloaded = 0u64;
    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
            },
            || {},
        )
        .await?;
    Ok(())
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQwOTg5OUFEOEVBMTNEN0UKUldSK1BhR09yWm1ZUUYzVEYwRGhxOXNvTVRWaWxOeGRjK1p3MDFHekEzRlF4Y05ya0lUYmRNTk0K";

    #[test]
    fn key_ids_are_read_from_both_public_keys() {
        // The comment of each file carries the id in big-endian hex; the bytes inside are little-endian.
        assert_eq!(key_id(MAIN_PUBKEY).unwrap(), 0x409899AD8EA13D7Eu64.to_le_bytes());
        assert_eq!(key_id(BACKUP_PUBKEY).unwrap(), 0x52D9F10D2DFC9BA2u64.to_le_bytes());
    }

    #[test]
    fn the_config_holds_the_main_key() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["plugins"]["updater"]["pubkey"], MAIN_PUBKEY);
    }

    #[test]
    fn garbage_has_no_key_id() {
        assert_eq!(key_id("not base64!"), None);
    }
}
