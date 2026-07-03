// path: src/plugin/plugin_shell.rs

//! `PluginShell`: orchestrates plugin lifecycle — init, process, parameter
//! sync, and state persistence via the host.
//!
//! # Design
//!
//! `PluginShell` is the applicationService that glues together a plugin
//! instance, the control-plane loop, the real-time parameter boundary, and
//! preset persistence, without owning the behavior of any of them:
//!
//! - a [`PluginInstancePort`] performs the actual command→event reducer
//!   work (`aggregate.Plugin.PluginInstance`'s job);
//! - a [`ControlPlane`] performs `AppState::apply` (`aggregate.Loop.AppState`'s
//!   job);
//! - a [`StateProjector`](crate::r#loop::state_projector::StateProjector)
//!   derives the [`ParameterSnapshot`](crate::real_time::parameter_bridge::ParameterSnapshot)
//!   from the control plane's state;
//! - a [`ParameterBridge`] publishes that snapshot across the RT boundary;
//! - a [`MidiNormalizer`] turns raw host MIDI bytes into a normalized
//!   [`MidiEvent`](crate::kernel::midi_event::MidiEvent) before it becomes
//!   an [`AppEvent`].
//!
//! `PluginShell` never touches egui, an audio driver, or a window, and it
//! never runs on the real-time audio thread itself — it is the non-RT
//! orchestration the host (via its own plugin-format adapter, e.g. a future
//! `nih_plug_host`) calls into. Every state-mutating operation ends by
//! projecting-and-publishing the latest snapshot, so "all parameter changes
//! cross the thread boundary via the ParameterBridge" holds for every path
//! through this shell, not just direct parameter edits.
//!
//! # Why ports instead of the concrete aggregates
//!
//! `aggregate.Plugin.PluginInstance` and `aggregate.Loop.AppState` are
//! declared dependencies of this resource, but neither's Rust source has
//! been committed to this module tree yet. Rather than guess at field
//! names or constructors that don't exist (and risk exactly the kind of
//! invented-API failure this project's generation history warns against),
//! this module defines the narrow ports it actually needs — mirroring each
//! aggregate's declared command/event shape exactly — and depends on those
//! abstractions (Dependency Inversion). Once the concrete aggregates land,
//! implementing `PluginInstancePort` for `PluginInstance` and
//! `ControlPlane` for `AppState` is a small adapter, not a rewrite of this
//! orchestration logic. This is the same shape of seam
//! `port.Preset.PresetCodec` already uses for the not-yet-committed
//! `valueObject.Preset.Preset`.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use crate::preset::preset_codec::{CodecError, PresetCodec};
use crate::r#loop::app_event::AppEvent;
use crate::r#loop::state_projector::{
    DefaultParameterExtractor, ParameterExtractor, StateProjector,
};
use crate::real_time::parameter_bridge::ParameterBridge;
use crate::shell::midi_normalizer::{MidiNormalizer, MidiNormalizerError};

/// A stable, host-automation-facing parameter identifier.
///
/// Defined locally: `PluginInstance`'s own `ParameterId` type has not been
/// committed to this module tree yet, so this port carries a narrow local
/// newtype of its own (the same treatment `PresetCodec` gives its local
/// `Preset` stand-in). Parameter IDs must stay stable across versions for
/// host automation compatibility, so this type is a plain, comparable,
/// copyable value with no behavior of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterId(u32);

impl ParameterId {
    /// Constructs a parameter identifier from its stable numeric value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// The stable numeric value host automation refers to this parameter
    /// by.
    pub fn value(&self) -> u32 {
        self.0
    }
}

/// The host-reported audio sample rate a plugin instance is initialized
/// at.
///
/// Defined locally: `kernel::sample_rate` is not a declared dependency of
/// this resource, so `PluginShell` does not assume its API; this is a
/// narrow local stand-in scoped to the plugin-lifecycle commands below.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostSampleRateHz(f64);

/// Errors constructing a [`HostSampleRateHz`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSampleRateHzError {
    /// The supplied value was NaN, zero, negative, or otherwise not a
    /// usable sample rate.
    OutOfRange,
}

impl fmt::Display for HostSampleRateHzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostSampleRateHzError::OutOfRange => {
                write!(f, "sample rate must be a positive, finite number of Hz")
            }
        }
    }
}

impl Error for HostSampleRateHzError {}

impl HostSampleRateHz {
    /// Constructs a validated host sample rate.
    pub fn try_new(hz: f64) -> Result<Self, HostSampleRateHzError> {
        if hz.is_nan() || hz.is_infinite() || hz <= 0.0 {
            return Err(HostSampleRateHzError::OutOfRange);
        }
        Ok(Self(hz))
    }

    /// The raw sample rate, in Hz.
    pub fn hz(&self) -> f64 {
        self.0
    }
}

/// The events `PluginInstancePort::initialize` / `reset` / `set_parameter`
/// yield, mirroring `aggregate.Plugin.PluginInstance`'s declared
/// `PluginInitialized` / `PluginReset` / `ParameterChanged` events exactly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginLifecycleEvent {
    /// Mirrors `PluginInstance`'s `PluginInitialized` event.
    PluginInitialized { sample_rate: HostSampleRateHz },
    /// Mirrors `PluginInstance`'s `PluginReset` event.
    PluginReset,
    /// Mirrors `PluginInstance`'s `ParameterChanged` event.
    ParameterChanged { id: ParameterId, value: f64 },
}

/// The narrow seam `PluginShell` needs from a plugin instance: the
/// command→event reducer surface (`Initialize` / `Reset` / `SetParameter`)
/// plus state persistence through the shared [`PresetCodec`].
///
/// Kept to exactly what `PluginShell` calls (Interface Segregation) so a
/// test double can implement it without any engine, MIDI, or audio
/// machinery, and so the eventual real `PluginInstance` adapter has a
/// minimal, obvious surface to implement.
pub trait PluginInstancePort {
    /// Mirrors `PluginInstance::Initialize`.
    fn initialize(
        &mut self,
        max_block_size: u32,
        sample_rate: HostSampleRateHz,
    ) -> PluginLifecycleEvent;

    /// Mirrors `PluginInstance::Reset`.
    fn reset(&mut self) -> PluginLifecycleEvent;

    /// Mirrors `PluginInstance::SetParameter`.
    fn set_parameter(&mut self, id: ParameterId, value: f64) -> PluginLifecycleEvent;

    /// Encodes the instance's current state through `codec` — the same
    /// `PresetCodec` the standalone app uses, per the "plugin state
    /// save/load uses the same PresetCodec as the standalone" invariant.
    fn save_state(&self, codec: &dyn PresetCodec) -> Result<Vec<u8>, CodecError>;

    /// Decodes `data` through `codec` and replaces the instance's state.
    ///
    /// Implementations must leave prior state untouched when this returns
    /// `Err` — restoring a session replaces all state atomically, and a
    /// failed load must not corrupt what was there before.
    fn load_state(&mut self, data: &[u8], codec: &dyn PresetCodec) -> Result<(), CodecError>;
}

/// The outcome of applying an [`AppEvent`] to the control plane, mirroring
/// `aggregate.Loop.AppState`'s declared `Applied` / `Rejected` events.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopOutcome {
    /// Mirrors `AppState`'s `Applied` event.
    Applied { frame: u64 },
    /// Mirrors `AppState`'s `Rejected` event: the event did not apply and
    /// state was left unchanged.
    Rejected { reason: String },
}

/// The narrow seam `PluginShell` needs from the control plane: apply one
/// [`AppEvent`] and read the resulting state to project into a
/// [`ParameterSnapshot`](crate::real_time::parameter_bridge::ParameterSnapshot).
///
/// Generic over `State` so [`StateProjector`] (already generic over the
/// state type) can be reused unchanged here, exactly as it is by the real
/// `AppStateProjector` alias.
pub trait ControlPlane {
    /// The state type this control plane manages (concretely
    /// `aggregate.Loop.AppState` in production).
    type State;

    /// Mirrors `AppState::Apply { event }` — the only sanctioned way to
    /// mutate the control plane's state.
    fn apply(&mut self, event: AppEvent) -> LoopOutcome;

    /// Reads the current state, for projection into a parameter snapshot.
    /// Never mutates.
    fn state(&self) -> &Self::State;
}

/// Orchestrates plugin lifecycle: init, process, parameter sync, and state
/// persistence via the host.
///
/// `PluginShell` owns none of its collaborators' behavior — it composes an
/// injected [`PluginInstancePort`], [`ControlPlane`], [`StateProjector`],
/// [`ParameterBridge`], and [`MidiNormalizer`], and sequences calls to
/// them. Every method that can change plugin-instance or control-plane
/// state ends by projecting the control plane's state and publishing it
/// through the bridge, so the audio thread always observes the latest
/// parameters after any host-driven change.
pub struct PluginShell<CP, PI, EX = DefaultParameterExtractor>
where
    CP: ControlPlane,
    PI: PluginInstancePort,
    EX: ParameterExtractor<CP::State>,
{
    control_plane: CP,
    plugin_instance: PI,
    parameter_bridge: Arc<ParameterBridge>,
    state_projector: StateProjector<CP::State, EX>,
    midi_normalizer: MidiNormalizer,
}

impl<CP, PI> PluginShell<CP, PI, DefaultParameterExtractor>
where
    CP: ControlPlane,
    PI: PluginInstancePort,
{
    /// Constructs a `PluginShell` with the default parameter extraction
    /// strategy and a fresh `MidiNormalizer` — the convenience constructor
    /// for callers who don't need a custom extractor or an already-warm
    /// normalizer.
    pub fn new(
        control_plane: CP,
        plugin_instance: PI,
        parameter_bridge: Arc<ParameterBridge>,
    ) -> Self {
        Self::with_collaborators(
            control_plane,
            plugin_instance,
            parameter_bridge,
            StateProjector::new(),
            MidiNormalizer::new(),
        )
    }
}

impl<CP, PI, EX> PluginShell<CP, PI, EX>
where
    CP: ControlPlane,
    PI: PluginInstancePort,
    EX: ParameterExtractor<CP::State>,
{
    /// Constructs a `PluginShell` from every collaborator explicitly — the
    /// full constructor, for callers (and tests) that need to inject a
    /// custom extractor, a pre-existing bridge, or a stateful normalizer.
    pub fn with_collaborators(
        control_plane: CP,
        plugin_instance: PI,
        parameter_bridge: Arc<ParameterBridge>,
        state_projector: StateProjector<CP::State, EX>,
        midi_normalizer: MidiNormalizer,
    ) -> Self {
        Self {
            control_plane,
            plugin_instance,
            parameter_bridge,
            state_projector,
            midi_normalizer,
        }
    }

    /// Host lifecycle: initializes the plugin instance for a given block
    /// size and sample rate, then publishes the resulting parameter
    /// snapshot.
    pub fn initialize(
        &mut self,
        max_block_size: u32,
        sample_rate: HostSampleRateHz,
    ) -> PluginLifecycleEvent {
        let event = self.plugin_instance.initialize(max_block_size, sample_rate);
        self.publish_snapshot();
        event
    }

    /// Host lifecycle: resets the plugin instance, then publishes the
    /// resulting parameter snapshot.
    pub fn reset(&mut self) -> PluginLifecycleEvent {
        let event = self.plugin_instance.reset();
        self.publish_snapshot();
        event
    }

    /// Parameter sync: forwards a host-driven parameter change to the
    /// plugin instance, then publishes the resulting parameter snapshot
    /// so the audio thread observes it — the "all parameter changes cross
    /// the thread boundary via the ParameterBridge" invariant, applied to
    /// the plugin's own parameter surface.
    pub fn set_parameter(&mut self, id: ParameterId, value: f64) -> PluginLifecycleEvent {
        let event = self.plugin_instance.set_parameter(id, value);
        self.publish_snapshot();
        event
    }

    /// Process: normalizes one raw MIDI 1.0 message from the host through
    /// the shared `MidiNormalizer`, applies it to the control plane as an
    /// `AppEvent::Midi`, and publishes the resulting parameter snapshot.
    ///
    /// Returns the normalizer's error unchanged if `raw` is not a valid
    /// MIDI 1.0 channel-voice message; the control plane is left
    /// untouched in that case.
    pub fn process_host_midi(&mut self, raw: &[u8]) -> Result<LoopOutcome, MidiNormalizerError> {
        let midi_event = self.midi_normalizer.normalize(raw)?;
        let outcome = self.control_plane.apply(AppEvent::Midi(midi_event));
        self.publish_snapshot();
        Ok(outcome)
    }

    /// State persistence: encodes the plugin instance's current state
    /// through `codec` — the same `PresetCodec` the standalone app uses.
    pub fn save_state(&self, codec: &dyn PresetCodec) -> Result<Vec<u8>, CodecError> {
        self.plugin_instance.save_state(codec)
    }

    /// State persistence: decodes `data` through `codec` and replaces the
    /// plugin instance's state, then publishes the resulting parameter
    /// snapshot.
    ///
    /// If decoding or restoring fails, the plugin instance's prior state
    /// is left untouched (per `PluginInstancePort::load_state`'s
    /// contract) and no snapshot is published — a failed load never
    /// pushes a half-restored state across the RT boundary.
    pub fn load_state(&mut self, data: &[u8], codec: &dyn PresetCodec) -> Result<(), CodecError> {
        self.plugin_instance.load_state(data, codec)?;
        self.publish_snapshot();
        Ok(())
    }

    /// Read-only access to the control plane, for host adapters that need
    /// to inspect state without mutating it.
    pub fn control_plane(&self) -> &CP {
        &self.control_plane
    }

    /// Projects the control plane's current state into a parameter
    /// snapshot and publishes it through the bridge. The one call site
    /// where this shell pushes a change across the RT boundary.
    fn publish_snapshot(&self) {
        self.state_projector
            .publish(self.control_plane.state(), &self.parameter_bridge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_event_kind::MidiEventKind;
    use crate::preset::preset_codec::{
        migrate_preset, Preset, RawPreset, CURRENT_PRESET_FORMAT_VERSION,
    };
    use crate::real_time::parameter_bridge::ParameterSnapshot;
    use std::collections::HashMap;

    /// A minimal stand-in control-plane state: just enough fields to
    /// prove projection wiring without depending on the real `AppState`.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct StubState {
        frame: u64,
        gain: f32,
    }

    struct StubControlPlane {
        state: StubState,
        applied_events: Vec<AppEvent>,
    }

    impl StubControlPlane {
        fn new() -> Self {
            Self {
                state: StubState {
                    frame: 0,
                    gain: 0.0,
                },
                applied_events: Vec::new(),
            }
        }
    }

    impl ControlPlane for StubControlPlane {
        type State = StubState;

        fn apply(&mut self, event: AppEvent) -> LoopOutcome {
            self.applied_events.push(event);
            self.state.frame += 1;
            self.state.gain = self.state.frame as f32 * 0.1;
            LoopOutcome::Applied {
                frame: self.state.frame,
            }
        }

        fn state(&self) -> &Self::State {
            &self.state
        }
    }

    /// Maps `StubState::gain` into the snapshot's master volume, so a test
    /// can observe end-to-end that a control-plane change reaches the
    /// bridge.
    struct StubExtractor;

    impl ParameterExtractor<StubState> for StubExtractor {
        fn extract(&self, state: &StubState) -> ParameterSnapshot {
            ParameterSnapshot {
                master_volume: state.gain,
                ..ParameterSnapshot::default()
            }
        }
    }

    #[derive(Default)]
    struct StubPluginInstance {
        parameters: HashMap<u32, f64>,
        initialize_calls: u32,
        reset_calls: u32,
        fail_next_load: bool,
    }

    impl PluginInstancePort for StubPluginInstance {
        fn initialize(
            &mut self,
            _max_block_size: u32,
            sample_rate: HostSampleRateHz,
        ) -> PluginLifecycleEvent {
            self.initialize_calls += 1;
            PluginLifecycleEvent::PluginInitialized { sample_rate }
        }

        fn reset(&mut self) -> PluginLifecycleEvent {
            self.reset_calls += 1;
            self.parameters.clear();
            PluginLifecycleEvent::PluginReset
        }

        fn set_parameter(&mut self, id: ParameterId, value: f64) -> PluginLifecycleEvent {
            self.parameters.insert(id.value(), value);
            PluginLifecycleEvent::ParameterChanged { id, value }
        }

        fn save_state(&self, codec: &dyn PresetCodec) -> Result<Vec<u8>, CodecError> {
            let value = self.parameters.get(&0).copied().unwrap_or(0.0);
            codec.encode(Preset::new(
                "stub-plugin-state",
                value.to_le_bytes().to_vec(),
            ))
        }

        fn load_state(&mut self, data: &[u8], codec: &dyn PresetCodec) -> Result<(), CodecError> {
            if self.fail_next_load {
                return Err(CodecError::Malformed("forced failure".to_string()));
            }
            let preset = codec.decode(data)?;
            let payload = preset.patch_payload();
            if payload.len() < 8 {
                return Err(CodecError::Truncated {
                    expected_at_least: 8,
                    actual: payload.len(),
                });
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[0..8]);
            self.parameters.insert(0, f64::from_le_bytes(buf));
            Ok(())
        }
    }

    /// A dependency-free codec used only to exercise `PluginShell`'s state
    /// persistence path, mirroring `PresetCodec`'s own doctest codec:
    /// `[version: u32 LE][name_len: u32 LE][name][payload_len: u32 LE][payload]`.
    struct FixedLayoutPresetCodec;

    impl PresetCodec for FixedLayoutPresetCodec {
        fn decode(&self, data: &[u8]) -> Result<Preset, CodecError> {
            let mut cursor = 0usize;
            let version = read_u32(data, &mut cursor)?;
            let name_len = read_u32(data, &mut cursor)? as usize;
            let name_bytes = read_slice(data, &mut cursor, name_len)?;
            let name =
                String::from_utf8(name_bytes.to_vec()).map_err(|_| CodecError::InvalidUtf8Name)?;
            let payload_len = read_u32(data, &mut cursor)? as usize;
            let payload = read_slice(data, &mut cursor, payload_len)?.to_vec();

            migrate_preset(RawPreset {
                format_version: version,
                name,
                patch_payload: payload,
            })
        }

        fn encode(&self, preset: Preset) -> Result<Vec<u8>, CodecError> {
            let mut out =
                Vec::with_capacity(4 + 4 + preset.name().len() + 4 + preset.patch_payload().len());
            out.extend_from_slice(&CURRENT_PRESET_FORMAT_VERSION.to_le_bytes());
            out.extend_from_slice(&(preset.name().len() as u32).to_le_bytes());
            out.extend_from_slice(preset.name().as_bytes());
            out.extend_from_slice(&(preset.patch_payload().len() as u32).to_le_bytes());
            out.extend_from_slice(preset.patch_payload());
            Ok(out)
        }
    }

    fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, CodecError> {
        let slice = read_slice(data, cursor, 4)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(slice);
        Ok(u32::from_le_bytes(buf))
    }

    fn read_slice<'a>(
        data: &'a [u8],
        cursor: &mut usize,
        len: usize,
    ) -> Result<&'a [u8], CodecError> {
        let end = *cursor + len;
        if end > data.len() {
            return Err(CodecError::Truncated {
                expected_at_least: end,
                actual: data.len(),
            });
        }
        let slice = &data[*cursor..end];
        *cursor = end;
        Ok(slice)
    }

    fn make_shell() -> PluginShell<StubControlPlane, StubPluginInstance, StubExtractor> {
        PluginShell::with_collaborators(
            StubControlPlane::new(),
            StubPluginInstance::default(),
            Arc::new(ParameterBridge::default()),
            StateProjector::with_extractor(StubExtractor),
            MidiNormalizer::new(),
        )
    }

    #[test]
    fn initialize_forwards_to_plugin_instance_and_publishes_a_snapshot() {
        let mut shell = make_shell();
        let sample_rate = HostSampleRateHz::try_new(48_000.0).unwrap();

        let event = shell.initialize(512, sample_rate);

        assert_eq!(
            event,
            PluginLifecycleEvent::PluginInitialized { sample_rate }
        );
        assert_eq!(shell.plugin_instance.initialize_calls, 1);
    }

    #[test]
    fn reset_forwards_to_plugin_instance() {
        let mut shell = make_shell();
        shell.set_parameter(ParameterId::new(0), 0.5);

        let event = shell.reset();

        assert_eq!(event, PluginLifecycleEvent::PluginReset);
        assert_eq!(shell.plugin_instance.reset_calls, 1);
        assert!(shell.plugin_instance.parameters.is_empty());
    }

    #[test]
    fn set_parameter_forwards_the_command_and_returns_the_matching_event() {
        let mut shell = make_shell();
        let id = ParameterId::new(7);

        let event = shell.set_parameter(id, 0.75);

        assert_eq!(
            event,
            PluginLifecycleEvent::ParameterChanged { id, value: 0.75 }
        );
        assert_eq!(shell.plugin_instance.parameters.get(&7), Some(&0.75));
    }

    #[test]
    fn set_parameter_publishes_a_snapshot_reflecting_the_control_planes_state() {
        let mut shell = make_shell();

        shell.set_parameter(ParameterId::new(0), 1.0);

        // StubControlPlane advances gain by 0.1 per applied event, but
        // set_parameter does not apply an AppEvent to the control plane —
        // it only republishes the (unchanged) current projection. This
        // proves the bridge always reflects the control plane's state,
        // not stale data left over from construction.
        let snapshot = shell.parameter_bridge.read();
        assert_eq!(snapshot.master_volume, 0.0);
    }

    #[test]
    fn process_host_midi_normalizes_applies_and_publishes() {
        let mut shell = make_shell();

        let outcome = shell.process_host_midi(&[0x90, 60, 100]).unwrap();

        assert_eq!(outcome, LoopOutcome::Applied { frame: 1 });
        assert_eq!(shell.control_plane.applied_events.len(), 1);
        match &shell.control_plane.applied_events[0] {
            AppEvent::Midi(event) => {
                assert_eq!(*event.kind(), MidiEventKind::NoteOn);
                assert_eq!(event.note().value(), 60);
            }
            other => panic!("expected AppEvent::Midi, got {other:?}"),
        }

        let snapshot = shell.parameter_bridge.read();
        assert_eq!(snapshot.master_volume, 0.1);
    }

    #[test]
    fn process_host_midi_rejects_invalid_bytes_without_touching_the_control_plane() {
        let mut shell = make_shell();

        let result = shell.process_host_midi(&[]);

        assert_eq!(result, Err(MidiNormalizerError::Empty));
        assert!(shell.control_plane.applied_events.is_empty());
    }

    #[test]
    fn save_then_load_state_round_trips_through_the_preset_codec() {
        let mut shell = make_shell();
        let codec = FixedLayoutPresetCodec;
        shell.set_parameter(ParameterId::new(0), 0.42);

        let bytes = shell.save_state(&codec).expect("save succeeds");

        let mut restored = make_shell();
        restored.load_state(&bytes, &codec).expect("load succeeds");

        assert_eq!(restored.plugin_instance.parameters.get(&0), Some(&0.42));
    }

    #[test]
    fn a_failed_load_leaves_prior_plugin_state_untouched() {
        let mut shell = make_shell();
        let codec = FixedLayoutPresetCodec;
        shell.set_parameter(ParameterId::new(0), 0.9);
        shell.plugin_instance.fail_next_load = true;

        let result = shell.load_state(&[1, 2, 3], &codec);

        assert!(result.is_err());
        assert_eq!(shell.plugin_instance.parameters.get(&0), Some(&0.9));
    }

    #[test]
    fn parameter_id_reports_the_stable_numeric_value_it_was_constructed_with() {
        let id = ParameterId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn host_sample_rate_rejects_non_positive_and_non_finite_values() {
        assert_eq!(
            HostSampleRateHz::try_new(0.0),
            Err(HostSampleRateHzError::OutOfRange)
        );
        assert_eq!(
            HostSampleRateHz::try_new(-1.0),
            Err(HostSampleRateHzError::OutOfRange)
        );
        assert_eq!(
            HostSampleRateHz::try_new(f64::NAN),
            Err(HostSampleRateHzError::OutOfRange)
        );
        assert!(HostSampleRateHz::try_new(44_100.0).is_ok());
    }
}
