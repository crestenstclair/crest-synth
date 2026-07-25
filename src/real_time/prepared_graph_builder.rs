use crate::adapter::global_reverb_delay::GlobalReverbDelay;
use crate::mixer::global_effects_processor::EffectError;
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::patch_audio_block::{PatchAudioBlock, PatchAudioBlockError};
use crate::real_time::prepared_graph::PreparedGraph;
use crate::synth::instrument_capability::CapabilityRegistry;
use crate::synth::instrument_preparer::InstrumentPreparer;
use crate::synth::patch::Patch;
use crate::synth::prepared_engine_rack_builder::{PreparedEngineRackBuilder, RackPreparationError};
use core::fmt;

/// Control/worker-side composition service for one complete prepared graph.
pub struct PreparedGraphBuilder<'a> {
    registry: &'a CapabilityRegistry,
    preparers: &'a [Box<dyn InstrumentPreparer>],
}

impl<'a> PreparedGraphBuilder<'a> {
    pub const fn new(
        registry: &'a CapabilityRegistry,
        preparers: &'a [Box<dyn InstrumentPreparer>],
    ) -> Self {
        Self {
            registry,
            preparers,
        }
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

        let patch_audio =
            PatchAudioBlock::prepare(max_frames).map_err(GraphPreparationError::PatchAudio)?;
        let mut mixer = MixEngine::new(GlobalReverbDelay::new());
        mixer
            .prepare(sample_rate, max_frames)
            .map_err(GraphPreparationError::Effects)?;

        Ok(PreparedGraph::new(
            revision,
            sample_rate,
            max_frames,
            parameters,
            engine_rack,
            patch_audio,
            mixer,
        ))
    }
}

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
    PatchAudio(PatchAudioBlockError),
    Effects(EffectError),
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
            Self::PatchAudio(source) => {
                write!(formatter, "Patch stem preparation failed: {source}")
            }
            Self::Effects(source) => {
                write!(formatter, "global effect preparation failed: {source}")
            }
        }
    }
}

impl std::error::Error for GraphPreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rack(source) => Some(source),
            Self::PatchAudio(source) => Some(source),
            Self::Effects(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GraphPreparationError, PreparedGraphBuilder};
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
    };
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessage;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_effects_processor::EffectError;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::graph_revision::{GraphRevision, GraphRevisionError};
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::synth::capability_id::CapabilityId;
    use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
    use crate::synth::patch::Patch;
    use crate::synth::prepared_engine_rack_builder::RackPreparationError;
    use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn patch(id: u32) -> Patch {
        let provider = HiDefSoundFontCapability::new().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(0, (id - 1) as u8, false).unwrap(),
            )
            .unwrap(),
            MidiChannel::new((id - 1) as u8).unwrap(),
            ChannelParameters::default(),
        )
    }

    fn globals() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap()
    }

    fn parameters(revision: GraphRevision, patches: &[Patch]) -> ParameterSnapshot {
        let patches: Vec<_> = patches
            .iter()
            .map(|patch| RtPatchParameters::new(patch.id(), *patch.parameters()))
            .collect();
        ParameterSnapshot::for_graph(1, revision, globals(), &patches).unwrap()
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

        fn dispatch(&mut self, _message: MidiMessage) -> Result<(), PreparedInstrumentError> {
            Ok(())
        }

        fn render(&mut self, output: &mut [f32], _frame_count: usize) {
            output.fill(0.25);
        }

        fn all_notes_off(&mut self) {}
    }

    #[test]
    fn builder_returns_one_complete_ordered_callback_ready_graph() {
        let provider = HiDefSoundFontCapability::new().unwrap();
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
    fn candidate_failures_leave_an_existing_graph_unchanged() {
        let provider = HiDefSoundFontCapability::new().unwrap();
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

        let empty_preparers: Vec<Box<dyn InstrumentPreparer>> = Vec::new();
        let effects_builder = PreparedGraphBuilder::new(&registry, &empty_preparers);
        assert_eq!(
            effects_builder
                .build(
                    first_revision,
                    &[],
                    ParameterSnapshot::for_graph(1, first_revision, globals(), &[]).unwrap(),
                    f32::MAX,
                    1,
                )
                .unwrap_err(),
            GraphPreparationError::Effects(EffectError::StorageAllocationFailed)
        );

        assert_eq!(existing.revision(), first_revision);
        assert_eq!(existing.engine_rack().patch_count(), 2);
        assert_eq!(existing.engine_rack().patch_id(0), Some(patches[0].id()));
        assert_eq!(existing.engine_rack().patch_id(1), Some(patches[1].id()));
    }
}
