//! Android has no in-app updater plugin; these commands only keep the command list the same on every platform.

use tauri::AppHandle;

use crate::error::CmdError;

#[derive(Default)]
pub struct PendingUpdate;

#[tauri::command]
pub async fn check_update() -> Result<Option<()>, CmdError> {
    Ok(None)
}

#[tauri::command]
pub async fn install_update() -> Result<(), CmdError> {
    Err(CmdError::update_not_checked())
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
