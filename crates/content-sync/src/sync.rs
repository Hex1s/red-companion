//! One sync run, in three phases so the database is never held across a network call:
//! `load_input` (sync, reads the state) → `run` (async, network and checks only) → `apply_*`
//! (sync, writes). `run` always reports a fresh registration even when a later step failed, so the
//! token is not lost with the error.

use std::collections::BTreeMap;

use serde_json::Value;
use rules_core::manifest::{check_collection, verify_manifest};
use storage::{CacheEntry, CachedCollection, Storage, StorageError};

use crate::error::SyncError;
use crate::server::{ContentServer, InstallInfo, Registration};

/// Public keys the manifest signature is checked against. A list so that a key can be replaced by
/// an ordinary app update: ship the new key next to the old one first.
pub const MANIFEST_PUBLIC_KEYS: &[&str] = &["/PiKqfpAcIy+7mVCbkr8s8cXoybEOXNYsHlfSi7ozPA="];

/// Keys in `app_settings` the sync owns.
pub mod keys {
    pub const INSTALLATION_ID: &str = "installation.id";
    pub const TOKEN: &str = "installation.token";
    /// Number of the newest content release accepted; an older one is refused (a replayed answer).
    pub const RELEASE: &str = "content.release";
    /// The last accepted manifest, as sent (to show what is available without the network).
    pub const MANIFEST: &str = "content.manifest";
}

/// What to download. Empty `langs`: only refresh the manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub langs: Vec<String>,
    /// `None`: every pack of the manifest.
    pub packs: Option<Vec<String>>,
    /// `None`: every collection of the chosen packs.
    pub collections: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SyncInput {
    pub install: InstallInfo,
    pub token: Option<String>,
    pub known_release: Option<i64>,
    pub cached: Vec<CacheEntry>,
    pub selection: Selection,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    /// The verified manifest, as sent.
    pub manifest: Value,
    pub release: i64,
    /// Collections that were missing or changed, each checked against its hash in the manifest.
    pub fetched: Vec<CachedCollection>,
    /// Collections already cached under the same hash.
    pub unchanged: usize,
}

#[derive(Debug)]
pub struct Run {
    /// Set when the run registered this installation: store it whatever `result` is.
    pub registration: Option<Registration>,
    pub result: Result<Outcome, SyncError>,
}

pub fn load_input(storage: &Storage, install: InstallInfo, selection: Selection) -> Result<SyncInput, StorageError> {
    Ok(SyncInput {
        install,
        token: storage.setting(keys::TOKEN)?,
        known_release: storage.setting(keys::RELEASE)?.and_then(|s| s.parse().ok()),
        cached: storage.cache_index()?,
        selection,
    })
}

pub fn apply_registration(storage: &Storage, registration: &Registration) -> Result<(), StorageError> {
    storage.set_setting(keys::INSTALLATION_ID, &registration.installation_id)?;
    storage.set_setting(keys::TOKEN, &registration.token)
}

pub fn apply_outcome(storage: &Storage, outcome: &Outcome) -> Result<(), StorageError> {
    storage.put_cached(&outcome.fetched)?;
    storage.set_setting(keys::RELEASE, &outcome.release.to_string())?;
    storage.set_setting(keys::MANIFEST, &outcome.manifest.to_string())
}

pub async fn run<S: ContentServer + Sync>(server: &S, trusted_keys: &[&str], input: SyncInput) -> Run {
    let mut registration = None;
    let result = run_inner(server, trusted_keys, &input, &mut registration).await;
    Run { registration, result }
}

async fn run_inner<S: ContentServer + Sync>(
    server: &S,
    trusted_keys: &[&str],
    input: &SyncInput,
    registration: &mut Option<Registration>,
) -> Result<Outcome, SyncError> {
    let mut token = match &input.token {
        Some(token) => token.clone(),
        None => register(server, &input.install, registration).await?,
    };
    let app_version = &input.install.app_version;
    let manifest = match server.manifest(&token, app_version).await {
        // The token is unknown to the server (a database restored from a backup, a wiped server):
        // register again once instead of failing for good.
        Err(SyncError::Server { code, .. }) if code == "server.unauthorized" && registration.is_none() => {
            token = register(server, &input.install, registration).await?;
            server.manifest(&token, app_version).await?
        }
        other => other?,
    };
    verify_manifest(&manifest, trusted_keys)?;

    let content = ContentManifest::parse(&manifest)?;
    if let Some(known) = input.known_release
        && content.release < known
    {
        return Err(SyncError::ReleaseRollback { offered: content.release, known });
    }
    if older_than(app_version, &content.min_app_version) {
        return Err(SyncError::AppTooOld { needed: content.min_app_version, current: app_version.clone() });
    }

    let cached: BTreeMap<(&str, &str, &str), &str> =
        input.cached.iter().map(|e| ((e.pack.as_str(), e.collection.as_str(), e.lang.as_str()), e.hash.as_str())).collect();
    let wanted = |list: &Option<Vec<String>>, name: &str| list.as_ref().is_none_or(|l| l.iter().any(|x| x == name));

    let mut fetched = Vec::new();
    let mut unchanged = 0;
    for (pack, by_lang) in &content.packs {
        if !wanted(&input.selection.packs, pack) {
            continue;
        }
        for lang in &input.selection.langs {
            let Some(collections) = by_lang.get(lang) else { continue };
            for (collection, hash) in collections {
                if !wanted(&input.selection.collections, collection) {
                    continue;
                }
                if cached.get(&(pack.as_str(), collection.as_str(), lang.as_str())) == Some(&hash.as_str()) {
                    unchanged += 1;
                    continue;
                }
                let body = server.collection(&token, pack, collection, lang).await?;
                let entries = body.get("entries").and_then(Value::as_array).ok_or_else(|| SyncError::BadResponse(format!("{pack}/{collection}: no entries")))?;
                check_collection(&format!("{pack}/{collection}"), entries, hash)?;
                fetched.push(CachedCollection {
                    pack: pack.clone(),
                    collection: collection.clone(),
                    lang: lang.clone(),
                    hash: hash.clone(),
                    release: content.release,
                    body: body.to_string(),
                });
            }
        }
    }
    Ok(Outcome { manifest, release: content.release, fetched, unchanged })
}

async fn register<S: ContentServer + Sync>(server: &S, install: &InstallInfo, slot: &mut Option<Registration>) -> Result<String, SyncError> {
    let registration = server.register(install).await?;
    let token = registration.token.clone();
    *slot = Some(registration);
    Ok(token)
}

/// The parts of the manifest the sync reads: the release and which hash each collection should have.
struct ContentManifest {
    release: i64,
    min_app_version: String,
    /// pack → language → collection → hash.
    packs: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
}

impl ContentManifest {
    fn parse(manifest: &Value) -> Result<Self, SyncError> {
        let bad = |what: &str| SyncError::BadResponse(format!("manifest: {what}"));
        let content = manifest.get("content").ok_or_else(|| bad("no content"))?;
        let release = content.get("release").and_then(Value::as_i64).ok_or_else(|| bad("no release"))?;
        let min_app_version = content.get("min_app_version").and_then(Value::as_str).unwrap_or("0.0.0").to_string();
        let mut packs = BTreeMap::new();
        for (code, pack) in content.get("packs").and_then(Value::as_object).ok_or_else(|| bad("no packs"))? {
            let mut by_lang = BTreeMap::new();
            for (lang, collections) in pack.get("collections").and_then(Value::as_object).ok_or_else(|| bad("no collections"))? {
                let hashes = collections
                    .as_object()
                    .ok_or_else(|| bad("collections of a language"))?
                    .iter()
                    .filter_map(|(name, hash)| hash.as_str().map(|h| (name.clone(), h.to_string())))
                    .collect();
                by_lang.insert(lang.clone(), hashes);
            }
            packs.insert(code.clone(), by_lang);
        }
        Ok(Self { release, min_app_version, packs })
    }
}

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.').map(|p| p.parse::<u64>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// `current < needed`; a version that does not parse never blocks.
fn older_than(current: &str, needed: &str) -> bool {
    matches!((parse_version(current), parse_version(needed)), (Some(c), Some(n)) if c < n)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use base64::{Engine, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer, SigningKey};
    use rules_core::manifest::{ManifestError, canonical_json, collection_hash};
    use serde_json::json;

    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn public(key: &SigningKey) -> String {
        STANDARD.encode(key.verifying_key().to_bytes())
    }

    fn sign(mut doc: Value, key: &SigningKey) -> Value {
        doc.as_object_mut().unwrap().remove("signature");
        let signature = key.sign(canonical_json(&doc).as_bytes());
        doc["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        doc
    }

    fn entries(tag: &str) -> Vec<Value> {
        vec![json!({ "code": "a", "name": format!("{tag}-a") }), json!({ "code": "b", "name": format!("{tag}-b") })]
    }

    /// A signed manifest of one pack `core` with `weapons` and `armor` in `ru` only.
    fn manifest(key: &SigningKey, release: i64, min_app: &str, weapons: &str, armor: &str) -> Value {
        sign(
            json!({
                "schema_version": 1,
                "issued_at": "2026-10-28T00:00:00Z",
                "content": {
                    "release": release,
                    "min_app_version": min_app,
                    "packs": { "core": { "collections": { "ru": {
                        "weapons": collection_hash(&entries(weapons)),
                        "armor": collection_hash(&entries(armor)),
                    } } } }
                }
            }),
            key,
        )
    }

    #[derive(Default)]
    struct Fake {
        manifest: Mutex<Value>,
        /// Collections served under a different content than the manifest hash: (collection, tag).
        served: Mutex<BTreeMap<String, String>>,
        registrations: Mutex<u32>,
        /// Tokens the server accepts.
        accepted: Mutex<Vec<String>>,
        fetched: Mutex<Vec<String>>,
    }

    impl Fake {
        fn new(manifest: Value) -> Self {
            let fake = Fake { manifest: Mutex::new(manifest), ..Default::default() };
            fake.served.lock().unwrap().extend([("weapons".to_string(), "w".to_string()), ("armor".to_string(), "r".to_string())]);
            fake
        }
    }

    impl ContentServer for Fake {
        async fn register(&self, _info: &InstallInfo) -> Result<Registration, SyncError> {
            let mut n = self.registrations.lock().unwrap();
            *n += 1;
            let token = format!("rcs_{n}");
            self.accepted.lock().unwrap().push(token.clone());
            Ok(Registration { installation_id: format!("id-{n}"), token })
        }

        async fn manifest(&self, token: &str, _app_version: &str) -> Result<Value, SyncError> {
            if !self.accepted.lock().unwrap().iter().any(|t| t == token) {
                return Err(SyncError::Server { code: "server.unauthorized".into(), status: 401, params: BTreeMap::new() });
            }
            Ok(self.manifest.lock().unwrap().clone())
        }

        async fn collection(&self, _token: &str, pack: &str, collection: &str, lang: &str) -> Result<Value, SyncError> {
            self.fetched.lock().unwrap().push(format!("{pack}/{collection}/{lang}"));
            let tag = self.served.lock().unwrap()[collection].clone();
            Ok(json!({ "name": collection, "version": 1, "entries": entries(&tag) }))
        }
    }

    fn input(token: Option<&str>, langs: &[&str]) -> SyncInput {
        SyncInput {
            install: InstallInfo { app_version: "0.9.4".into(), os: "windows".into(), lang: "ru".into() },
            token: token.map(String::from),
            known_release: None,
            cached: Vec::new(),
            selection: Selection { langs: langs.iter().map(|l| l.to_string()).collect(), ..Default::default() },
        }
    }

    #[tokio::test]
    async fn a_first_run_registers_checks_the_signature_and_downloads_everything_chosen() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 3, "0.1.0", "w", "r"));
        let run = run(&server, &[&public(&k)], input(None, &["ru"])).await;
        assert_eq!(run.registration.as_ref().unwrap().token, "rcs_1");
        let outcome = run.result.unwrap();
        assert_eq!((outcome.release, outcome.fetched.len(), outcome.unchanged), (3, 2, 0));
        assert!(outcome.fetched.iter().all(|c| c.release == 3 && c.lang == "ru" && c.pack == "core"));
    }

    #[tokio::test]
    async fn collections_already_cached_under_the_same_hash_are_not_downloaded_again() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 3, "0.1.0", "w", "r"));
        let mut input = input(Some("rcs_known"), &["ru"]);
        server.accepted.lock().unwrap().push("rcs_known".into());
        input.cached = vec![CacheEntry { pack: "core".into(), collection: "weapons".into(), lang: "ru".into(), hash: collection_hash(&entries("w")), release: 2 }];
        let outcome = run(&server, &[&public(&k)], input).await.result.unwrap();
        assert_eq!((outcome.fetched.len(), outcome.unchanged), (1, 1));
        assert_eq!(*server.fetched.lock().unwrap(), vec!["core/armor/ru".to_string()]);
    }

    #[tokio::test]
    async fn a_collection_that_does_not_match_its_signed_hash_is_refused_and_nothing_is_kept() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 3, "0.1.0", "w", "r"));
        server.served.lock().unwrap().insert("armor".into(), "tampered".into());
        let result = run(&server, &[&public(&k)], input(None, &["ru"])).await.result;
        assert!(matches!(result, Err(SyncError::Manifest(ManifestError::HashMismatch { .. }))), "{result:?}");
    }

    #[tokio::test]
    async fn a_manifest_signed_by_another_key_is_refused() {
        let server = Fake::new(manifest(&key(2), 3, "0.1.0", "w", "r"));
        let result = run(&server, &[&public(&key(1))], input(None, &["ru"])).await.result;
        assert!(matches!(result, Err(SyncError::Manifest(ManifestError::UntrustedSignature))), "{result:?}");
    }

    #[tokio::test]
    async fn a_spare_key_in_the_list_is_enough() {
        let (old, new) = (key(1), key(2));
        let server = Fake::new(manifest(&new, 3, "0.1.0", "w", "r"));
        let outcome = run(&server, &[&public(&old), &public(&new)], input(None, &["ru"])).await.result.unwrap();
        assert_eq!(outcome.fetched.len(), 2);
    }

    #[tokio::test]
    async fn an_older_release_than_the_accepted_one_is_a_rollback() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 4, "0.1.0", "w", "r"));
        let mut input = input(None, &["ru"]);
        input.known_release = Some(5);
        let result = run(&server, &[&public(&k)], input).await.result;
        assert!(matches!(result, Err(SyncError::ReleaseRollback { offered: 4, known: 5 })), "{result:?}");
    }

    #[tokio::test]
    async fn content_for_a_newer_app_is_not_downloaded() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 4, "1.0.0", "w", "r"));
        let result = run(&server, &[&public(&k)], input(None, &["ru"])).await.result;
        assert!(matches!(result, Err(SyncError::AppTooOld { .. })), "{result:?}");
        assert!(server.fetched.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_unknown_token_registers_once_and_retries_and_the_new_token_is_reported() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 3, "0.1.0", "w", "r"));
        let run = run(&server, &[&public(&k)], input(Some("rcs_stale"), &["ru"])).await;
        assert_eq!(run.registration.as_ref().unwrap().token, "rcs_1");
        assert_eq!(run.result.unwrap().fetched.len(), 2);
        assert_eq!(*server.registrations.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn the_registration_is_reported_even_when_the_run_fails_later() {
        let server = Fake::new(manifest(&key(2), 3, "0.1.0", "w", "r"));
        let run = run(&server, &[&public(&key(1))], input(None, &["ru"])).await;
        assert!(run.result.is_err());
        assert_eq!(run.registration.unwrap().token, "rcs_1", "the token must not be lost with the error");
    }

    #[tokio::test]
    async fn the_selection_narrows_packs_collections_and_languages() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 3, "0.1.0", "w", "r"));
        let mut only_armor = input(None, &["ru"]);
        only_armor.selection.collections = Some(vec!["armor".into()]);
        assert_eq!(run(&server, &[&public(&k)], only_armor).await.result.unwrap().fetched.len(), 1);
        let mut other_pack = input(None, &["ru"]);
        other_pack.selection.packs = Some(vec!["dlc1".into()]);
        assert_eq!(run(&server, &[&public(&k)], other_pack).await.result.unwrap().fetched.len(), 0);
        // A language the manifest does not list, or none at all, only refreshes the manifest.
        assert_eq!(run(&server, &[&public(&k)], input(None, &["en"])).await.result.unwrap().fetched.len(), 0);
        assert_eq!(run(&server, &[&public(&k)], input(None, &[])).await.result.unwrap().fetched.len(), 0);
    }

    #[tokio::test]
    async fn what_a_run_found_is_stored_and_read_back_as_the_next_input() {
        let k = key(1);
        let server = Fake::new(manifest(&k, 3, "0.1.0", "w", "r"));
        let storage = Storage::open_in_memory().unwrap();
        let first = load_input(&storage, input(None, &["ru"]).install, input(None, &["ru"]).selection).unwrap();
        assert_eq!((first.token.clone(), first.known_release, first.cached.len()), (None, None, 0));

        let run = run(&server, &[&public(&k)], first).await;
        apply_registration(&storage, run.registration.as_ref().unwrap()).unwrap();
        let outcome = run.result.unwrap();
        apply_outcome(&storage, &outcome).unwrap();

        let second = load_input(&storage, input(None, &["ru"]).install, input(None, &["ru"]).selection).unwrap();
        assert_eq!((second.token.as_deref(), second.known_release, second.cached.len()), (Some("rcs_1"), Some(3), 2));
        // Nothing left to download the second time.
        let again = run_second(&server, &k, second).await;
        assert_eq!((again.fetched.len(), again.unchanged), (0, 2));
    }

    async fn run_second(server: &Fake, k: &SigningKey, input: SyncInput) -> Outcome {
        run(server, &[&public(k)], input).await.result.unwrap()
    }

    #[test]
    fn versions_compare_and_an_unparseable_one_never_blocks() {
        assert!(older_than("0.9.4", "1.0.0"));
        assert!(older_than("0.9.4", "0.10.0"));
        assert!(!older_than("1.0.0", "1.0.0"));
        assert!(!older_than("1.0.1", "1.0.0"));
        assert!(!older_than("dev", "1.0.0"));
        assert!(!older_than("0.9.4", ""));
    }
}
