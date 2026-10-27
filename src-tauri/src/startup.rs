//! Start-up failures: the log file, the panic hook and the message shown when the database will not open.
//!
//! The window does not exist yet when the database fails, so the message cannot come from the i18n files;
//! it is the only text in the backend that is chosen by the system language instead.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use storage::{BACKUP_DIR, StorageError};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

const LOG_FILE: &str = "red-companion.log";

/// Appends one line (Unix seconds first) to the log in `dir`. Never fails: a log must not break the app.
pub fn log_line(dir: &Path, text: &str) {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let _ = std::fs::create_dir_all(dir);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(dir.join(LOG_FILE)) {
        let _ = writeln!(file, "[{secs}] {text}");
    }
}

/// Release builds abort on a panic and have no console, so the panic goes to the log first.
pub fn install_panic_hook(dir: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log_line(&dir, &format!("panic: {info}"));
        previous(info);
    }));
}

fn system_is_russian() -> bool {
    sys_locale::get_locale().is_some_and(|locale| locale.to_lowercase().starts_with("ru"))
}

/// Tells the user what happened to the data file and what to do; the developer details stay in the log.
pub fn show_database_error(app: &AppHandle, error: &StorageError, db: &Path, log_dir: &Path) {
    app.dialog()
        .message(database_error_text(system_is_russian(), error, db, log_dir))
        .title("Red Companion")
        .kind(MessageDialogKind::Error)
        .blocking_show();
}

fn database_error_text(russian: bool, error: &StorageError, db: &Path, log_dir: &Path) -> String {
    let backups = db.parent().unwrap_or_else(|| Path::new(".")).join(BACKUP_DIR);
    let (db, backups, log) = (db.display(), backups.display(), log_dir.display());
    match (error, russian) {
        (StorageError::SchemaTooNew { .. }, true) => format!(
            "Данные созданы более новой версией Red Companion, эта версия их не откроет.\n\n\
             Обновите приложение до последней версии. Файл не изменён:\n{db}"
        ),
        (StorageError::SchemaTooNew { .. }, false) => format!(
            "The data was created by a newer version of Red Companion and this version cannot open it.\n\n\
             Update the app to the latest version. The file is untouched:\n{db}"
        ),
        (StorageError::Backup(_), true) => format!(
            "Не удалось сделать копию данных перед обновлением, поэтому обновление не выполнено и данные не тронуты.\n\n\
             Освободите место на диске и запустите приложение снова.\n\nФайл данных:\n{db}\n\nПодробности в журнале:\n{log}"
        ),
        (StorageError::Backup(_), false) => format!(
            "Could not copy your data before updating, so the update was not done and the data is untouched.\n\n\
             Free some disk space and start the app again.\n\nData file:\n{db}\n\nDetails in the log:\n{log}"
        ),
        (_, true) => format!(
            "Не удалось открыть данные приложения.\n\n\
             Не удаляйте файл данных, сначала сделайте его копию, затем запустите приложение снова. \
             Если ошибка повторится, замените файл копией из папки с резервными копиями (их делает приложение перед обновлением).\n\n\
             Файл данных:\n{db}\n\nРезервные копии:\n{backups}\n\nПодробности в журнале:\n{log}"
        ),
        (_, false) => format!(
            "Could not open the app data.\n\n\
             Do not delete the data file: copy it first, then start the app again. \
             If the error repeats, replace the file with a copy from the backups folder (the app makes one before every update).\n\n\
             Data file:\n{db}\n\nBackups:\n{backups}\n\nDetails in the log:\n{log}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(russian: bool, error: &StorageError) -> String {
        database_error_text(russian, error, Path::new("data/red-companion.db"), Path::new("logs"))
    }

    #[test]
    fn every_failure_names_the_data_file_in_both_languages() {
        let errors = [
            StorageError::SchemaTooNew { found: 9, supported: 5 },
            StorageError::Backup("disk full".into()),
            unreadable(),
        ];
        for error in &errors {
            for russian in [true, false] {
                assert!(text(russian, error).contains("red-companion.db"), "{error}");
            }
        }
    }

    #[test]
    fn an_unreadable_file_points_at_the_backups_and_keeps_developer_text_out() {
        let error = unreadable();
        for russian in [true, false] {
            let shown = text(russian, &error);
            assert!(shown.contains(BACKUP_DIR), "{shown}");
            assert!(!shown.contains(&error.to_string()), "{shown}");
        }
    }

    #[test]
    fn the_log_gets_one_line_per_call() {
        let dir = tempfile::tempdir().unwrap();
        log_line(dir.path(), "first");
        log_line(dir.path(), "second");
        let written = std::fs::read_to_string(dir.path().join(LOG_FILE)).unwrap();
        assert_eq!(written.lines().count(), 2);
        assert!(written.lines().next().unwrap().ends_with("] first"));
    }

    /// The error SQLite gives when the file cannot be opened at all.
    fn unreadable() -> StorageError {
        storage::Storage::open(Path::new("no-such-dir/red-companion.db")).err().unwrap()
    }
}
