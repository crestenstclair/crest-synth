use crate::adapter::hidef_soundfont_capability::{
    HIDEF_CAPABILITY_ID, HIDEF_SOUNDFONT_PATH, SOUNDFONT_FILE_PARAMETER_ID,
    SOUNDFONT_PRESET_PARAMETER_ID,
};
use crate::control::app_event::AppEvent;
use crate::control::app_loop::AppLoop;
use crate::control::app_state::EventRejection;
use crate::control::event_record::EventSource;
use crate::kernel::patch_id::PatchId;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::patch_output::PatchOutput;
use crate::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
use crate::synth::instrument_capability::{
    AssetAssignment, AssetKind, AssetReference, CapabilityDescriptor, CapabilityError,
    CapabilityRegistry, InstrumentConfig, ParameterAssignment, ParameterDefault, ParameterValue,
};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::parameter_id::ParameterId;
use crate::synth::patch::Patch;
use crate::synth::sound_font_instrument::SoundFontInstrument;
use crate::synth::{
    EffectCapabilityDescriptor, EffectCapabilityError, EffectCapabilityProvider, EffectSlotId,
};
use crate::testing::midi_event_source::{FixedEventBatch, MidiEventSource, MidiSourceError};
use core::fmt;
use std::time::Duration;

/// A failure while preparing or advancing the automatic MIDI fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestInputError {
    Source(MidiSourceError),
    Capability(CapabilityError),
    EffectCapability(EffectCapabilityError),
    Control(EventRejection),
    AudioBoundaryFull(BoundaryFull),
    PatchIdentityOverflow { position: usize },
    DuplicatePartIndex { part_index: usize },
    UnknownPartIndex { part_index: usize },
    AlreadyInitialized,
    NotInitialized,
    AlreadyStarted,
    NotStarted,
}

impl fmt::Display for TestInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(formatter, "automatic MIDI source failed: {error}"),
            Self::Capability(error) => {
                write!(
                    formatter,
                    "automatic MIDI capability configuration failed: {error}"
                )
            }
            Self::EffectCapability(error) => {
                write!(
                    formatter,
                    "automatic MIDI effect configuration failed: {error}"
                )
            }
            Self::Control(error) => {
                write!(formatter, "automatic MIDI event was rejected: {error}")
            }
            Self::AudioBoundaryFull(error) => error.fmt(formatter),
            Self::PatchIdentityOverflow { position } => write!(
                formatter,
                "fixture part at position {position} cannot be assigned a PatchId"
            ),
            Self::DuplicatePartIndex { part_index } => {
                write!(
                    formatter,
                    "fixture contains duplicate part index {part_index}"
                )
            }
            Self::UnknownPartIndex { part_index } => {
                write!(
                    formatter,
                    "fixture MIDI targets unknown part index {part_index}"
                )
            }
            Self::AlreadyInitialized => {
                formatter.write_str("automatic MIDI test is already initialized")
            }
            Self::NotInitialized => {
                formatter.write_str("automatic MIDI test must be initialized before ticking")
            }
            Self::AlreadyStarted => formatter.write_str("automatic MIDI source is already started"),
            Self::NotStarted => {
                formatter.write_str("automatic MIDI source must be started before ticking")
            }
        }
    }
}

impl std::error::Error for TestInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Capability(error) => Some(error),
            Self::EffectCapability(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::AudioBoundaryFull(error) => Some(error),
            Self::PatchIdentityOverflow { .. }
            | Self::DuplicatePartIndex { .. }
            | Self::UnknownPartIndex { .. }
            | Self::AlreadyInitialized
            | Self::NotInitialized
            | Self::AlreadyStarted
            | Self::NotStarted => None,
        }
    }
}

impl From<MidiSourceError> for TestInputError {
    fn from(error: MidiSourceError) -> Self {
        Self::Source(error)
    }
}

impl From<CapabilityError> for TestInputError {
    fn from(error: CapabilityError) -> Self {
        Self::Capability(error)
    }
}

impl From<EffectCapabilityError> for TestInputError {
    fn from(error: EffectCapabilityError) -> Self {
        Self::EffectCapability(error)
    }
}

impl From<EventRejection> for TestInputError {
    fn from(error: EventRejection) -> Self {
        Self::Control(error)
    }
}

impl From<BoundaryFull> for TestInputError {
    fn from(error: BoundaryFull) -> Self {
        Self::AudioBoundaryFull(error)
    }
}

/// Installs and advances the one automatic MIDI fixture.
///
/// The service owns only fixture-specific input state. The composition root
/// retains ownership of the installed capability providers and the production
/// AppLoop so every fixture config is created through its selected provider
/// while every fixture event still uses the same reducer path.
pub struct AutomaticMidiTest<Source> {
    source: Source,
    patch_ids: Vec<(usize, PatchId)>,
    events: FixedEventBatch,
    initialized: bool,
    started: bool,
}

fn exact_provider_descriptors(
    providers: &[Box<dyn InstrumentCapabilityProvider>],
    registry: &CapabilityRegistry,
) -> Result<Vec<CapabilityDescriptor>, CapabilityError> {
    if providers.is_empty() {
        let capability_id = registry
            .descriptors()
            .first()
            .ok_or(CapabilityError::EmptyRegistry)?
            .id()
            .clone();
        return Err(CapabilityError::ProviderRegistryMismatch(capability_id));
    }

    let descriptors = providers
        .iter()
        .map(|provider| provider.descriptor())
        .collect::<Vec<_>>();
    if descriptors == registry.descriptors() {
        return Ok(descriptors);
    }

    let capability_id = descriptors
        .iter()
        .zip(registry.descriptors())
        .find_map(|(provider, registered)| (provider != registered).then(|| provider.id().clone()))
        .or_else(|| {
            descriptors
                .get(registry.descriptors().len())
                .map(|descriptor| descriptor.id().clone())
        })
        .or_else(|| {
            registry
                .descriptors()
                .get(descriptors.len())
                .map(|descriptor| descriptor.id().clone())
        })
        .expect("a nonempty registry or provider list has a mismatched descriptor");
    Err(CapabilityError::ProviderRegistryMismatch(capability_id))
}

impl<Source> AutomaticMidiTest<Source>
where
    Source: MidiEventSource,
{
    /// Creates an uninitialized automatic test around a replaceable input port.
    pub fn new(source: Source) -> Self {
        Self {
            source,
            patch_ids: Vec::new(),
            events: FixedEventBatch::new(),
            initialized: false,
            started: false,
        }
    }

    /// Discovers every fixture Patch and installs it through AppLoop without
    /// configuring a sound engine or starting the event source.
    pub fn initialize<Boundary>(
        &mut self,
        providers: &[Box<dyn InstrumentCapabilityProvider>],
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<(), TestInputError>
    where
        Boundary: ControlAudioBoundary,
    {
        self.initialize_with_effects(providers, &[], app_loop)
    }

    /// Installs the production fixture with one descriptor-built effect on its first Patch.
    pub fn initialize_with_effects<Boundary>(
        &mut self,
        providers: &[Box<dyn InstrumentCapabilityProvider>],
        effect_providers: &[Box<dyn EffectCapabilityProvider>],
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<(), TestInputError>
    where
        Boundary: ControlAudioBoundary,
    {
        if self.initialized {
            return Err(TestInputError::AlreadyInitialized);
        }

        let descriptors = exact_provider_descriptors(providers, app_loop.capabilities())?;
        let effect_descriptors =
            exact_effect_provider_descriptors(effect_providers, app_loop.effects())?;

        let parts = self.source.prepare()?;
        let mut patch_ids = Vec::new();
        let mut patches = Vec::new();
        patch_ids.try_reserve_exact(parts.len()).map_err(|_| {
            TestInputError::PatchIdentityOverflow {
                position: parts.len(),
            }
        })?;
        patches.try_reserve_exact(parts.len()).map_err(|_| {
            TestInputError::PatchIdentityOverflow {
                position: parts.len(),
            }
        })?;

        for (position, part) in parts.into_iter().enumerate() {
            if patch_ids
                .iter()
                .any(|(part_index, _)| *part_index == part.index())
            {
                return Err(TestInputError::DuplicatePartIndex {
                    part_index: part.index(),
                });
            }

            let patch_number = position
                .checked_add(1)
                .ok_or(TestInputError::PatchIdentityOverflow { position })?;
            let patch_number = u32::try_from(patch_number)
                .map_err(|_| TestInputError::PatchIdentityOverflow { position })?;
            let patch_id = PatchId::new(patch_number)
                .expect("a one-based fixture position always produces a non-zero PatchId");
            let provider_index = position % providers.len();
            let provider = providers[provider_index].as_ref();
            let descriptor = &descriptors[provider_index];
            let instrument_config = create_fixture_config(provider, descriptor, part.instrument())?;
            if instrument_config.capability_id() != descriptor.id() {
                return Err(
                    CapabilityError::ProviderRegistryMismatch(descriptor.id().clone()).into(),
                );
            }
            app_loop
                .capabilities()
                .validate_config(&instrument_config)?;
            let mut patch = Patch::new(
                patch_id,
                part.name().to_owned(),
                instrument_config,
                part.assigned_channel(),
                PatchOutput::to_track(
                    MixerTrackId::new((position % MixerTrackId::COUNT) as u8)
                        .expect("fixture position maps to a fixed mixer track"),
                ),
            );
            if position == 0 {
                if let (Some(provider), Some(descriptor)) =
                    (effect_providers.first(), effect_descriptors.first())
                {
                    let effect = create_default_effect_config(
                        provider.as_ref(),
                        descriptor,
                        EffectSlotId::new(1).expect("the production fixture slot is nonzero"),
                    )?;
                    patch = patch.with_post_effects(vec![effect]);
                }
            }

            patch_ids.push((part.index(), patch_id));
            patches.push(patch);
        }

        app_loop.dispatch_from(AppEvent::InstallPatches(patches), EventSource::Startup)?;
        self.patch_ids = patch_ids;
        self.events.clear();
        self.initialized = true;
        Ok(())
    }

    /// Starts automatic MIDI only after the composition root has prepared and
    /// installed complete audio graph ownership.
    pub fn start(&mut self) -> Result<(), TestInputError> {
        if !self.initialized {
            return Err(TestInputError::NotInitialized);
        }
        if self.started {
            return Err(TestInputError::AlreadyStarted);
        }
        self.source.start();
        self.started = true;
        Ok(())
    }

    /// Polls due fixture data into reusable bounded storage and sends every
    /// message through the production AppLoop.
    pub fn tick<Boundary>(
        &mut self,
        elapsed: Duration,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<(), TestInputError>
    where
        Boundary: ControlAudioBoundary,
    {
        if !self.initialized {
            return Err(TestInputError::NotInitialized);
        }
        if !self.started {
            return Err(TestInputError::NotStarted);
        }

        self.events.clear();
        self.source.poll(elapsed, &mut self.events)?;

        for event in self.events.iter().copied() {
            let patch_id = self
                .patch_ids
                .iter()
                .find_map(|(part_index, patch_id)| {
                    (*part_index == event.part_index()).then_some(*patch_id)
                })
                .ok_or(TestInputError::UnknownPartIndex {
                    part_index: event.part_index(),
                })?;

            let result = app_loop.dispatch_from(
                AppEvent::Midi {
                    patch_id,
                    message: event.message(),
                },
                EventSource::AutomaticMidi,
            )?;
            if let Some(boundary_full) = result.boundary_full() {
                return Err(boundary_full.into());
            }
        }

        Ok(())
    }
}

fn exact_effect_provider_descriptors(
    providers: &[Box<dyn EffectCapabilityProvider>],
    registry: &crate::synth::EffectCapabilityRegistry,
) -> Result<Vec<EffectCapabilityDescriptor>, EffectCapabilityError> {
    let descriptors = providers
        .iter()
        .map(|provider| provider.descriptor())
        .collect::<Vec<_>>();
    if descriptors == registry.descriptors() {
        return Ok(descriptors);
    }
    let id = descriptors
        .first()
        .map(|descriptor| descriptor.id().clone())
        .or_else(|| {
            registry
                .descriptors()
                .first()
                .map(|descriptor| descriptor.id().clone())
        })
        .unwrap_or_else(|| {
            crate::synth::EffectCapabilityId::new("effect.registry-mismatch")
                .expect("static mismatch id is valid")
        });
    Err(EffectCapabilityError::ProviderRegistryMismatch(id))
}

fn create_default_effect_config(
    provider: &dyn EffectCapabilityProvider,
    descriptor: &EffectCapabilityDescriptor,
    slot_id: EffectSlotId,
) -> Result<crate::synth::PostEffectConfig, EffectCapabilityError> {
    let mut values = Vec::new();
    let mut assets = Vec::new();
    for spec in descriptor.parameters() {
        match spec.default_value() {
            ParameterDefault::Value(value) => {
                values.push(ParameterAssignment::new(spec.id().clone(), value.clone()))
            }
            ParameterDefault::Asset(reference) => {
                assets.push(AssetAssignment::new(spec.id().clone(), reference.clone()))
            }
        }
    }
    provider.create_config(slot_id, &values, &assets)
}

/// Creates one fixture config through the selected provider, overriding only
/// the legacy source identity fields that its descriptor actually declares.
/// This keeps discovery-order alternation provider-backed and fallback-free.
fn create_fixture_config(
    provider: &dyn InstrumentCapabilityProvider,
    descriptor: &CapabilityDescriptor,
    instrument: SoundFontInstrument,
) -> Result<InstrumentConfig, CapabilityError> {
    let mut values = Vec::with_capacity(descriptor.parameters().count());
    let mut assets = Vec::with_capacity(descriptor.asset_requirements().len());

    for spec in descriptor.parameters() {
        match spec.default_value() {
            ParameterDefault::Asset(reference) => {
                assets.push(AssetAssignment::new(spec.id().clone(), reference.clone()))
            }
            ParameterDefault::Value(default) => {
                let value = if descriptor.id().as_str() == HIDEF_CAPABILITY_ID
                    && spec.id().as_str() == SOUNDFONT_PRESET_PARAMETER_ID
                {
                    ParameterValue::Choice(instrument.preset_id().choice_id())
                } else {
                    default.clone()
                };
                values.push(ParameterAssignment::new(spec.id().clone(), value));
            }
        }
    }
    provider.create_config(&values, &assets)
}

/// Translates the fixed MIDI fixture's source identity into generic assignments.
///
/// The provider boundary remains generic; this narrow testing service owns all
/// knowledge of the legacy SoundFont fixture shape.
pub fn create_soundfont_config<Provider>(
    provider: &Provider,
    instrument: SoundFontInstrument,
) -> Result<InstrumentConfig, CapabilityError>
where
    Provider: InstrumentCapabilityProvider,
{
    let descriptor = provider.descriptor();
    if descriptor.id().as_str() != HIDEF_CAPABILITY_ID {
        return Err(CapabilityError::ProviderRegistryMismatch(
            descriptor.id().clone(),
        ));
    }
    let parameter_id = |value: &str| {
        ParameterId::new(value)
            .map_err(|_| CapabilityError::InvalidMetadataIdentifier(value.to_owned()))
    };
    provider.create_config(
        &[ParameterAssignment::new(
            parameter_id(SOUNDFONT_PRESET_PARAMETER_ID)?,
            ParameterValue::Choice(instrument.preset_id().choice_id()),
        )],
        &[AssetAssignment::new(
            parameter_id(SOUNDFONT_FILE_PARAMETER_ID)?,
            AssetReference::new(AssetKind::SoundFont, HIDEF_SOUNDFONT_PATH)?,
        )],
    )
}

#[cfg(test)]
mod tests {
    use super::{AutomaticMidiTest, TestInputError};
    use crate::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
    use crate::adapter::hidef_soundfont_capability::SOUNDFONT_PRESET_PARAMETER_ID;
    use crate::control::app_loop::AppLoop;
    use crate::control::app_state::AppState;
    use crate::control::event_record::EventSource;
    use crate::control::state_projector::StateProjector;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{
        AssetAssignment, CapabilityDescriptor, CapabilityError, CapabilityRegistry,
        InstrumentCapabilityProvider, InstrumentConfig, ParameterAssignment, ParameterId,
        ParameterValue,
    };
    use crate::testing::instrument_part::InstrumentPart;
    use crate::testing::midi_event_source::{
        FixedEventBatch, MidiEventSource, MidiSourceError, TargetedMidiEvent,
    };
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    type Providers = Vec<Box<dyn InstrumentCapabilityProvider>>;

    struct TestSource {
        parts: Vec<InstrumentPart>,
        due: Vec<TargetedMidiEvent>,
        started: bool,
    }

    impl MidiEventSource for TestSource {
        fn prepare(&mut self) -> Result<Vec<InstrumentPart>, MidiSourceError> {
            Ok(self.parts.clone())
        }

        fn start(&mut self) {
            self.started = true;
        }

        fn poll(
            &mut self,
            _elapsed: Duration,
            output: &mut FixedEventBatch,
        ) -> Result<(), MidiSourceError> {
            if !self.started {
                return Err(MidiSourceError::new("source was not started"));
            }
            for event in self.due.drain(..) {
                output.try_push(event)?;
            }
            Ok(())
        }

        fn finished(&self) -> bool {
            self.started && self.due.is_empty()
        }
    }

    #[derive(Default)]
    struct Observations {
        parameters: Option<ParameterSnapshot>,
        commands: Vec<AudioCommand>,
    }

    struct TestBoundary {
        observations: Arc<Mutex<Observations>>,
    }

    impl ControlAudioBoundary for TestBoundary {
        fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
            self.observations.lock().unwrap().commands.push(command);
            Ok(())
        }

        fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
            self.observations.lock().unwrap().parameters = Some(parameters);
        }
    }

    fn part(index: usize, name: &str, program: u8) -> InstrumentPart {
        InstrumentPart::new(
            index,
            name.to_owned(),
            SoundFontInstrument::new(0, program, false).unwrap(),
        )
    }

    fn message(channel: u8, note: u8) -> MidiMessage {
        MidiMessage::try_new(
            MidiChannel::new(channel).unwrap(),
            MidiMessageKind::NoteOn,
            note,
            100,
        )
        .unwrap()
    }

    fn source(parts: Vec<InstrumentPart>, due: Vec<TargetedMidiEvent>) -> TestSource {
        TestSource {
            parts,
            due,
            started: false,
        }
    }

    fn app_loop_for_registry(
        registry: CapabilityRegistry,
    ) -> (AppLoop<TestBoundary>, Arc<Mutex<Observations>>) {
        let global = GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap();
        let observations = Arc::new(Mutex::new(Observations::default()));
        let app_loop = AppLoop::new(
            AppState::new(registry, global),
            StateProjector::new(),
            TestBoundary {
                observations: Arc::clone(&observations),
            },
        )
        .unwrap();
        (app_loop, observations)
    }

    fn app_loop() -> (AppLoop<TestBoundary>, Arc<Mutex<Observations>>, Providers) {
        let providers: Providers = vec![Box::new(
            crate::adapter::production_instruments::production_soundfont_capability().unwrap(),
        )];
        let registry = CapabilityRegistry::new(
            providers
                .iter()
                .map(|provider| provider.descriptor())
                .collect(),
        )
        .unwrap();
        let (app_loop, observations) = app_loop_for_registry(registry);
        (app_loop, observations, providers)
    }

    fn mixed_app_loop() -> (AppLoop<TestBoundary>, CapabilityRegistry, Providers) {
        let providers: Providers = vec![
            Box::new(
                crate::adapter::production_instruments::production_soundfont_capability().unwrap(),
            ),
            Box::new(BraidsCapability::new().unwrap()),
        ];
        let registry = CapabilityRegistry::new(
            providers
                .iter()
                .map(|provider| provider.descriptor())
                .collect(),
        )
        .unwrap();
        let (app_loop, _) = app_loop_for_registry(registry.clone());
        (app_loop, registry, providers)
    }

    struct FailingConfigProvider {
        descriptor: CapabilityDescriptor,
    }

    impl InstrumentCapabilityProvider for FailingConfigProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            self.descriptor.clone()
        }

        fn create_config(
            &self,
            _values: &[ParameterAssignment],
            _asset_references: &[AssetAssignment],
        ) -> Result<InstrumentConfig, CapabilityError> {
            Err(CapabilityError::UnknownChoice(
                ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap(),
            ))
        }
    }

    #[test]
    fn initialize_installs_one_default_patch_per_part_without_starting() {
        let source = source(
            vec![part(4, "Piano", 0), part(12, "Strings", 48)],
            Vec::new(),
        );
        let mut service = AutomaticMidiTest::new(source);
        let (mut app_loop, observations, providers) = app_loop();

        service.initialize(&providers, &mut app_loop).unwrap();

        let event_log = app_loop.event_log();
        assert_eq!(event_log.records().len(), 1);
        assert_eq!(event_log.records()[0].source(), EventSource::Startup);
        assert!(!service.source.started);

        let observations = observations.lock().unwrap();
        let parameters = observations.parameters.as_ref().unwrap();
        assert_eq!(parameters.generation(), 1);
        assert_eq!(parameters.patches().len(), 2);
        assert!(app_loop.current_text().body().contains(":Piano"));
        assert!(app_loop.current_text().body().contains(":Strings"));
    }

    #[test]
    fn initialize_alternates_exact_soundfont_and_default_braids_configs_in_discovery_order() {
        let source = source(
            vec![
                part(0, "Piano", 0),
                part(1, "Bass", 32),
                part(2, "Strings", 48),
                part(3, "Lead", 80),
            ],
            Vec::new(),
        );
        let mut service = AutomaticMidiTest::new(source);
        let (mut app_loop, registry, providers) = mixed_app_loop();
        service.initialize(&providers, &mut app_loop).unwrap();

        assert_eq!(
            app_loop
                .patches()
                .iter()
                .map(|patch| patch.instrument_config().capability_id().as_str())
                .collect::<Vec<_>>(),
            [
                crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID,
                BRAIDS_CAPABILITY_ID,
                crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID,
                BRAIDS_CAPABILITY_ID,
            ]
        );
        for patch in app_loop.patches() {
            registry.validate_config(patch.instrument_config()).unwrap();
        }
        assert_eq!(
            app_loop.patches()[0]
                .instrument_config()
                .value(&ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Choice(
                SoundFontInstrument::new(0, 0, false)
                    .unwrap()
                    .preset_id()
                    .choice_id()
            ))
        );
        assert_eq!(
            app_loop.patches()[2]
                .instrument_config()
                .value(&ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Choice(
                SoundFontInstrument::new(0, 48, false)
                    .unwrap()
                    .preset_id()
                    .choice_id()
            ))
        );
        let braids_default = BraidsCapability::new().unwrap().default_config().unwrap();
        assert_eq!(app_loop.patches()[1].instrument_config(), &braids_default);
        assert_eq!(app_loop.patches()[3].instrument_config(), &braids_default);
    }

    #[test]
    fn initialize_rejects_provider_conversion_failure_atomically_without_substitution() {
        let descriptor = crate::adapter::production_instruments::production_soundfont_capability()
            .unwrap()
            .descriptor();
        let registry = CapabilityRegistry::new(vec![descriptor.clone()]).unwrap();
        let providers: Providers = vec![Box::new(FailingConfigProvider { descriptor })];
        let (mut app_loop, observations) = app_loop_for_registry(registry);
        let mut service = AutomaticMidiTest::new(source(vec![part(0, "Piano", 48)], Vec::new()));

        let error = service.initialize(&providers, &mut app_loop).unwrap_err();

        assert_eq!(
            error,
            TestInputError::Capability(CapabilityError::UnknownChoice(
                ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap()
            ))
        );
        assert!(app_loop.patches().is_empty());
        assert!(app_loop.event_log().records().is_empty());
        assert!(!service.source.started);
        let observations = observations.lock().unwrap();
        assert!(observations.commands.is_empty());
        assert_eq!(
            observations
                .parameters
                .as_ref()
                .map(ParameterSnapshot::generation),
            Some(0)
        );
    }

    #[test]
    fn initialize_rejects_provider_registry_mismatch_before_patch_installation() {
        let registry = crate::adapter::production_instruments::production_soundfont_capability()
            .unwrap()
            .registry()
            .unwrap();
        let providers: Providers = vec![Box::new(BraidsCapability::new().unwrap())];
        let mismatched_capability = providers[0].descriptor().id().clone();
        let (mut app_loop, observations) = app_loop_for_registry(registry);
        let mut service = AutomaticMidiTest::new(source(vec![part(0, "Piano", 0)], Vec::new()));

        let error = service.initialize(&providers, &mut app_loop).unwrap_err();

        assert_eq!(
            error,
            TestInputError::Capability(CapabilityError::ProviderRegistryMismatch(
                mismatched_capability
            ))
        );
        assert!(app_loop.patches().is_empty());
        assert!(app_loop.event_log().records().is_empty());
        assert!(!service.source.started);
        let observations = observations.lock().unwrap();
        assert!(observations.commands.is_empty());
        assert_eq!(
            observations
                .parameters
                .as_ref()
                .map(ParameterSnapshot::generation),
            Some(0)
        );
    }

    #[test]
    fn initialize_rejects_missing_provider_before_patch_installation() {
        let registry = crate::adapter::production_instruments::production_soundfont_capability()
            .unwrap()
            .registry()
            .unwrap();
        let missing_capability = registry.descriptors()[0].id().clone();
        let providers: Providers = Vec::new();
        let (mut app_loop, observations) = app_loop_for_registry(registry);
        let mut service = AutomaticMidiTest::new(source(vec![part(0, "Piano", 0)], Vec::new()));

        let error = service.initialize(&providers, &mut app_loop).unwrap_err();

        assert_eq!(
            error,
            TestInputError::Capability(CapabilityError::ProviderRegistryMismatch(
                missing_capability
            ))
        );
        assert!(app_loop.patches().is_empty());
        assert!(app_loop.event_log().records().is_empty());
        assert!(!service.source.started);
        let observations = observations.lock().unwrap();
        assert!(observations.commands.is_empty());
        assert_eq!(
            observations
                .parameters
                .as_ref()
                .map(ParameterSnapshot::generation),
            Some(0)
        );
    }

    #[test]
    fn explicit_start_is_required_once_after_patch_installation() {
        let mut service = AutomaticMidiTest::new(source(vec![part(4, "Piano", 0)], Vec::new()));
        let (mut app_loop, _, providers) = app_loop();

        assert_eq!(service.start(), Err(TestInputError::NotInitialized));
        service.initialize(&providers, &mut app_loop).unwrap();
        assert_eq!(
            service.tick(Duration::from_millis(1), &mut app_loop),
            Err(TestInputError::NotStarted)
        );
        service.start().unwrap();
        assert!(service.source.started);
        assert_eq!(service.start(), Err(TestInputError::AlreadyStarted));
    }

    #[test]
    fn tick_maps_part_identity_and_dispatches_through_app_loop() {
        let due = TargetedMidiEvent::new(12, message(12, 64));
        let source = source(
            vec![part(4, "Piano", 0), part(12, "Strings", 48)],
            vec![due],
        );
        let mut service = AutomaticMidiTest::new(source);
        let (mut app_loop, observations, providers) = app_loop();
        service.initialize(&providers, &mut app_loop).unwrap();
        service.start().unwrap();

        service
            .tick(Duration::from_millis(10), &mut app_loop)
            .unwrap();

        let event_log = app_loop.event_log();
        assert_eq!(event_log.records().len(), 2);
        assert_eq!(event_log.records()[0].source(), EventSource::Startup);
        assert_eq!(event_log.records()[1].source(), EventSource::AutomaticMidi);

        let observations = observations.lock().unwrap();
        assert_eq!(
            observations.commands,
            vec![AudioCommand::patch_midi(
                PatchId::new(2).unwrap(),
                due.message()
            )]
        );
        assert_eq!(observations.parameters.as_ref().unwrap().generation(), 2);
    }

    #[test]
    fn duplicate_part_indexes_are_rejected_before_installation_or_start() {
        let source = source(
            vec![part(7, "First", 1), part(7, "Duplicate", 2)],
            Vec::new(),
        );
        let mut service = AutomaticMidiTest::new(source);
        let (mut app_loop, _, providers) = app_loop();

        let error = service.initialize(&providers, &mut app_loop).unwrap_err();

        assert_eq!(error, TestInputError::DuplicatePartIndex { part_index: 7 });
        assert!(!service.source.started);
    }

    #[test]
    fn tick_rejects_unknown_part_targets_without_bypassing_app_loop() {
        let due = TargetedMidiEvent::new(99, message(0, 60));
        let source = source(vec![part(0, "Piano", 0)], vec![due]);
        let mut service = AutomaticMidiTest::new(source);
        let (mut app_loop, observations, providers) = app_loop();
        service.initialize(&providers, &mut app_loop).unwrap();
        service.start().unwrap();

        let error = service
            .tick(Duration::from_millis(1), &mut app_loop)
            .unwrap_err();

        assert_eq!(error, TestInputError::UnknownPartIndex { part_index: 99 });
        assert!(observations.lock().unwrap().commands.is_empty());
    }
}
