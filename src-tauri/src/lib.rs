use std::sync::Mutex;

use rules_core::RulesInfo;
use storage::Storage;
use tauri::Manager;

/// User database, opened once at startup in the app data directory.
pub struct AppStorage(pub Mutex<Storage>);

#[tauri::command]
fn rules_info() -> RulesInfo {
    rules_core::rules_info()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let storage = Storage::open(&dir.join("red-companion.db"))?;
            app.manage(AppStorage(Mutex::new(storage)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![rules_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
