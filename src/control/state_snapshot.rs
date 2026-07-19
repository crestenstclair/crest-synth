/// Canonical JSON serialization of one accepted control state.
///
/// The state projector owns serialization and must pass the complete,
/// deterministic serde_json output to StateSnapshot::new. The snapshot then
/// provides one stable fingerprint for downstream text and real-time
/// projections without exposing any mutable state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSnapshot {
    json: String,
    hash: String,
}

impl StateSnapshot {
    /// Captures canonical JSON and computes its deterministic content hash.
    pub fn new(json: impl Into<String>) -> Self {
        let json = json.into();
        let hash = hash_json(json.as_bytes());

        Self { json, hash }
    }

    /// Returns the complete canonical JSON document.
    pub fn json(&self) -> &str {
        &self.json
    }

    /// Returns the stable lowercase hexadecimal fingerprint of the JSON.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Consumes the snapshot and returns its canonical JSON document.
    pub fn into_json(self) -> String {
        self.json
    }
}

/// FNV-1a gives the snapshot a stable, platform-independent content identity.
///
/// This is an identity fingerprint, not a security boundary.
fn hash_json(bytes: &[u8]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }

    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::StateSnapshot;

    #[test]
    fn retains_the_complete_canonical_json() {
        let json = r#"{"global":{"masterGainDb":-3.0},"patches":[],"selection":{"patch":0}}"#;
        let snapshot = StateSnapshot::new(json);

        assert_eq!(snapshot.json(), json);
        assert_eq!(snapshot.clone().into_json(), json);
    }

    #[test]
    fn hash_is_deterministic_and_content_sensitive() {
        let first = StateSnapshot::new(r#"{"patches":[]}"#);
        let same = StateSnapshot::new(r#"{"patches":[]}"#);
        let different = StateSnapshot::new(r#"{"patches":[{}]}"#);

        assert_eq!(first.hash(), same.hash());
        assert_ne!(first.hash(), different.hash());
        assert_eq!(first.hash().len(), 16);
        assert!(first.hash().bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn value_equality_covers_json_and_derived_hash() {
        let left = StateSnapshot::new(r#"{"channels":[]}"#);
        let right = StateSnapshot::new(r#"{"channels":[]}"#);

        assert_eq!(left, right);
    }
}
