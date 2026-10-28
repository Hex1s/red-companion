//! Commands of the content sync (RP-7): what the app has, what the server offers, the mode and the
//! languages, and the download itself. The network and the checks live in the `content-sync` crate;
//! this file holds the state, takes the database lock only around reads and writes (never across a
//! network call) and turns the results into what the UI shows.

use std::collections::BTreeMap;

use content_sync::{HttpServer, InstallInfo, MANIFEST_PUBLIC_KEYS, Selection, apply_outcome, apply_registration, load_input};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use storage::Storage;
use tauri::{AppHandle, State};

use crate::error::CmdError;
use crate::{AppStorage, ContentLang};

const MODE_KEY: &str = "content.mode";
const LANGS_KEY: &str = "content.langs";
/// Data on request: a collection the app needs and does not have is fetched when it is opened.
const MODE_ONLINE: &str = "online";
/// Only what was downloaded on purpose.
const MODE_LOCAL: &str = "local";
const LANGS: [&str; 2] = ["ru", "en"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackStatus {
    code: String,
    kind: String,
    name: BTreeMap<String, String>,
    summary: BTreeMap<String, String>,
    enabled: bool,
    /// Entries per collection, as the server counts them.
    counts: BTreeMap<String, i64>,
    /// Languages the server has this pack in.
    languages: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pack: String,
    collection: String,
    lang: String,
    release: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentStatus {
    /// The installation has a token (it registered at least once).
    registered: bool,
    mode: String,
    langs: Vec<String>,
    /// The newest content release accepted from the server.
    release: Option<i64>,
    /// Packs of the last accepted manifest; empty until the first successful check.
    packs: Vec<PackStatus>,
    cached: Vec<CacheStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRequest {
    /// Languages to download; empty only refreshes the manifest.
    langs: Vec<String>,
    /// `None`: every pack.
    packs: Option<Vec<String>>,
    /// `None`: every collection.
    collections: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    release: i64,
    downloaded: usize,
    unchanged: usize,
}

/// One sync run at a time: the interface asks for several collections at once, and parallel runs on a
/// fresh installation would each register it. A run waits here for the one before it.
#[derive(Default)]
pub struct SyncLock(tauri::async_runtime::Mutex<()>);

fn lock<'a>(state: &'a State<'_, AppStorage>) -> Result<std::sync::MutexGuard<'a, Storage>, CmdError> {
    state.0.lock().map_err(|_| CmdError::locked("storage"))
}

fn lang_code(state: &State<'_, ContentLang>) -> Result<&'static str, CmdError> {
    let lang = *state.0.lock().map_err(|_| CmdError::locked("content language"))?;
    Ok(match lang {
        rules_core::data_source::Lang::Ru => "ru",
        rules_core::data_source::Lang::En => "en",
    })
}

/// The server to talk to. A debug build may point at another one with `RC_SERVER_URL`.
fn server_url() -> String {
    if cfg!(debug_assertions)
        && let Ok(url) = std::env::var("RC_SERVER_URL")
        && !url.is_empty()
    {
        return url;
    }
    content_sync::DEFAULT_SERVER.to_string()
}

fn mode_of(storage: &Storage) -> Result<String, CmdError> {
    Ok(storage.setting(MODE_KEY)?.filter(|m| m == MODE_LOCAL).unwrap_or_else(|| MODE_ONLINE.to_string()))
}

fn langs_of(storage: &Storage, fallback: &str) -> Result<Vec<String>, CmdError> {
    let stored: Vec<String> = storage
        .setting(LANGS_KEY)?
        .map(|s| s.split(',').filter(|l| LANGS.contains(l)).map(String::from).collect())
        .unwrap_or_default();
    Ok(if stored.is_empty() { vec![fallback.to_string()] } else { stored })
}

/// Settings the UI may store: a known mode and one or two known languages, no repeats.
fn valid_settings(mode: &str, langs: &[String]) -> bool {
    let mut seen = langs.to_vec();
    seen.sort();
    seen.dedup();
    (mode == MODE_ONLINE || mode == MODE_LOCAL) && !langs.is_empty() && seen.len() == langs.len() && langs.iter().all(|l| LANGS.contains(&l.as_str()))
}

fn texts(value: Option<&Value>) -> BTreeMap<String, String> {
    value.and_then(Value::as_object).map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()).unwrap_or_default()
}

/// The packs of an accepted manifest, as the UI lists them.
fn packs_of(manifest: &Value, enabled: impl Fn(&str) -> bool) -> Vec<PackStatus> {
    let Some(packs) = manifest.pointer("/content/packs").and_then(Value::as_object) else { return Vec::new() };
    let mut list: Vec<(i64, PackStatus)> = packs
        .iter()
        .map(|(code, pack)| {
            let counts = pack.get("counts").and_then(Value::as_object).map(|o| o.iter().filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n))).collect()).unwrap_or_default();
            let languages = pack.get("collections").and_then(Value::as_object).map(|o| o.keys().cloned().collect()).unwrap_or_default();
            let status = PackStatus {
                code: code.clone(),
                kind: pack.get("kind").and_then(Value::as_str).unwrap_or("dlc").to_string(),
                name: texts(pack.get("name")),
                summary: texts(pack.get("summary")),
                enabled: enabled(code),
                counts,
                languages,
            };
            (pack.get("position").and_then(Value::as_i64).unwrap_or(0), status)
        })
        .collect();
    list.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.code.cmp(&b.1.code)));
    list.into_iter().map(|(_, p)| p).collect()
}

#[tauri::command]
pub fn content_status(storage: State<'_, AppStorage>, lang: State<'_, ContentLang>) -> Result<ContentStatus, CmdError> {
    let fallback = lang_code(&lang)?;
    let storage = lock(&storage)?;
    let manifest: Option<Value> = storage.setting(content_sync::sync::keys::MANIFEST)?.and_then(|s| serde_json::from_str(&s).ok());
    let packs = match &manifest {
        Some(m) => {
            let mut switches = BTreeMap::new();
            for code in m.pointer("/content/packs").and_then(Value::as_object).map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
                let on = storage.pack_enabled(&code)?;
                switches.insert(code, on);
            }
            packs_of(m, |code| switches.get(code).copied().unwrap_or(true))
        }
        None => Vec::new(),
    };
    Ok(ContentStatus {
        registered: storage.setting(content_sync::sync::keys::TOKEN)?.is_some(),
        mode: mode_of(&storage)?,
        langs: langs_of(&storage, fallback)?,
        release: storage.setting(content_sync::sync::keys::RELEASE)?.and_then(|s| s.parse().ok()),
        packs,
        cached: storage.cache_index()?.into_iter().map(|e| CacheStatus { pack: e.pack, collection: e.collection, lang: e.lang, release: e.release }).collect(),
    })
}

#[tauri::command]
pub fn set_content_settings(storage: State<'_, AppStorage>, mode: String, langs: Vec<String>) -> Result<(), CmdError> {
    if !valid_settings(&mode, &langs) {
        return Err(CmdError::invalid_content_settings());
    }
    let storage = lock(&storage)?;
    storage.set_setting(MODE_KEY, &mode)?;
    storage.set_setting(LANGS_KEY, &langs.join(","))?;
    Ok(())
}

#[tauri::command]
pub fn set_pack_enabled(storage: State<'_, AppStorage>, code: String, enabled: bool) -> Result<(), CmdError> {
    Ok(lock(&storage)?.set_pack_enabled(&code, enabled)?)
}

/// Deletes downloaded collections of a pack, narrowed to one collection and / or one language.
#[tauri::command]
pub fn delete_cached_content(storage: State<'_, AppStorage>, pack: String, collection: Option<String>, lang: Option<String>) -> Result<usize, CmdError> {
    Ok(lock(&storage)?.delete_cached(&pack, collection.as_deref(), lang.as_deref())?)
}

/// One sync run: state read under the lock, then the network without it, then the writes.
async fn run_sync(app: &AppHandle, storage: &State<'_, AppStorage>, ui_lang: &str, selection: Selection) -> Result<SyncReport, CmdError> {
    let install = InstallInfo { app_version: app.package_info().version.to_string(), os: std::env::consts::OS.to_string(), lang: ui_lang.to_string() };
    let input = {
        let guard = lock(storage)?;
        load_input(&guard, install, selection)?
    };
    let server = HttpServer::new(&server_url())?;
    let run = content_sync::run(&server, MANIFEST_PUBLIC_KEYS, input).await;

    let storage = lock(storage)?;
    // A token the server just issued is kept even when the rest of the run failed.
    if let Some(registration) = &run.registration {
        apply_registration(&storage, registration)?;
    }
    let outcome = run.result?;
    apply_outcome(&storage, &outcome)?;
    Ok(SyncReport { release: outcome.release, downloaded: outcome.fetched.len(), unchanged: outcome.unchanged })
}

/// Checks the server and downloads what the request names (see `SyncRequest`).
#[tauri::command]
pub async fn sync_content(
    app: AppHandle,
    storage: State<'_, AppStorage>,
    lang: State<'_, ContentLang>,
    queue: State<'_, SyncLock>,
    request: SyncRequest,
) -> Result<SyncReport, CmdError> {
    if !request.langs.iter().all(|l| LANGS.contains(&l.as_str())) {
        return Err(CmdError::invalid_content_settings());
    }
    let ui_lang = lang_code(&lang)?;
    let selection = Selection { langs: request.langs, packs: request.packs, collections: request.collections };
    let _turn = queue.0.lock().await;
    run_sync(&app, &storage, ui_lang, selection).await
}

/// Data on request: downloads a collection in the current content language when the mode is
/// `online` and nothing of it is cached yet. Returns how many collections were downloaded.
#[tauri::command]
pub async fn ensure_content(
    app: AppHandle,
    storage: State<'_, AppStorage>,
    lang: State<'_, ContentLang>,
    queue: State<'_, SyncLock>,
    collection: String,
) -> Result<usize, CmdError> {
    let ui_lang = lang_code(&lang)?;
    let _turn = queue.0.lock().await;
    // Checked after the wait: the run before this one may have brought the collection already.
    {
        let storage = lock(&storage)?;
        if mode_of(&storage)? == MODE_LOCAL || !storage.cached_for(&collection, ui_lang)?.is_empty() {
            return Ok(0);
        }
    }
    let selection = Selection { langs: vec![ui_lang.to_string()], packs: None, collections: Some(vec![collection]) };
    Ok(run_sync(&app, &storage, ui_lang, selection).await?.downloaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn langs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn settings_need_a_known_mode_and_one_or_two_known_languages() {
        assert!(valid_settings("online", &langs(&["ru"])));
        assert!(valid_settings("local", &langs(&["ru", "en"])));
        assert!(!valid_settings("offline", &langs(&["ru"])));
        assert!(!valid_settings("online", &[]));
        assert!(!valid_settings("online", &langs(&["de"])));
        assert!(!valid_settings("online", &langs(&["ru", "ru"])));
    }

    #[test]
    fn mode_and_languages_default_when_nothing_is_stored_or_stored_values_are_junk() {
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(mode_of(&s).unwrap(), "online");
        assert_eq!(langs_of(&s, "ru").unwrap(), vec!["ru"]);
        s.set_setting(MODE_KEY, "weird").unwrap();
        s.set_setting(LANGS_KEY, "xx,en").unwrap();
        assert_eq!(mode_of(&s).unwrap(), "online");
        assert_eq!(langs_of(&s, "ru").unwrap(), vec!["en"]);
        s.set_setting(MODE_KEY, "local").unwrap();
        assert_eq!(mode_of(&s).unwrap(), "local");
    }

    #[test]
    fn the_packs_of_a_manifest_are_listed_in_order_with_their_switch() {
        let manifest = serde_json::json!({ "content": { "packs": {
            "dlc1": { "kind": "dlc", "position": 2, "name": { "ru": "Доп", "en": "Extra" }, "summary": { "ru": "", "en": "" },
                      "counts": { "weapons": 3 }, "collections": { "ru": {}, "en": {} } },
            "core": { "kind": "core", "position": 1, "name": { "ru": "Основа", "en": "Core" }, "summary": { "ru": "", "en": "" },
                      "counts": { "weapons": 29, "armor": 8 }, "collections": { "ru": {}, "en": {} } }
        } } });
        let packs = packs_of(&manifest, |code| code != "dlc1");
        assert_eq!(packs.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(), vec!["core", "dlc1"]);
        assert_eq!((packs[0].enabled, packs[1].enabled), (true, false));
        assert_eq!((packs[0].counts["armor"], packs[0].name["ru"].as_str(), packs[0].languages.len()), (8, "Основа", 2));
        assert!(packs_of(&serde_json::json!({}), |_| true).is_empty());
    }
}
