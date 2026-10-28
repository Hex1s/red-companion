//! The signed content manifest as the app checks it (docs/SERVER-SPEC.md, section 5): canonical
//! JSON, the Ed25519 signature over it and the hashes of downloaded collections. Pure functions
//! over `serde_json::Value`, no network and no clock: the sync code feeds them what it fetched.
//!
//! The bytes are the server's (`canonicalize` in `src/lib/content/canonical.ts`): keys sorted
//! bytewise, no whitespace. The shared vectors in `vectors/manifest-vectors.json` pin both sides.

use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("the manifest has no signature")]
    MissingSignature,
    #[error("the manifest signature is not base64 of 64 bytes")]
    MalformedSignature,
    #[error("the manifest signature matches none of the trusted keys")]
    UntrustedSignature,
    #[error("a built-in public key is malformed")]
    BadKey,
    #[error("collection {name:?} does not match the hash in the signed manifest")]
    HashMismatch { name: String },
}

/// Canonical JSON of a value: object keys sorted bytewise, no spaces. `serde_json` objects are
/// sorted maps (the `preserve_order` feature is off), and the data holds integers only, which
/// both sides print the same way.
pub fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Checks the manifest signature: Ed25519 over the canonical JSON of the document without its
/// `signature` field. `trusted_keys` are raw 32-byte public keys in base64; the signature is
/// accepted when any of them verifies it (a spare key lets the primary one be replaced by an
/// ordinary app update).
pub fn verify_manifest(document: &Value, trusted_keys: &[&str]) -> Result<(), ManifestError> {
    let signature = document.get("signature").and_then(Value::as_str).ok_or(ManifestError::MissingSignature)?;
    let signature = STANDARD.decode(signature).map_err(|_| ManifestError::MalformedSignature)?;
    let signature = Signature::from_slice(&signature).map_err(|_| ManifestError::MalformedSignature)?;

    let mut unsigned = document.clone();
    if let Some(object) = unsigned.as_object_mut() {
        object.remove("signature");
    }
    let message = canonical_json(&unsigned);

    for key in trusted_keys {
        let raw = STANDARD.decode(key).map_err(|_| ManifestError::BadKey)?;
        let raw: [u8; 32] = raw.try_into().map_err(|_| ManifestError::BadKey)?;
        let key = VerifyingKey::from_bytes(&raw).map_err(|_| ManifestError::BadKey)?;
        if key.verify_strict(message.as_bytes(), &signature).is_ok() {
            return Ok(());
        }
    }
    Err(ManifestError::UntrustedSignature)
}

/// SHA-256 of the bytes, lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash of a collection inside one language, as the server computes it: SHA-256 of the canonical
/// JSON of the entries ordered by `code` (else `id`, else `uuid`). Taken over the raw JSON the
/// server sent, not over a typed struct, so a field the app does not know cannot change it.
pub fn collection_hash(entries: &[Value]) -> String {
    let key = |entry: &Value| ["code", "id", "uuid"].iter().find_map(|k| entry.get(*k).and_then(Value::as_str)).unwrap_or_default().to_string();
    let mut sorted: Vec<&Value> = entries.iter().collect();
    sorted.sort_by_key(|entry| key(entry));
    let text = serde_json::to_string(&sorted).unwrap_or_default();
    sha256_hex(text.as_bytes())
}

/// Accepts a downloaded collection only when it hashes to what the signed manifest lists.
pub fn check_collection(name: &str, entries: &[Value], expected_hash: &str) -> Result<(), ManifestError> {
    if collection_hash(entries) == expected_hash {
        Ok(())
    } else {
        Err(ManifestError::HashMismatch { name: name.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("../vectors/manifest-vectors.json")).unwrap()
    }

    #[test]
    fn the_servers_manifest_vectors_verify_and_canonicalize_the_same_way() {
        let vectors = vectors();
        let key = vectors["public_key"].as_str().unwrap();
        for case in vectors["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let document = &case["document"];
            let result = verify_manifest(document, &[key]);
            if case["valid"].as_bool().unwrap() {
                assert_eq!(result, Ok(()), "{name} must verify");
                let mut unsigned = document.clone();
                unsigned.as_object_mut().unwrap().remove("signature");
                assert_eq!(canonical_json(&unsigned), case["canonical"].as_str().unwrap(), "{name}: canonical JSON differs from the server's");
            } else {
                assert_eq!(result, Err(ManifestError::UntrustedSignature), "{name} must be rejected");
            }
        }
    }

    #[test]
    fn a_spare_key_is_enough_and_a_foreign_one_is_not() {
        let vectors = vectors();
        let key = vectors["public_key"].as_str().unwrap();
        let other = "/PiKqfpAcIy+7mVCbkr8s8cXoybEOXNYsHlfSi7ozPA=";
        let document = &vectors["cases"][0]["document"];
        assert_eq!(verify_manifest(document, &[other, key]), Ok(()));
        assert_eq!(verify_manifest(document, &[other]), Err(ManifestError::UntrustedSignature));
    }

    #[test]
    fn a_missing_or_broken_signature_and_a_broken_key_are_told_apart() {
        let vectors = vectors();
        let key = vectors["public_key"].as_str().unwrap();
        let mut document = vectors["cases"][0]["document"].clone();
        document["signature"] = Value::String("not base64!".into());
        assert_eq!(verify_manifest(&document, &[key]), Err(ManifestError::MalformedSignature));
        document.as_object_mut().unwrap().remove("signature");
        assert_eq!(verify_manifest(&document, &[key]), Err(ManifestError::MissingSignature));
        let signed = &vectors["cases"][0]["document"];
        assert_eq!(verify_manifest(signed, &["short"]), Err(ManifestError::BadKey));
    }

    #[test]
    fn a_tampered_collection_is_refused() {
        let entries: Vec<Value> = serde_json::from_str(r#"[{"code":"b","n":1},{"code":"a","n":2}]"#).unwrap();
        let hash = collection_hash(&entries);
        assert_eq!(check_collection("x", &entries, &hash), Ok(()));
        let mut tampered = entries.clone();
        tampered[0]["n"] = Value::from(3);
        assert_eq!(check_collection("x", &tampered, &hash), Err(ManifestError::HashMismatch { name: "x".into() }));
        // Order of the entries in the response does not matter.
        let mut reordered = entries;
        reordered.reverse();
        assert_eq!(collection_hash(&reordered), hash);
    }
}
