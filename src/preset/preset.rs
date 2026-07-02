// path: src/preset/preset.rs

use crate::kernel::preset_id::PresetId;
use crate::preset::preset_metadata::PresetMetadata;

/// A versioned snapshot of one patch's complete configuration (voice,
/// sample, mod, mixer, sends, inserts).
///
/// `Preset` itself carries identity, descriptive metadata, and the format
/// version the underlying payload was written at. It does not perform
/// migration itself — that is the responsibility of the preset codec
/// (`crate::preset::preset_codec`), which reads `version` to decide how to
/// interpret serialized bytes before handing back a `Preset` stamped at
/// `CURRENT_VERSION`. Keeping migration out of this type follows the
/// project's versioning invariant: presets serialize with an explicit
/// version and older versions are migrated on load, never silently
/// reinterpreted in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    id: PresetId,
    meta: PresetMetadata,
    version: u32,
}

impl Preset {
    /// The current on-disk/in-memory format version newly authored presets
    /// are stamped with. Serialized presets carrying a smaller `version`
    /// must be migrated by the codec before their payload can be trusted.
    pub const CURRENT_VERSION: u32 = 1;

    /// Construct a preset at the current format version.
    pub fn new(id: PresetId, meta: PresetMetadata) -> Self {
        Self {
            id,
            meta,
            version: Self::CURRENT_VERSION,
        }
    }

    /// Reconstruct a preset at an explicit format version — used by the
    /// codec when decoding a serialized preset (whatever version it was
    /// written at) or after migrating one forward.
    pub fn at_version(id: PresetId, meta: PresetMetadata, version: u32) -> Self {
        Self { id, meta, version }
    }

    /// The preset's identity. `PresetId` is a small `Copy` type, so this
    /// returns by value rather than by reference.
    pub fn id(&self) -> PresetId {
        self.id
    }

    pub fn meta(&self) -> &PresetMetadata {
        &self.meta
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    /// Whether this preset was serialized at an older format version than
    /// `CURRENT_VERSION`. A `true` result means the codec must migrate the
    /// underlying payload before it can be interpreted.
    pub fn needs_migration(&self) -> bool {
        self.version < Self::CURRENT_VERSION
    }

    /// Produce a copy of this preset stamped with a new format version, e.g.
    /// after the codec has migrated its underlying payload forward. Identity
    /// and metadata are preserved; only the version changes.
    pub fn migrated_to(&self, version: u32) -> Self {
        Self {
            id: self.id,
            meta: self.meta.clone(),
            version,
        }
    }

    /// Produce a copy of this preset with replaced metadata. Identity and
    /// version are preserved.
    pub fn with_meta(&self, meta: PresetMetadata) -> Self {
        Self {
            id: self.id,
            meta,
            version: self.version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_id() -> PresetId {
        PresetId::new(1)
    }

    fn sample_meta(name: &str) -> PresetMetadata {
        PresetMetadata::try_new(
            name,
            "cresten",
            "Lead",
            "a test preset",
            Vec::<String>::new(),
        )
        .expect("valid metadata should construct")
    }

    #[test]
    fn new_preset_is_stamped_at_current_version() {
        let preset = Preset::new(sample_id(), sample_meta("Init"));
        assert_eq!(preset.version(), Preset::CURRENT_VERSION);
        assert!(!preset.needs_migration());
    }

    #[test]
    fn older_version_reports_needs_migration() {
        let preset = Preset::at_version(sample_id(), sample_meta("Legacy"), 0);
        assert!(preset.needs_migration());
    }

    #[test]
    fn migrated_to_preserves_identity_and_metadata() {
        let preset = Preset::at_version(sample_id(), sample_meta("Legacy"), 0);
        let migrated = preset.migrated_to(Preset::CURRENT_VERSION);
        assert_eq!(migrated.id(), preset.id());
        assert_eq!(migrated.meta(), preset.meta());
        assert_eq!(migrated.version(), Preset::CURRENT_VERSION);
        assert!(!migrated.needs_migration());
    }

    #[test]
    fn with_meta_preserves_id_and_version() {
        let preset = Preset::new(sample_id(), sample_meta("Init"));
        let renamed = preset.with_meta(sample_meta("Renamed"));
        assert_eq!(renamed.id(), preset.id());
        assert_eq!(renamed.version(), preset.version());
        assert_eq!(renamed.meta().name(), "Renamed");
    }

    #[test]
    fn current_version_preset_never_needs_migration() {
        let preset = Preset::new(sample_id(), sample_meta("Fresh"));
        assert!(!preset.needs_migration());
    }
}
