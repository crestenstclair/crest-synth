use crate::mixer::bus_id::BusId;
use crate::mixer::bus_return::BusReturnBank;
use crate::mixer::bus_return::EffectError;
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::patch_audio_block::{PatchAudioBlock, PatchAudioBlockError};
use crate::real_time::prepared_graph::{PreparedGraph, PreparedGraphResources};
use crate::synth::instrument_capability::CapabilityRegistry;
use crate::synth::instrument_preparer::InstrumentPreparer;
use crate::synth::patch::Patch;
use crate::synth::prepared_engine_rack_builder::{PreparedEngineRackBuilder, RackPreparationError};
use crate::synth::EffectPreparationError;
use crate::synth::{
    EffectCapabilityRegistry, EffectPreparer, EffectRackPreparationError,
    PreparedPostEffectRackBuilder,
};
use core::fmt;

/// Control/worker-side composition service for one complete prepared graph.
pub struct PreparedGraphBuilder<'a> {
    registry: &'a CapabilityRegistry,
    preparers: &'a [Box<dyn InstrumentPreparer>],
    effect_registry: Option<&'a EffectCapabilityRegistry>,
    effect_preparers: &'a [Box<dyn EffectPreparer>],
    returns: Option<&'a BusReturnBank>,
}

impl<'a> PreparedGraphBuilder<'a> {
    pub const fn new(
        registry: &'a CapabilityRegistry,
        preparers: &'a [Box<dyn InstrumentPreparer>],
    ) -> Self {
        Self {
            registry,
            preparers,
            effect_registry: None,
            effect_preparers: &[],
            returns: None,
        }
    }

    /// Injects the complete installed effect registry and its exact preparers.
    pub const fn with_effects(
        mut self,
        registry: &'a EffectCapabilityRegistry,
        preparers: &'a [Box<dyn EffectPreparer>],
    ) -> Self {
        self.effect_registry = Some(registry);
        self.effect_preparers = preparers;
        self
    }

    /// Injects the canonical bus-return occupancy this graph must prepare.
    ///
    /// Every occupied return is prepared through the injected effect registry
    /// and preparers and installed at its exact bus. Without a bank the
    /// prepared return rack stays empty — occupancy is never installed
    /// implicitly.
    pub const fn with_returns(mut self, returns: &'a BusReturnBank) -> Self {
        self.returns = Some(returns);
        self
    }

    /// Prepares every graph owner and buffer before returning one atomic
    /// ownership unit. Any error destroys only candidate values on this
    /// control/worker call stack.
    pub fn build(
        &self,
        revision: GraphRevision,
        patches: &[Patch],
        parameters: ParameterSnapshot,
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<PreparedGraph, GraphPreparationError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(GraphPreparationError::InvalidSampleRate);
        }
        if max_frames == 0 {
            return Err(GraphPreparationError::InvalidFrameCapacity);
        }
        if parameters.graph_revision() != revision {
            return Err(GraphPreparationError::RevisionMismatch {
                graph: revision,
                parameters: parameters.graph_revision(),
            });
        }
        if parameters.patch_count() != patches.len()
            || parameters
                .patches()
                .iter()
                .zip(patches)
                .any(|(parameters, patch)| parameters.patch_id() != Some(patch.id()))
        {
            return Err(GraphPreparationError::ParameterLayoutMismatch);
        }

        let engine_rack = PreparedEngineRackBuilder::build(
            patches,
            self.registry,
            self.preparers,
            sample_rate,
            max_frames,
        )
        .map_err(GraphPreparationError::Rack)?;
        if !engine_rack.matches_parameters(&parameters) {
            return Err(GraphPreparationError::ParameterLayoutMismatch);
        }

        let empty_effect_registry = EffectCapabilityRegistry::default();
        let effect_registry = self.effect_registry.unwrap_or(&empty_effect_registry);
        let effect_rack = PreparedPostEffectRackBuilder::build(
            patches,
            effect_registry,
            self.effect_preparers,
            sample_rate,
            max_frames,
        )
        .map_err(GraphPreparationError::EffectRack)?;
        if !effect_rack.matches_parameters(&parameters) {
            return Err(GraphPreparationError::ParameterLayoutMismatch);
        }

        let patch_audio =
            PatchAudioBlock::prepare(max_frames).map_err(GraphPreparationError::PatchAudio)?;
        let mut mixer = MixEngine::new();
        mixer
            .prepare(sample_rate, max_frames)
            .map_err(GraphPreparationError::Effects)?;
        // Return occupancy comes exclusively from the injected canonical
        // bank. `prepare` empties the rack, so occupancy is (re-)installed
        // after every preparation; a bank the injected registry or preparers
        // cannot satisfy refuses the complete graph with the failing bus.
        if let Some(bank) = self.returns {
            for bus_return in bank.returns() {
                let Some(config) = bus_return.effect() else {
                    continue;
                };
                let bus = bus_return.id();
                let descriptor = effect_registry.descriptor(config.capability_id()).ok_or(
                    GraphPreparationError::BusReturn {
                        bus,
                        source: BusReturnPreparationError::UnknownRegistryEntry,
                    },
                )?;
                let invalid = |_| GraphPreparationError::BusReturn {
                    bus,
                    source: BusReturnPreparationError::InvalidConfiguration,
                };
                let scalars = descriptor
                    .scalar_parameters()
                    .map(|spec| {
                        let value =
                            config
                                .value(spec.id())
                                .ok_or(GraphPreparationError::BusReturn {
                                    bus,
                                    source: BusReturnPreparationError::InvalidConfiguration,
                                })?;
                        spec.scalar_value(value).map_err(invalid)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let live =
                    crate::real_time::RtPostEffectParameters::new(config.slot_id(), &scalars)
                        .map_err(|_| GraphPreparationError::BusReturn {
                            bus,
                            source: BusReturnPreparationError::InvalidConfiguration,
                        })?;
                let preparer = self
                    .effect_preparers
                    .iter()
                    .find(|preparer| preparer.capability_id() == config.capability_id())
                    .ok_or(GraphPreparationError::BusReturn {
                        bus,
                        source: BusReturnPreparationError::MissingPreparer,
                    })?;
                let prepared = preparer
                    .prepare(RETURN_OCCUPANT_PATCH_ID, config, sample_rate, max_frames)
                    .map_err(|source| GraphPreparationError::BusReturn {
                        bus,
                        source: BusReturnPreparationError::Preparation(source),
                    })?;
                mixer
                    .install_bus_return(bus, prepared, live, bus_return.return_level())
                    .map_err(|source| GraphPreparationError::BusReturn {
                        bus,
                        source: BusReturnPreparationError::Install(source),
                    })?;
            }
        }
        if !mixer.bus_returns().matches_parameters(&parameters) {
            return Err(GraphPreparationError::ParameterLayoutMismatch);
        }

        Ok(PreparedGraph::new(
            revision,
            sample_rate,
            max_frames,
            parameters,
            PreparedGraphResources::new(engine_rack, effect_rack, patch_audio, mixer),
        ))
    }
}

/// The stable synthetic Patch identity carried by prepared return occupants.
///
/// Bus returns are not owned by any Patch; the prepared-effect port requires
/// a Patch identity, so returns use the reserved maximum, exactly as the
/// retired production bridge did.
const RETURN_OCCUPANT_PATCH_ID: crate::kernel::PatchId = match crate::kernel::PatchId::new(u32::MAX)
{
    Ok(patch_id) => patch_id,
    Err(_) => panic!("the reserved bus-return Patch id is non-zero"),
};

/// A typed atomic complete-graph preparation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphPreparationError {
    InvalidSampleRate,
    InvalidFrameCapacity,
    RevisionMismatch {
        graph: GraphRevision,
        parameters: GraphRevision,
    },
    ParameterLayoutMismatch,
    Rack(RackPreparationError),
    EffectRack(EffectRackPreparationError),
    PatchAudio(PatchAudioBlockError),
    Effects(EffectError),
    /// One bus-return occupant could not be prepared; the failure names its
    /// exact bus so the refusal stays attributable to that position.
    BusReturn {
        bus: BusId,
        source: BusReturnPreparationError,
    },
}

/// Why one bus-return occupant refused preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BusReturnPreparationError {
    /// The occupying entry is not installed in the injected effect registry.
    UnknownRegistryEntry,
    /// The occupant's configuration does not satisfy its descriptor.
    InvalidConfiguration,
    /// No injected preparer accepts the occupying registry entry.
    MissingPreparer,
    /// The preparer refused to build the instance.
    Preparation(EffectPreparationError),
    /// The prepared instance could not be installed into the return rack.
    Install(EffectError),
}

impl fmt::Display for BusReturnPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRegistryEntry => {
                formatter.write_str("occupying entry is not in the effect registry")
            }
            Self::InvalidConfiguration => {
                formatter.write_str("occupant configuration does not satisfy its descriptor")
            }
            Self::MissingPreparer => formatter.write_str("no preparer accepts the occupying entry"),
            Self::Preparation(source) => write!(formatter, "preparation refused: {source}"),
            Self::Install(source) => write!(formatter, "installation refused: {source}"),
        }
    }
}

impl fmt::Display for GraphPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate => {
                formatter.write_str("prepared graph sample rate must be finite and positive")
            }
            Self::InvalidFrameCapacity => {
                formatter.write_str("prepared graph frame capacity must be nonzero")
            }
            Self::RevisionMismatch { graph, parameters } => write!(
                formatter,
                "prepared graph revision {graph} does not match parameters {parameters}"
            ),
            Self::ParameterLayoutMismatch => formatter.write_str(
                "prepared graph Patch count or ordered identities do not match parameters",
            ),
            Self::Rack(source) => write!(formatter, "prepared engine rack failed: {source}"),
            Self::EffectRack(source) => {
                write!(formatter, "prepared post-effect rack failed: {source}")
            }
            Self::PatchAudio(source) => {
                write!(formatter, "Patch stem preparation failed: {source}")
            }
            Self::Effects(source) => {
                write!(formatter, "global effect preparation failed: {source}")
            }
            Self::BusReturn { bus, source } => {
                write!(formatter, "bus return {bus} preparation failed: {source}")
            }
        }
    }
}

impl std::error::Error for GraphPreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rack(source) => Some(source),
            Self::EffectRack(source) => Some(source),
            Self::PatchAudio(source) => Some(source),
            Self::Effects(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GraphPreparationError, PreparedGraphBuilder};
    use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessage;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_state::MixerState;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::graph_revision::{GraphRevision, GraphRevisionError};
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::PreparedGraphRefreshError;
    use crate::synth::capability_id::CapabilityId;
    use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
    use crate::synth::patch::Patch;
    use crate::synth::prepared_engine_rack_builder::RackPreparationError;
    use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn patch(id: u32) -> Patch {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(0, (id - 1) as u8, false).unwrap(),
            )
            .unwrap(),
            MidiChannel::new((id - 1) as u8).unwrap(),
            PatchOutput::to_track(MixerTrackId::new((id - 1) as u8).unwrap()),
        )
    }

    fn globals() -> GlobalParameters {
        GlobalParameters::new(0.0).unwrap()
    }

    fn parameters(revision: GraphRevision, patches: &[Patch]) -> ParameterSnapshot {
        let patches: Vec<_> = patches
            .iter()
            .map(|patch| RtPatchParameters::new(patch.id(), patch.output()))
            .collect();
        ParameterSnapshot::for_graph(1, revision, globals(), MixerState::default(), &patches)
            .unwrap()
    }

    struct FixturePreparer {
        capability_id: CapabilityId,
        fail: bool,
    }

    impl FixturePreparer {
        fn boxed(fail: bool) -> Box<dyn InstrumentPreparer> {
            Box::new(Self {
                capability_id: CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap(),
                fail,
            })
        }
    }

    impl InstrumentPreparer for FixturePreparer {
        fn capability_id(&self) -> &CapabilityId {
            &self.capability_id
        }

        fn prepare(
            &self,
            patch: &Patch,
            _sample_rate: f32,
            _max_frames: usize,
        ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
            if self.fail {
                Err(InstrumentPreparationError::PreparationFailed {
                    patch_id: patch.id(),
                })
            } else {
                Ok(Box::new(FixtureInstrument {
                    patch_id: patch.id(),
                }))
            }
        }
    }

    struct FixtureInstrument {
        patch_id: PatchId,
    }

    impl PreparedInstrument for FixtureInstrument {
        fn patch_id(&self) -> PatchId {
            self.patch_id
        }

        fn dispatch(
            &mut self,
            _message: MidiMessage,
            _parameters: &crate::real_time::RtPatchParameters,
        ) -> Result<(), PreparedInstrumentError> {
            Ok(())
        }

        fn render(
            &mut self,
            output: &mut [f32],
            _frame_count: usize,
            _parameters: &crate::real_time::RtPatchParameters,
        ) {
            output.fill(0.25);
        }

        fn all_notes_off(&mut self) {}
    }

    #[test]
    fn builder_returns_one_complete_ordered_callback_ready_graph() {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let registry = provider.registry().unwrap();
        let preparers = vec![FixturePreparer::boxed(false)];
        let builder = PreparedGraphBuilder::new(&registry, &preparers);
        let revision = GraphRevision::new(7).unwrap();
        let patches = [patch(1), patch(2)];

        let graph = builder
            .build(
                revision,
                &patches,
                parameters(revision, &patches),
                48_000.0,
                256,
            )
            .unwrap();

        assert_eq!(graph.revision(), revision);
        assert_eq!(graph.sample_rate(), 48_000.0);
        assert_eq!(graph.max_frames(), 256);
        assert_eq!(graph.engine_rack().patch_count(), 2);
        assert_eq!(graph.engine_rack().patch_id(0), Some(patches[0].id()));
        assert_eq!(graph.engine_rack().patch_id(1), Some(patches[1].id()));
        assert_eq!(graph.patch_audio().max_frames(), 256);
        assert!(graph
            .engine_rack()
            .matches_parameters(graph.initial_parameters()));
    }

    #[test]
    fn prepared_graph_refreshes_only_an_exact_target_revision_and_engine_layout() {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let registry = provider.registry().unwrap();
        let preparers = vec![FixturePreparer::boxed(false)];
        let builder = PreparedGraphBuilder::new(&registry, &preparers);
        let revision = GraphRevision::new(7).unwrap();
        let patches = [patch(1), patch(2)];
        let mut graph = builder
            .build(
                revision,
                &patches,
                parameters(revision, &patches),
                48_000.0,
                256,
            )
            .unwrap();

        let mut edited_patches = patches.clone();
        edited_patches[0]
            .set_output(PatchOutput::new(MixerTrackId::new(3).unwrap(), -9.0).unwrap());
        let refreshed = parameters(revision, &edited_patches).with_generation(99);
        graph.refresh_initial_parameters(refreshed).unwrap();
        assert_eq!(graph.initial_parameters(), &refreshed);
        assert_eq!(
            graph
                .initial_parameters()
                .patch(edited_patches[0].id())
                .unwrap()
                .output(),
            edited_patches[0].output()
        );

        let retained = *graph.initial_parameters();
        assert_eq!(
            graph.refresh_initial_parameters(parameters(GraphRevision::new(8).unwrap(), &patches)),
            Err(PreparedGraphRefreshError::RevisionMismatch)
        );
        assert_eq!(graph.initial_parameters(), &retained);
        assert_eq!(
            graph.refresh_initial_parameters(parameters(revision, &[patch(2), patch(1)])),
            Err(PreparedGraphRefreshError::LayoutMismatch)
        );
        assert_eq!(graph.initial_parameters(), &retained);
    }

    #[test]
    fn candidate_failures_leave_an_existing_graph_unchanged() {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let registry = provider.registry().unwrap();
        let good_preparers = vec![FixturePreparer::boxed(false)];
        let builder = PreparedGraphBuilder::new(&registry, &good_preparers);
        let first_revision = GraphRevision::new(1).unwrap();
        let patches = [patch(1), patch(2)];
        let existing = builder
            .build(
                first_revision,
                &patches,
                parameters(first_revision, &patches),
                48_000.0,
                64,
            )
            .unwrap();

        assert_eq!(GraphRevision::new(0), Err(GraphRevisionError::Zero));
        assert!(matches!(
            builder.build(
                GraphRevision::new(2).unwrap(),
                &patches,
                parameters(first_revision, &patches),
                48_000.0,
                64,
            ),
            Err(GraphPreparationError::RevisionMismatch { .. })
        ));
        assert!(matches!(
            builder.build(
                first_revision,
                &patches,
                parameters(first_revision, &[patch(2), patch(1)]),
                48_000.0,
                64,
            ),
            Err(GraphPreparationError::ParameterLayoutMismatch)
        ));
        assert_eq!(
            builder
                .build(
                    first_revision,
                    &patches,
                    parameters(first_revision, &patches),
                    f32::NAN,
                    64,
                )
                .unwrap_err(),
            GraphPreparationError::InvalidSampleRate
        );
        assert_eq!(
            builder
                .build(
                    first_revision,
                    &patches,
                    parameters(first_revision, &patches),
                    48_000.0,
                    0,
                )
                .unwrap_err(),
            GraphPreparationError::InvalidFrameCapacity
        );

        let failing_preparers = vec![FixturePreparer::boxed(true)];
        let failing_builder = PreparedGraphBuilder::new(&registry, &failing_preparers);
        assert!(matches!(
            failing_builder.build(
                first_revision,
                &patches,
                parameters(first_revision, &patches),
                48_000.0,
                64,
            ),
            Err(GraphPreparationError::Rack(
                RackPreparationError::Instrument { .. }
            ))
        ));

        // An occupied bank the builder cannot prepare refuses the complete
        // graph and names the failing bus; nothing is silently substituted.
        let empty_preparers: Vec<Box<dyn InstrumentPreparer>> = Vec::new();
        let effect_registry =
            crate::adapter::production_effects::production_effect_registry().unwrap();
        let effect_preparers =
            crate::adapter::production_effects::production_effect_preparers().unwrap();
        let bank =
            crate::adapter::production_effects::production_default_bus_returns(&effect_registry)
                .unwrap();
        let return_parameters =
            ParameterSnapshot::for_graph(1, first_revision, globals(), MixerState::default(), &[])
                .unwrap()
                .with_returns(ParameterSnapshot::project_returns(&effect_registry, &bank).unwrap());
        let effects_builder = PreparedGraphBuilder::new(&registry, &empty_preparers)
            .with_effects(&effect_registry, &effect_preparers)
            .with_returns(&bank);
        assert!(matches!(
            effects_builder
                .build(first_revision, &[], return_parameters, f32::MAX, 1)
                .unwrap_err(),
            GraphPreparationError::BusReturn { bus, .. }
                if bus == crate::mixer::bus_id::BusId::new(0).unwrap()
        ));
        // Without its registry the same occupied bank is refused up front.
        let no_registry_builder =
            PreparedGraphBuilder::new(&registry, &empty_preparers).with_returns(&bank);
        assert!(matches!(
            no_registry_builder
                .build(first_revision, &[], return_parameters, 48_000.0, 1)
                .unwrap_err(),
            GraphPreparationError::BusReturn {
                source: super::BusReturnPreparationError::UnknownRegistryEntry,
                ..
            }
        ));

        assert_eq!(existing.revision(), first_revision);
        assert_eq!(existing.engine_rack().patch_count(), 2);
        assert_eq!(existing.engine_rack().patch_id(0), Some(patches[0].id()));
        assert_eq!(existing.engine_rack().patch_id(1), Some(patches[1].id()));
    }
}
