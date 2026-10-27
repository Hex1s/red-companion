//! Files chosen through the system dialogs. On Android a dialog hands back a `content://` URI rather than a path,
//! so the bytes go through the fs plugin, which opens both (ROADMAP 8.3).

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_fs::{FsExt, OpenOptions};

use crate::error::CmdError;

/// Asks the user for a file and reads it; `None` when the dialog is closed without a choice.
pub async fn pick_and_read(app: &AppHandle) -> Result<Option<String>, CmdError> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || handle.dialog().file().blocking_pick_file())
        .await
        .map_err(|_| CmdError::dialog_failed())?;
    let Some(path) = picked else { return Ok(None) };
    let bytes = app.fs().read(path)?;
    String::from_utf8(bytes).map(Some).map_err(|e| CmdError::from(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
}

/// Asks the user where to save `bytes` under the suggested `name`; false when the dialog is closed without a choice.
pub async fn save_bytes(app: &AppHandle, name: &str, bytes: Vec<u8>) -> Result<bool, CmdError> {
    let handle = app.clone();
    let name = name.to_string();
    let target: Option<FilePath> =
        tauri::async_runtime::spawn_blocking(move || handle.dialog().file().set_file_name(name).blocking_save_file())
            .await
            .map_err(|_| CmdError::dialog_failed())?;
    let Some(target) = target else { return Ok(false) };
    let mut options = OpenOptions::new();
    options.read(false).write(true).truncate(true).create(true);
    let mut file = app.fs().open(target, options)?;
    std::io::Write::write_all(&mut file, &bytes)?;
    Ok(true)
}
