use crate::control::{
    AppEvent, AppState, SessionReplacementPayload, StateProjectionError, StateProjector,
};
use crate::kernel::{MidiChannel, PatchId};
use crate::mixer::bus_id::BusId;
use crate::mixer::bus_return::BusReturnBank;
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mixer_state::MixerState;
use crate::mixer::patch_output::PatchOutput;
use crate::real_time::{GraphPreparationError, GraphRevision, PreparedGraph, PreparedGraphBuilder};
use crate::synth::effect_slot_id::{EffectSlotIndex, MAX_EFFECT_SLOTS};
use crate::synth::{
    CapabilityRegistry, EffectCapabilityRegistry, EffectPreparer, InstrumentConfig,
    InstrumentPreparer, Patch, PostEffectConfig, VoiceEnvelope,
};
use serde::{Deserialize, Serialize};

pub const SAVED_SESSION_VERSION: u32 = 2;

/// Versioned control-side session state. Runtime and interaction state are
/// absent by construction: no focus/modal/browser/request/preview, decoded
/// PCM, prepared graph, device value, or absolute library root has a field.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSession {
    version: u32,
    patches: Vec<SavedPatch>,
    mixer: MixerState,
    master_gain_db: f32,
    returns: Vec<SavedReturn>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedPatch {
    id: u32,
    name: String,
    instrument: InstrumentConfig,
    channel: u8,
    envelope: VoiceEnvelope,
    output: PatchOutput,
    effects: [Option<PostEffectConfig>; MAX_EFFECT_SLOTS],
    voice_limit: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedReturn {
    effect: Option<PostEffectConfig>,
    return_level: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedSessionV2 {
    version: u32,
    patches: Vec<SavedPatch>,
    mixer: MixerState,
    master_gain_db: f32,
    returns: Vec<SavedReturn>,
}

/// Phase-6 predecessor: the same canonical values before Patch voice limit
/// became explicit persistence. Migration derives the limit from the saved
/// capability policy; it never guesses an asset or capability.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedSessionV1 {
    #[serde(default)]
    version: Option<u32>,
    patches: Vec<SavedPatchV1>,
    mixer: MixerState,
    master_gain_db: f32,
    returns: Vec<SavedReturn>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedPatchV1 {
    id: u32,
    name: String,
    instrument: InstrumentConfig,
    channel: u8,
    envelope: VoiceEnvelope,
    output: PatchOutput,
    effects: [Option<PostEffectConfig>; MAX_EFFECT_SLOTS],
}

impl SavedSession {
    pub fn capture(state: &AppState) -> Self {
        Self {
            version: SAVED_SESSION_VERSION,
            patches: state
                .patches()
                .iter()
                .map(|patch| SavedPatch {
                    id: patch.id().value(),
                    name: patch.name().to_owned(),
                    instrument: patch.instrument_config().clone(),
                    channel: patch.channel().value(),
                    envelope: *patch.envelope(),
                    output: patch.output(),
                    effects: patch.effect_slots().clone(),
                    voice_limit: patch.voice_limit().value(),
                })
                .collect(),
            mixer: *state.mixer(),
            master_gain_db: state.global().master_gain_db(),
            returns: state
                .bus_returns()
                .returns()
                .iter()
                .map(|bus_return| SavedReturn {
                    effect: bus_return.effect().cloned(),
                    return_level: bus_return.return_level(),
                })
                .collect(),
        }
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub fn to_json(&self) -> Result<String, SavedSessionError> {
        serde_json::to_string_pretty(self).map_err(|_| SavedSessionError::Encode)
    }

    pub fn from_json(
        json: &str,
        capabilities: &CapabilityRegistry,
    ) -> Result<Self, SavedSessionError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|_| SavedSessionError::Decode)?;
        let version = value.get("version").and_then(serde_json::Value::as_u64);
        match version {
            Some(version) if version == u64::from(SAVED_SESSION_VERSION) => {
                let decoded: SavedSessionV2 =
                    serde_json::from_value(value).map_err(|_| SavedSessionError::Decode)?;
                if decoded.version != SAVED_SESSION_VERSION {
                    return Err(SavedSessionError::UnsupportedVersion(decoded.version));
                }
                Ok(Self {
                    version: decoded.version,
                    patches: decoded.patches,
                    mixer: decoded.mixer,
                    master_gain_db: decoded.master_gain_db,
                    returns: decoded.returns,
                })
            }
            None | Some(1) => {
                let decoded: SavedSessionV1 =
                    serde_json::from_value(value).map_err(|_| SavedSessionError::Decode)?;
                if decoded.version.is_some_and(|value| value != 1) {
                    return Err(SavedSessionError::UnsupportedVersion(
                        decoded.version.unwrap_or_default(),
                    ));
                }
                let patches = decoded
                    .patches
                    .into_iter()
                    .map(|patch| {
                        let descriptor = capabilities
                            .descriptor_for_config(&patch.instrument)
                            .ok_or(SavedSessionError::InvalidCapability)?;
                        Ok(SavedPatch {
                            id: patch.id,
                            name: patch.name,
                            instrument: patch.instrument,
                            channel: patch.channel,
                            envelope: patch.envelope,
                            output: patch.output,
                            effects: patch.effects,
                            voice_limit: descriptor.voice_policy().polyphony_ceiling(),
                        })
                    })
                    .collect::<Result<Vec<_>, SavedSessionError>>()?;
                Ok(Self {
                    version: SAVED_SESSION_VERSION,
                    patches,
                    mixer: decoded.mixer,
                    master_gain_db: decoded.master_gain_db,
                    returns: decoded.returns,
                })
            }
            Some(version) => Err(SavedSessionError::UnsupportedVersion(
                u32::try_from(version).unwrap_or(u32::MAX),
            )),
        }
    }

    /// Validates and prepares a complete replacement session without exposing
    /// the candidate canonical state separately from its callback-ready graph.
    /// Callers may stage [`PreparedSavedSession::into_replacement`] only after
    /// this returns `Ok`; every failure leaves their active session and graph
    /// untouched.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_restore(
        &self,
        capabilities: CapabilityRegistry,
        effects: EffectCapabilityRegistry,
        instrument_preparers: &[Box<dyn InstrumentPreparer>],
        effect_preparers: &[Box<dyn EffectPreparer>],
        graph_revision: GraphRevision,
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<PreparedSavedSession, SavedSessionRestoreError> {
        self.validate_shape()?;
        let mut capabilities = capabilities;
        for patch in &self.patches {
            let preparer = instrument_preparers
                .iter()
                .find(|preparer| preparer.capability_id() == patch.instrument.capability_id())
                .ok_or(SavedSessionError::InvalidCapability)?;
            if let Some(descriptor) = preparer
                .asset_descriptor(&patch.instrument)
                .map_err(SavedSessionRestoreError::AssetMetadata)?
            {
                capabilities = capabilities
                    .with_asset_descriptor(descriptor)
                    .map_err(|_| SavedSessionError::InvalidCapability)?;
            }
        }
        let state = self.restore_candidate(capabilities, effects, graph_revision)?;
        let parameters = StateProjector::for_graph(graph_revision)
            .project(&state)
            .map_err(SavedSessionRestoreError::Projection)?
            .2;
        let graph = PreparedGraphBuilder::new(state.capabilities(), instrument_preparers)
            .with_effects(state.effects(), effect_preparers)
            .with_returns(state.bus_returns())
            .build(
                graph_revision,
                state.patches(),
                parameters,
                sample_rate,
                max_frames,
            )
            .map_err(SavedSessionRestoreError::Preparation)?;
        Ok(PreparedSavedSession { state, graph })
    }

    /// Reconstructs one private candidate with fresh transient runtime and
    /// interaction state. It is deliberately not public: persistence callers
    /// must pass through [`Self::prepare_restore`] so canonical state cannot be
    /// committed before complete-graph preparation succeeds.
    fn restore_candidate(
        &self,
        capabilities: CapabilityRegistry,
        effects: EffectCapabilityRegistry,
        graph_revision: GraphRevision,
    ) -> Result<AppState, SavedSessionError> {
        self.validate_shape()?;
        let mut patches = Vec::with_capacity(self.patches.len());
        for saved in &self.patches {
            if saved
                .instrument
                .asset_references()
                .iter()
                .any(|assignment| {
                    crate::synth::AssetFileId::new(assignment.reference().locator())
                        .is_ok_and(|asset| asset.is_external())
                })
            {
                return Err(SavedSessionError::InvalidCapability);
            }
            capabilities
                .validate_config(&saved.instrument)
                .map_err(|_| SavedSessionError::InvalidCapability)?;
            let descriptor = capabilities
                .descriptor_for_config(&saved.instrument)
                .ok_or(SavedSessionError::InvalidCapability)?;
            if saved.voice_limit == 0
                || saved.voice_limit > descriptor.voice_policy().polyphony_ceiling()
            {
                return Err(SavedSessionError::InvalidVoiceLimit);
            }
            let mut patch = Patch::new(
                PatchId::new(saved.id).map_err(|_| SavedSessionError::InvalidPatch)?,
                saved.name.clone(),
                saved.instrument.clone(),
                MidiChannel::new(saved.channel).map_err(|_| SavedSessionError::InvalidPatch)?,
                saved.output,
            )
            .with_envelope(saved.envelope)
            .with_voice_limit(saved.voice_limit)
            .map_err(|_| SavedSessionError::InvalidVoiceLimit)?;
            for (index, occupant) in saved.effects.iter().enumerate() {
                if let Some(config) = occupant {
                    effects
                        .validate_config(config)
                        .map_err(|_| SavedSessionError::InvalidEffect)?;
                    patch = patch.with_effect_slot(
                        EffectSlotIndex::new(index)
                            .map_err(|_| SavedSessionError::InvalidEffect)?,
                        config.clone(),
                    );
                }
            }
            patches.push(patch);
        }

        let mut returns = BusReturnBank::default();
        for (index, saved) in self.returns.iter().enumerate() {
            let bus = BusId::new(index as u8).map_err(|_| SavedSessionError::InvalidReturn)?;
            returns
                .set_return_level(bus, saved.return_level)
                .map_err(|_| SavedSessionError::InvalidReturn)?;
            if let Some(config) = saved.effect.as_ref() {
                effects
                    .validate_config(config)
                    .map_err(|_| SavedSessionError::InvalidEffect)?;
                returns
                    .set_return_occupancy(&effects, bus, Some(config.capability_id()))
                    .map_err(|_| SavedSessionError::InvalidReturn)?;
                returns
                    .replace_occupant_values(bus, config.clone())
                    .map_err(|_| SavedSessionError::InvalidReturn)?;
            }
        }

        let limits = patches
            .iter()
            .map(|patch| (patch.id(), patch.voice_limit().value()))
            .collect::<Vec<_>>();
        let mut state = AppState::for_graph_with_effects(
            capabilities,
            effects,
            GlobalParameters::new(self.master_gain_db)
                .map_err(|_| SavedSessionError::InvalidGlobal)?,
            graph_revision,
        )
        .with_initial_mixer(self.mixer)
        .with_initial_returns(returns);
        state
            .apply(AppEvent::InstallPatches(patches))
            .map_err(|_| SavedSessionError::InvalidPatch)?;
        state
            .restore_voice_limits(&limits)
            .map_err(|_| SavedSessionError::InvalidVoiceLimit)?;
        Ok(state)
    }

    /// Bound asset resolution before any preparer can read a saved bank.
    fn validate_shape(&self) -> Result<(), SavedSessionError> {
        if self.version != SAVED_SESSION_VERSION || self.returns.len() != BusId::COUNT {
            return Err(SavedSessionError::InvalidShape);
        }
        if self.patches.len() > crate::kernel::MAX_ACTIVE_PATCHES {
            return Err(SavedSessionError::InvalidPatch);
        }
        Ok(())
    }
}

/// One completely validated persistence replacement. The state and graph are
/// consumed together so application composition can perform one atomic
/// control-side install/handoff rather than publishing an unprepared session.
pub struct PreparedSavedSession {
    state: AppState,
    graph: PreparedGraph,
}

impl PreparedSavedSession {
    pub const fn state(&self) -> &AppState {
        &self.state
    }

    pub const fn graph(&self) -> &PreparedGraph {
        &self.graph
    }

    /// Consumes the private candidate into the only reducer payload allowed to
    /// accompany its exact prepared graph. No mutable candidate `AppState`
    /// escapes the saved-session boundary.
    pub fn into_replacement(self) -> (SessionReplacementPayload, PreparedGraph) {
        let visualizations = self
            .state
            .patches()
            .iter()
            .filter_map(|patch| {
                self.graph
                    .prepared_sample_visualization(patch.id())
                    .cloned()
                    .map(|value| (patch.id(), value))
            })
            .collect();
        let payload = SessionReplacementPayload::from_prepared_state(
            &self.state,
            self.graph.revision(),
            visualizations,
        );
        (payload, self.graph)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SavedSessionRestoreError {
    #[error("saved instrument asset could not be resolved: {0}")]
    AssetMetadata(crate::synth::InstrumentPreparationError),
    #[error("saved session validation failed: {0}")]
    Session(#[from] SavedSessionError),
    #[error("saved session projection failed: {0}")]
    Projection(StateProjectionError),
    #[error("saved session graph preparation failed: {0}")]
    Preparation(GraphPreparationError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SavedSessionError {
    #[error("saved session could not be encoded")]
    Encode,
    #[error("saved session could not be decoded")]
    Decode,
    #[error("unsupported saved session version {0}")]
    UnsupportedVersion(u32),
    #[error("saved session shape is invalid")]
    InvalidShape,
    #[error("saved Patch is invalid")]
    InvalidPatch,
    #[error("saved capability configuration is unavailable or invalid")]
    InvalidCapability,
    #[error("saved effect configuration is unavailable or invalid")]
    InvalidEffect,
    #[error("saved return is invalid")]
    InvalidReturn,
    #[error("saved global parameters are invalid")]
    InvalidGlobal,
    #[error("saved voice limit is invalid for its capability")]
    InvalidVoiceLimit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::sample_capability::{
        SampleCapability, SAMPLE_ASSET_PARAMETER_ID, SAMPLE_PLAYBACK_START_PARAMETER_ID,
    };
    use crate::kernel::midi_message::MidiMessage;
    use crate::kernel::MidiChannel;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::RtPatchParameters;
    use crate::synth::{
        AssetFileId, AssetKind, AssetReference, CapabilityId, InstrumentCapabilityProvider,
        InstrumentPreparationError, ParameterId, ParameterValue, PreparedAssetFootprint,
        PreparedInstrument, PreparedInstrumentError, RackPreparationError, SampleAssetError,
        MAX_SAMPLE_GRAPH_PCM_BYTES,
    };

    fn state() -> AppState {
        let provider = SampleCapability::new(AssetFileId::new("folder/kick.wav").unwrap()).unwrap();
        let registry = CapabilityRegistry::new(vec![provider.descriptor()]).unwrap();
        let descriptor = provider.descriptor();
        let config = provider
            .default_config()
            .unwrap()
            .with_scalar_value(
                &descriptor,
                &ParameterId::new(SAMPLE_PLAYBACK_START_PARAMETER_ID).unwrap(),
                ParameterValue::continuous(0.25).unwrap(),
            )
            .unwrap();
        let patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Saved Sample".to_owned(),
            config,
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        );
        let mut state = AppState::new(registry, GlobalParameters::new(-3.0).unwrap());
        state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
        state
    }

    #[test]
    fn oversized_restore_is_rejected_before_asset_resolution() {
        let active = state();
        let mut saved = SavedSession::capture(&active);
        saved.patches = vec![saved.patches[0].clone(); crate::kernel::MAX_ACTIVE_PATCHES + 1];
        assert!(matches!(
            saved.prepare_restore(
                active.capabilities().clone(),
                active.effects().clone(),
                &[],
                &[],
                GraphRevision::INITIAL,
                48_000.0,
                64,
            ),
            Err(SavedSessionRestoreError::Session(
                SavedSessionError::InvalidPatch
            ))
        ));
    }

    #[test]
    fn saved_assets_cannot_reference_transient_browser_locations() {
        let state = state();
        let document = SavedSession::capture(&state)
            .to_json()
            .unwrap()
            .replace("folder/kick.wav", "@home/Music/kick.wav");
        let saved = SavedSession::from_json(&document, state.capabilities()).unwrap();
        assert!(matches!(
            saved.restore_candidate(
                state.capabilities().clone(),
                state.effects().clone(),
                GraphRevision::INITIAL
            ),
            Err(SavedSessionError::InvalidCapability)
        ));
    }

    #[test]
    fn version_two_round_trip_keeps_relative_asset_and_normalized_values_only() {
        let state = state();
        let saved = SavedSession::capture(&state);
        let json = saved.to_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["version"], SAVED_SESSION_VERSION);
        assert_eq!(
            value["patches"][0]["instrument"]["assetReferences"][0]["reference"]["locator"],
            "folder/kick.wav"
        );
        assert!(!json.contains("fileBrowser"));
        assert!(!json.contains("preview"));
        assert!(!json.contains("engineSelection"));
        assert!(!json.contains("focus"));
        assert!(!json.contains("libraryRoot"));
        assert!(!json.contains("decoded"));

        let decoded_session = SavedSession::from_json(&json, state.capabilities()).unwrap();
        assert_eq!(decoded_session, saved);
        let preparers: Vec<Box<dyn InstrumentPreparer>> = vec![Box::new(
            crate::adapter::sample_preparer::SamplePreparer::new(
                std::sync::Arc::new(crate::testing::DeterministicSampleCatalog::new(
                    [],
                    [(AssetFileId::new("folder/kick.wav").unwrap(), Ok(vec![1]))],
                )),
                std::sync::Arc::new(crate::testing::DeterministicSampleDecoder::new([(
                    AssetFileId::new("folder/kick.wav").unwrap(),
                    Ok(decoded("folder/kick.wav")),
                )])),
            )
            .unwrap(),
        )];
        let prepared = decoded_session
            .prepare_restore(
                state.capabilities().clone(),
                EffectCapabilityRegistry::default(),
                &preparers,
                &[],
                GraphRevision::INITIAL,
                48_000.0,
                64,
            )
            .unwrap();
        let restored = prepared.state();
        assert_eq!(prepared.graph().revision(), GraphRevision::INITIAL);
        assert_eq!(
            restored.patches()[0]
                .instrument_config()
                .asset_reference(&ParameterId::new(SAMPLE_ASSET_PARAMETER_ID).unwrap())
                .unwrap()
                .locator(),
            "folder/kick.wav"
        );
        assert_eq!(
            restored.patches()[0]
                .instrument_config()
                .value(&ParameterId::new(SAMPLE_PLAYBACK_START_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::continuous(0.25).unwrap())
        );
        assert_eq!(
            restored.file_browser().lifecycle(),
            crate::control::SampleAssetLifecycle::Unavailable,
            "restore makes unresolved asset availability explicit and never substitutes"
        );
        let (replacement, graph) = prepared.into_replacement();
        assert_eq!(
            replacement.patch_ids().collect::<Vec<_>>(),
            [PatchId::new(1).unwrap()]
        );
        assert_eq!(replacement.target_graph_revision(), graph.revision());
    }

    #[test]
    fn exact_capacity_session_round_trips_and_prepares_sixteen_created_patches_only() {
        let seed = state();
        let config = seed.patches()[0].instrument_config().clone();
        let mut full = AppState::new(
            seed.capabilities().clone(),
            GlobalParameters::new(-3.0).unwrap(),
        );
        full.apply(AppEvent::InstallPatches(
            (1..=crate::kernel::MAX_ACTIVE_PATCHES as u32)
                .map(|id| {
                    Patch::new(
                        PatchId::new(id).unwrap(),
                        format!("Patch {id}"),
                        config.clone(),
                        MidiChannel::new((id - 1) as u8).unwrap(),
                        PatchOutput::new(
                            crate::mixer::mixer_track_id::MixerTrackId::new((id - 1) as u8)
                                .unwrap(),
                            0.0,
                        )
                        .unwrap(),
                    )
                })
                .collect(),
        ))
        .unwrap();

        let saved = SavedSession::capture(&full);
        let json = saved.to_json().unwrap();
        assert!(!json.contains("trailingEmpty"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap()["patches"]
                .as_array()
                .unwrap()
                .len(),
            crate::kernel::MAX_ACTIVE_PATCHES
        );
        let restored_saved = SavedSession::from_json(&json, full.capabilities()).unwrap();
        assert_eq!(restored_saved, saved);

        let prepared = restored_saved
            .prepare_restore(
                full.capabilities().clone(),
                EffectCapabilityRegistry::default(),
                &sample_preparers(Ok(vec![1]), Ok(decoded("folder/kick.wav"))),
                &[],
                GraphRevision::INITIAL.checked_next().unwrap(),
                48_000.0,
                64,
            )
            .unwrap();
        assert_eq!(
            prepared.state().patches().len(),
            crate::kernel::MAX_ACTIVE_PATCHES
        );
        assert_eq!(
            prepared.graph().initial_parameters().patch_count(),
            crate::kernel::MAX_ACTIVE_PATCHES
        );
        assert_eq!(
            prepared
                .state()
                .patches()
                .iter()
                .map(Patch::id)
                .collect::<Vec<_>>(),
            (1..=crate::kernel::MAX_ACTIVE_PATCHES as u32)
                .map(|id| PatchId::new(id).unwrap())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn version_two_has_exact_top_level_fields_and_no_midi_device_runtime_data() {
        let value = serde_json::to_value(SavedSession::capture(&state())).unwrap();
        let object = value.as_object().unwrap();
        let mut fields = object.keys().map(String::as_str).collect::<Vec<_>>();
        fields.sort_unstable();
        assert_eq!(
            fields,
            ["masterGainDb", "mixer", "patches", "returns", "version"]
        );
        assert_eq!(object["version"], SAVED_SESSION_VERSION);

        fn reject_midi_runtime_fields(value: &serde_json::Value) {
            match value {
                serde_json::Value::Object(object) => {
                    for (key, nested) in object {
                        let normalized = key.to_ascii_lowercase();
                        for forbidden in [
                            "midiinput",
                            "mididevice",
                            "preference",
                            "descriptor",
                            "handle",
                            "connection",
                            "callback",
                            "queue",
                            "timestamp",
                            "observation",
                        ] {
                            assert!(
                                !normalized.contains(forbidden),
                                "SavedSession unexpectedly contains `{key}`"
                            );
                        }
                        reject_midi_runtime_fields(nested);
                    }
                }
                serde_json::Value::Array(values) => {
                    values.iter().for_each(reject_midi_runtime_fields)
                }
                _ => {}
            }
        }
        reject_midi_runtime_fields(&value);
    }

    fn decoded(asset: &str) -> crate::synth::DecodedSample {
        let samples = vec![0.0_f32; 128];
        crate::synth::DecodedSample::new(
            crate::synth::SampleMetadata::new(
                AssetFileId::new(asset).unwrap(),
                256,
                48_000,
                1,
                32,
                crate::synth::SampleEncoding::Float,
                samples.len() as u64,
            )
            .unwrap(),
            samples,
        )
        .unwrap()
    }

    fn sample_preparers(
        read: Result<Vec<u8>, SampleAssetError>,
        decode: Result<crate::synth::DecodedSample, SampleAssetError>,
    ) -> Vec<Box<dyn InstrumentPreparer>> {
        let asset = AssetFileId::new("folder/kick.wav").unwrap();
        vec![Box::new(
            crate::adapter::sample_preparer::SamplePreparer::new(
                std::sync::Arc::new(crate::testing::DeterministicSampleCatalog::new(
                    [],
                    [(asset.clone(), read)],
                )),
                std::sync::Arc::new(crate::testing::DeterministicSampleDecoder::new([(
                    asset, decode,
                )])),
            )
            .unwrap(),
        )]
    }

    struct FootprintPreparer {
        capability_id: CapabilityId,
    }

    impl InstrumentPreparer for FootprintPreparer {
        fn capability_id(&self) -> &CapabilityId {
            &self.capability_id
        }

        fn prepare(
            &self,
            patch: &Patch,
            _sample_rate: f32,
            _max_frames: usize,
        ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
            Ok(Box::new(FootprintInstrument {
                patch_id: patch.id(),
                footprint: PreparedAssetFootprint::new(
                    AssetReference::new(AssetKind::Sample, "folder/kick.wav").unwrap(),
                    1,
                    MAX_SAMPLE_GRAPH_PCM_BYTES + 1,
                ),
            }))
        }
    }

    struct FootprintInstrument {
        patch_id: PatchId,
        footprint: PreparedAssetFootprint,
    }

    impl PreparedInstrument for FootprintInstrument {
        fn patch_id(&self) -> PatchId {
            self.patch_id
        }

        fn dispatch(
            &mut self,
            _message: MidiMessage,
            _parameters: &RtPatchParameters,
        ) -> Result<(), PreparedInstrumentError> {
            Ok(())
        }

        fn render(
            &mut self,
            output: &mut [f32],
            _frame_count: usize,
            _parameters: &RtPatchParameters,
        ) -> Result<(), crate::synth::PreparedInstrumentError> {
            output.fill(0.0);

            Ok(())
        }

        fn all_notes_off(&mut self) {}

        fn prepared_asset_footprint(&self) -> Option<&PreparedAssetFootprint> {
            Some(&self.footprint)
        }
    }

    #[test]
    fn restore_prepares_before_commit_and_keeps_unavailable_invalid_and_over_budget_typed() {
        let active = state();
        let active_json = SavedSession::capture(&active).to_json().unwrap();
        let saved = SavedSession::from_json(&active_json, active.capabilities()).unwrap();

        let ready = sample_preparers(Ok(vec![1]), Ok(decoded("folder/kick.wav")));
        let prepared = saved
            .prepare_restore(
                active.capabilities().clone(),
                EffectCapabilityRegistry::default(),
                &ready,
                &[],
                GraphRevision::INITIAL.checked_next().unwrap(),
                48_000.0,
                64,
            )
            .unwrap();
        assert_eq!(
            prepared.graph().revision(),
            GraphRevision::INITIAL.checked_next().unwrap()
        );
        assert_eq!(
            SavedSession::capture(prepared.state()).to_json().unwrap(),
            active_json
        );

        let before = SavedSession::capture(&active).to_json().unwrap();
        let unavailable = sample_preparers(
            Err(SampleAssetError::Unavailable),
            Err(SampleAssetError::Unavailable),
        );
        assert!(matches!(
            saved.prepare_restore(
                active.capabilities().clone(),
                EffectCapabilityRegistry::default(),
                &unavailable,
                &[],
                GraphRevision::INITIAL.checked_next().unwrap(),
                48_000.0,
                64,
            ),
            Err(SavedSessionRestoreError::Preparation(
                GraphPreparationError::Rack(RackPreparationError::Instrument {
                    source: InstrumentPreparationError::SampleAsset {
                        cause: SampleAssetError::Unavailable,
                        ..
                    },
                    ..
                })
            ))
        ));

        let invalid = sample_preparers(Ok(vec![1]), Err(SampleAssetError::MalformedWave));
        assert!(matches!(
            saved.prepare_restore(
                active.capabilities().clone(),
                EffectCapabilityRegistry::default(),
                &invalid,
                &[],
                GraphRevision::INITIAL.checked_next().unwrap(),
                48_000.0,
                64,
            ),
            Err(SavedSessionRestoreError::Preparation(
                GraphPreparationError::Rack(RackPreparationError::Instrument {
                    source: InstrumentPreparationError::SampleAsset {
                        cause: SampleAssetError::MalformedWave,
                        ..
                    },
                    ..
                })
            ))
        ));

        let over_budget: Vec<Box<dyn InstrumentPreparer>> = vec![Box::new(FootprintPreparer {
            capability_id: active.patches()[0]
                .instrument_config()
                .capability_id()
                .clone(),
        })];
        assert!(matches!(
            saved.prepare_restore(
                active.capabilities().clone(),
                EffectCapabilityRegistry::default(),
                &over_budget,
                &[],
                GraphRevision::INITIAL.checked_next().unwrap(),
                48_000.0,
                64,
            ),
            Err(SavedSessionRestoreError::Preparation(
                GraphPreparationError::Rack(
                    RackPreparationError::PreparedAssetCapacityExceeded { .. }
                )
            ))
        ));
        assert_eq!(
            SavedSession::capture(&active).to_json().unwrap(),
            before,
            "every failed candidate leaves the caller-owned active session unchanged"
        );
    }

    #[test]
    fn version_one_without_runtime_fields_migrates_explicitly() {
        let state = state();
        let mut value = serde_json::to_value(SavedSession::capture(&state)).unwrap();
        value["version"] = serde_json::json!(1);
        value["patches"][0]
            .as_object_mut()
            .unwrap()
            .remove("voiceLimit");
        let migrated = SavedSession::from_json(
            &serde_json::to_string(&value).unwrap(),
            state.capabilities(),
        )
        .unwrap();
        assert_eq!(migrated.version(), SAVED_SESSION_VERSION);
        assert!(migrated.to_json().unwrap().contains("voiceLimit"));
    }
}
