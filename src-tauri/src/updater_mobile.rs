//! Android has no updater plugin. The same endpoint answers an APK instead of a signed package: this module asks it,
//! downloads the file with progress, checks its `sha256` and hands it to the system installer (ROADMAP 8.6).
//! Android itself refuses an APK signed with another key, so the hash only guards the transfer.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::error::CmdError;

/// Event with the download progress of the update (same name as on desktop).
const PROGRESS_EVENT: &str = "update-progress";

/// Kotlin class in the app project (`ApkInstallerPlugin.kt`).
const PLUGIN_PACKAGE: &str = "app.red_companion";
const PLUGIN_CLASS: &str = "ApkInstallerPlugin";

/// What the server says about the newest APK.
#[derive(Clone, Deserialize)]
struct ApkUpdate {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    url: String,
    sha256: String,
}

/// The update found by the last check, waiting for the install command.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<ApkUpdate>>);

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

/// The Kotlin side: the "install unknown apps" permission and the installer intent.
pub struct ApkInstaller<R: Runtime>(PluginHandle<R>);

#[derive(Deserialize)]
struct Allowed {
    allowed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallArgs {
    path: String,
}

/// Registers the Kotlin plugin and keeps its handle.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apk-installer")
        .setup(|app, api| {
            let handle = api.register_android_plugin(PLUGIN_PACKAGE, PLUGIN_CLASS)?;
            app.manage(ApkInstaller(handle));
            Ok(())
        })
        .build()
}

fn unavailable(error: impl std::fmt::Display) -> CmdError {
    CmdError::update_unavailable(error.to_string())
}

fn install_failed(error: impl std::fmt::Display) -> CmdError {
    CmdError::update_install_failed(error.to_string())
}

/// The update endpoint of `tauri.conf.json` for Android (`{{target}}` = android, `{{arch}}` = universal).
fn endpoint(app: &AppHandle) -> Result<String, CmdError> {
    let config = app.config().plugins.0.get("updater").ok_or_else(|| unavailable("no updater config"))?;
    let template = config
        .get("endpoints")
        .and_then(|e| e.get(0))
        .and_then(|e| e.as_str())
        .ok_or_else(|| unavailable("no updater endpoint"))?;
    Ok(template
        .replace("{{target}}", "android")
        .replace("{{arch}}", "universal")
        .replace("{{current_version}}", &app.package_info().version.to_string()))
}

fn client(app: &AppHandle) -> Result<reqwest::Client, CmdError> {
    reqwest::Client::builder()
        .user_agent(format!("RedCompanion/{}", app.package_info().version))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(unavailable)
}

/// Asks the server for a newer version. `None` = this is the latest one.
#[tauri::command]
pub async fn check_update(app: AppHandle, pending: State<'_, PendingUpdate>) -> Result<Option<UpdateInfo>, CmdError> {
    let response = client(&app)?
        .get(endpoint(&app)?)
        .header("X-App-Version", app.package_info().version.to_string())
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(unavailable)?;
    let found = if response.status() == reqwest::StatusCode::NO_CONTENT {
        None
    } else {
        Some(response.error_for_status().map_err(unavailable)?.json::<ApkUpdate>().await.map_err(unavailable)?)
    };
    let info = found.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.notes.clone() });
    *pending.0.lock().map_err(|_| CmdError::locked("update"))? = found;
    Ok(info)
}

/// Where downloaded APKs wait for the installer (inside the cache folder the file provider shares).
fn updates_dir(app: &AppHandle) -> Result<PathBuf, CmdError> {
    let dir = app.path().app_cache_dir().map_err(install_failed)?.join("updates");
    if dir.exists() {
        // Earlier downloads are of no use any more.
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Downloads the APK with `update-progress` events and checks its hash; returns the path.
async fn download(app: &AppHandle, update: &ApkUpdate) -> Result<PathBuf, CmdError> {
    let path = updates_dir(app)?.join(format!("Red-Companion-{}.apk", update.version));
    let mut response = client(app)?.get(&update.url).send().await.and_then(|r| r.error_for_status()).map_err(unavailable)?;
    let total = response.content_length();
    let mut file = std::fs::File::create(&path)?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(unavailable)? {
        hasher.update(&chunk);
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
    }
    file.flush()?;
    let actual: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if !actual.eq_ignore_ascii_case(update.sha256.trim()) {
        let _ = std::fs::remove_file(&path);
        return Err(CmdError::update_bad_checksum());
    }
    Ok(path)
}

/// Downloads the update found by `check_update`, checks it and opens the system installer. Returns false: the
/// app does not restart by itself, the system asks the user and restarts it after the install.
#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    installer: State<'_, ApkInstaller<tauri::Wry>>,
) -> Result<bool, CmdError> {
    let update = pending
        .0
        .lock()
        .map_err(|_| CmdError::locked("update"))?
        .clone()
        .ok_or_else(CmdError::update_not_checked)?;
    // Ask for the permission before the download: the user should not wait for a file that cannot be installed.
    let allowed: Allowed = installer.0.run_mobile_plugin("canInstall", ()).map_err(install_failed)?;
    if !allowed.allowed {
        let _: serde_json::Value = installer.0.run_mobile_plugin("openInstallSettings", ()).map_err(install_failed)?;
        return Err(CmdError::update_install_permission());
    }
    let path = download(&app, &update).await?;
    let _: serde_json::Value = installer
        .0
        .run_mobile_plugin("install", InstallArgs { path: path.display().to_string() })
        .map_err(install_failed)?;
    Ok(false)
}

/// The system installer restarts the app itself; nothing to do here.
#[tauri::command]
pub fn restart_app() {}
