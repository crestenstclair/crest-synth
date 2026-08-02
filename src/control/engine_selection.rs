use crate::kernel::patch_id::PatchId;
use crate::mixer::bus_id::BusId;
use crate::real_time::graph_revision::GraphRevision;
use crate::synth::capability_id::CapabilityId;
use crate::synth::effect_slot_id::EffectSlotIndex;
use crate::synth::{EffectCapabilityId, ParameterId};
use core::fmt;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

/// Monotonic correlation identity for one engine-selection attempt.
///
/// Zero is reserved for the absence of a request. Every identity stored in an
/// active correlation is nonzero.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct EngineSelectionRequestId(u64);

impl EngineSelectionRequestId {
    pub const NONE: Self = Self(0);
    pub const FIRST: Self = Self(1);

    pub const fn new(value: u64) -> Result<Self, EngineSelectionRequestIdError> {
        if value == 0 {
            Err(EngineSelectionRequestIdError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    pub const fn checked_next(self) -> Result<Self, EngineSelectionRequestIdError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(EngineSelectionRequestIdError::Exhausted),
        }
    }
}

impl fmt::Display for EngineSelectionRequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineSelectionRequestIdError {
    Zero,
    Exhausted,
}

impl fmt::Display for EngineSelectionRequestIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("engine-selection request id must be nonzero"),
            Self::Exhausted => {
                formatter.write_str("engine-selection request id range is exhausted")
            }
        }
    }
}

impl std::error::Error for EngineSelectionRequestIdError {}

/// Stable visible reasons an engine candidate could not be prepared.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineSelectionFailure {
    UnknownCapability,
    MissingDefault,
    InvalidDefaultConfig,
    ProviderMismatch,
    PreparerMissing,
    AssetUnavailable,
    PresetUnavailable,
    UnsupportedAudioConfig,
    PreparationFailed,
    GraphIncompatible,
    WorkerUnavailable,
}

impl EngineSelectionFailure {
    pub const ALL: [Self; 11] = [
        Self::UnknownCapability,
        Self::MissingDefault,
        Self::InvalidDefaultConfig,
        Self::ProviderMismatch,
        Self::PreparerMissing,
        Self::AssetUnavailable,
        Self::PresetUnavailable,
        Self::UnsupportedAudioConfig,
        Self::PreparationFailed,
        Self::GraphIncompatible,
        Self::WorkerUnavailable,
    ];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::UnknownCapability => "unknownCapability",
            Self::MissingDefault => "missingDefault",
            Self::InvalidDefaultConfig => "invalidDefaultConfig",
            Self::ProviderMismatch => "providerMismatch",
            Self::PreparerMissing => "preparerMissing",
            Self::AssetUnavailable => "assetUnavailable",
            Self::PresetUnavailable => "presetUnavailable",
            Self::UnsupportedAudioConfig => "unsupportedAudioConfig",
            Self::PreparationFailed => "preparationFailed",
            Self::GraphIncompatible => "graphIncompatible",
            Self::WorkerUnavailable => "workerUnavailable",
        }
    }
}

/// The complete permitted config delta for one shared structural request.
///
/// The two occupancy variants are the topology deltas of this mission: they
/// change what exists at one exact position — a Patch effect slot or a bus
/// return — carrying an optional registry entry where `None` clears. The
/// vocabulary stays generic: positions and registry identities, never effect
/// names.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StructuralEditIntent {
    ReplaceCapability {
        target_capability_id: CapabilityId,
    },
    ReplaceParameterChoice {
        capability_id: CapabilityId,
        parameter_id: ParameterId,
        choice_id: String,
    },
    SetSlotOccupancy {
        patch_id: PatchId,
        slot: EffectSlotIndex,
        entry: Option<EffectCapabilityId>,
    },
    SetReturnOccupancy {
        bus: BusId,
        entry: Option<EffectCapabilityId>,
    },
}

impl StructuralEditIntent {
    pub const fn target_capability_id(&self) -> Option<&CapabilityId> {
        match self {
            Self::ReplaceCapability {
                target_capability_id,
            } => Some(target_capability_id),
            Self::ReplaceParameterChoice { capability_id, .. } => Some(capability_id),
            Self::SetSlotOccupancy { .. } | Self::SetReturnOccupancy { .. } => None,
        }
    }

    pub const fn parameter_id(&self) -> Option<&ParameterId> {
        match self {
            Self::ReplaceParameterChoice { parameter_id, .. } => Some(parameter_id),
            _ => None,
        }
    }

    pub fn choice_id(&self) -> Option<&str> {
        match self {
            Self::ReplaceParameterChoice { choice_id, .. } => Some(choice_id),
            _ => None,
        }
    }

    /// Reports whether this intent is a slot or return occupancy change.
    pub const fn is_occupancy(&self) -> bool {
        matches!(
            self,
            Self::SetSlotOccupancy { .. } | Self::SetReturnOccupancy { .. }
        )
    }

    /// Validates one correlation's Patch and capability context against this
    /// intent. Instrument intents require the full capability pair; occupancy
    /// intents carry their position themselves and require exactly the Patch
    /// context they name — nothing weaker and nothing fabricated.
    pub(crate) fn matches_context(
        &self,
        patch_id: Option<PatchId>,
        source: Option<&CapabilityId>,
        target: Option<&CapabilityId>,
    ) -> bool {
        match self {
            Self::ReplaceCapability {
                target_capability_id,
            } => match (patch_id, source, target) {
                (Some(_), Some(source), Some(target)) => {
                    source != target && target_capability_id == target
                }
                _ => false,
            },
            Self::ReplaceParameterChoice { capability_id, .. } => {
                match (patch_id, source, target) {
                    (Some(_), Some(source), Some(target)) => {
                        source == target && capability_id == source
                    }
                    _ => false,
                }
            }
            Self::SetSlotOccupancy {
                patch_id: intent_patch_id,
                ..
            } => patch_id == Some(*intent_patch_id) && source.is_none() && target.is_none(),
            Self::SetReturnOccupancy { .. } => {
                patch_id.is_none() && source.is_none() && target.is_none()
            }
        }
    }
}

/// Closed lifecycle kinds projected from canonical reducer state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineSelectionStatusKind {
    Ready,
    Preparing,
    Activating,
    Failed,
}

/// Stable structural-effect kinds emitted after accepted lifecycle steps.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineSelectionEffectKind {
    PrepareRequested,
    CandidateCommitted,
    GraphStaged,
    GraphPublished,
    ActivationAcknowledged,
}

impl EngineSelectionEffectKind {
    pub const ALL: [Self; 5] = [
        Self::PrepareRequested,
        Self::CandidateCommitted,
        Self::GraphStaged,
        Self::GraphPublished,
        Self::ActivationAcknowledged,
    ];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::PrepareRequested => "prepareRequested",
            Self::CandidateCommitted => "candidateCommitted",
            Self::GraphStaged => "graphStaged",
            Self::GraphPublished => "graphPublished",
            Self::ActivationAcknowledged => "activationAcknowledged",
        }
    }
}

impl EngineSelectionStatusKind {
    pub const ALL: [Self; 4] = [Self::Ready, Self::Preparing, Self::Activating, Self::Failed];

    pub const fn surface_descriptor() -> &'static [Self] {
        &Self::ALL
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Preparing => "preparing",
            Self::Activating => "activating",
            Self::Failed => "failed",
        }
    }
}

/// Stable control metadata shared by preparation, activation, and evidence.
///
/// Instrument intents carry the full Patch and capability context; occupancy
/// intents carry their position in the intent itself — a return edit has no
/// Patch, so its context fields stay absent rather than fabricated.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSelectionCorrelation {
    request_id: EngineSelectionRequestId,
    patch_id: Option<PatchId>,
    intent: StructuralEditIntent,
    source_capability_id: Option<CapabilityId>,
    target_capability_id: Option<CapabilityId>,
    source_graph_revision: GraphRevision,
    target_graph_revision: Option<GraphRevision>,
}

impl EngineSelectionCorrelation {
    pub const fn request_id(&self) -> EngineSelectionRequestId {
        self.request_id
    }

    pub const fn patch_id(&self) -> Option<PatchId> {
        self.patch_id
    }

    pub const fn intent(&self) -> &StructuralEditIntent {
        &self.intent
    }

    pub const fn source_capability_id(&self) -> Option<&CapabilityId> {
        self.source_capability_id.as_ref()
    }

    pub const fn target_capability_id(&self) -> Option<&CapabilityId> {
        self.target_capability_id.as_ref()
    }

    pub const fn source_graph_revision(&self) -> GraphRevision {
        self.source_graph_revision
    }

    pub const fn target_graph_revision(&self) -> Option<GraphRevision> {
        self.target_graph_revision
    }
}

/// One observable control-side structural effect with no graph ownership.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSelectionEffect {
    kind: EngineSelectionEffectKind,
    request_id: EngineSelectionRequestId,
    patch_id: Option<PatchId>,
    intent: StructuralEditIntent,
    source_capability_id: Option<CapabilityId>,
    target_capability_id: Option<CapabilityId>,
    source_graph_revision: GraphRevision,
    target_graph_revision: Option<GraphRevision>,
}

impl EngineSelectionEffect {
    pub fn from_correlation(
        kind: EngineSelectionEffectKind,
        correlation: &EngineSelectionCorrelation,
    ) -> Result<Self, EngineSelectionStatusError> {
        let target_graph_revision = correlation.target_graph_revision();
        match kind {
            EngineSelectionEffectKind::PrepareRequested if target_graph_revision.is_some() => {
                return Err(EngineSelectionStatusError::UnexpectedTargetRevision);
            }
            EngineSelectionEffectKind::PrepareRequested => {}
            _ if target_graph_revision.is_none() => {
                return Err(EngineSelectionStatusError::MissingTargetRevision);
            }
            _ => {}
        }
        Ok(Self {
            kind,
            request_id: correlation.request_id(),
            patch_id: correlation.patch_id(),
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().cloned(),
            target_capability_id: correlation.target_capability_id().cloned(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision,
        })
    }

    pub const fn kind(&self) -> EngineSelectionEffectKind {
        self.kind
    }

    pub const fn request_id(&self) -> EngineSelectionRequestId {
        self.request_id
    }

    pub const fn patch_id(&self) -> Option<PatchId> {
        self.patch_id
    }

    pub const fn intent(&self) -> &StructuralEditIntent {
        &self.intent
    }

    pub const fn source_capability_id(&self) -> Option<&CapabilityId> {
        self.source_capability_id.as_ref()
    }

    pub const fn target_capability_id(&self) -> Option<&CapabilityId> {
        self.target_capability_id.as_ref()
    }

    pub const fn source_graph_revision(&self) -> GraphRevision {
        self.source_graph_revision
    }

    pub const fn target_graph_revision(&self) -> Option<GraphRevision> {
        self.target_graph_revision
    }
}

/// Reducer-owned state for the single app-wide structural selection lifecycle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSelectionStatus {
    kind: EngineSelectionStatusKind,
    active_graph_revision: GraphRevision,
    correlation: Option<EngineSelectionCorrelation>,
    failure: Option<EngineSelectionFailure>,
}

impl EngineSelectionStatus {
    pub const fn ready(active_graph_revision: GraphRevision) -> Self {
        Self {
            kind: EngineSelectionStatusKind::Ready,
            active_graph_revision,
            correlation: None,
            failure: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn preparing(
        active_graph_revision: GraphRevision,
        request_id: EngineSelectionRequestId,
        patch_id: PatchId,
        source_capability_id: CapabilityId,
        target_capability_id: CapabilityId,
    ) -> Result<Self, EngineSelectionStatusError> {
        let intent = StructuralEditIntent::ReplaceCapability {
            target_capability_id: target_capability_id.clone(),
        };
        Self::preparing_with_intent(
            active_graph_revision,
            request_id,
            patch_id,
            intent,
            source_capability_id,
            target_capability_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn preparing_with_intent(
        active_graph_revision: GraphRevision,
        request_id: EngineSelectionRequestId,
        patch_id: PatchId,
        intent: StructuralEditIntent,
        source_capability_id: CapabilityId,
        target_capability_id: CapabilityId,
    ) -> Result<Self, EngineSelectionStatusError> {
        Self::preparing_with_context(
            active_graph_revision,
            request_id,
            Some(patch_id),
            intent,
            Some(source_capability_id),
            Some(target_capability_id),
        )
    }

    /// Enters Preparing for one occupancy intent, deriving the Patch context
    /// the intent itself declares.
    pub fn preparing_for_occupancy(
        active_graph_revision: GraphRevision,
        request_id: EngineSelectionRequestId,
        intent: StructuralEditIntent,
    ) -> Result<Self, EngineSelectionStatusError> {
        let patch_id = match &intent {
            StructuralEditIntent::SetSlotOccupancy { patch_id, .. } => Some(*patch_id),
            StructuralEditIntent::SetReturnOccupancy { .. } => None,
            StructuralEditIntent::ReplaceCapability { .. }
            | StructuralEditIntent::ReplaceParameterChoice { .. } => {
                return Err(EngineSelectionStatusError::IntentMismatch)
            }
        };
        Self::preparing_with_context(
            active_graph_revision,
            request_id,
            patch_id,
            intent,
            None,
            None,
        )
    }

    fn preparing_with_context(
        active_graph_revision: GraphRevision,
        request_id: EngineSelectionRequestId,
        patch_id: Option<PatchId>,
        intent: StructuralEditIntent,
        source_capability_id: Option<CapabilityId>,
        target_capability_id: Option<CapabilityId>,
    ) -> Result<Self, EngineSelectionStatusError> {
        if request_id.is_none() {
            return Err(EngineSelectionStatusError::MissingRequestIdentity);
        }
        if !intent.matches_context(
            patch_id,
            source_capability_id.as_ref(),
            target_capability_id.as_ref(),
        ) {
            return Err(EngineSelectionStatusError::IntentMismatch);
        }
        Ok(Self {
            kind: EngineSelectionStatusKind::Preparing,
            active_graph_revision,
            correlation: Some(EngineSelectionCorrelation {
                request_id,
                patch_id,
                intent,
                source_capability_id,
                target_capability_id,
                source_graph_revision: active_graph_revision,
                target_graph_revision: None,
            }),
            failure: None,
        })
    }

    pub fn activating(
        &self,
        target_graph_revision: GraphRevision,
    ) -> Result<Self, EngineSelectionStatusError> {
        if self.kind != EngineSelectionStatusKind::Preparing {
            return Err(EngineSelectionStatusError::InvalidTransition);
        }
        if target_graph_revision <= self.active_graph_revision {
            return Err(EngineSelectionStatusError::TargetRevisionNotNewer);
        }
        let mut correlation = self
            .correlation
            .clone()
            .ok_or(EngineSelectionStatusError::MissingCorrelation)?;
        correlation.target_graph_revision = Some(target_graph_revision);
        Ok(Self {
            kind: EngineSelectionStatusKind::Activating,
            active_graph_revision: self.active_graph_revision,
            correlation: Some(correlation),
            failure: None,
        })
    }

    pub fn failed(
        &self,
        failure: EngineSelectionFailure,
    ) -> Result<Self, EngineSelectionStatusError> {
        if self.kind != EngineSelectionStatusKind::Preparing {
            return Err(EngineSelectionStatusError::InvalidTransition);
        }
        let correlation = self
            .correlation
            .clone()
            .ok_or(EngineSelectionStatusError::MissingCorrelation)?;
        Ok(Self {
            kind: EngineSelectionStatusKind::Failed,
            active_graph_revision: self.active_graph_revision,
            correlation: Some(correlation),
            failure: Some(failure),
        })
    }

    pub fn acknowledged(&self) -> Result<Self, EngineSelectionStatusError> {
        if self.kind != EngineSelectionStatusKind::Activating {
            return Err(EngineSelectionStatusError::InvalidTransition);
        }
        let target_graph_revision = self
            .correlation
            .as_ref()
            .and_then(EngineSelectionCorrelation::target_graph_revision)
            .ok_or(EngineSelectionStatusError::MissingTargetRevision)?;
        Ok(Self::ready(target_graph_revision))
    }

    pub const fn kind(&self) -> EngineSelectionStatusKind {
        self.kind
    }

    pub const fn active_graph_revision(&self) -> GraphRevision {
        self.active_graph_revision
    }

    pub const fn correlation(&self) -> Option<&EngineSelectionCorrelation> {
        self.correlation.as_ref()
    }

    pub const fn failure(&self) -> Option<EngineSelectionFailure> {
        self.failure
    }

    pub const fn is_in_flight(&self) -> bool {
        matches!(
            self.kind,
            EngineSelectionStatusKind::Preparing | EngineSelectionStatusKind::Activating
        )
    }

    /// Returns the complete graph revision targeted by current scalar projections.
    /// Activating state has committed the candidate config, so projections target
    /// its newer graph before callback acknowledgement advances the active revision.
    pub const fn projection_graph_revision(&self) -> GraphRevision {
        match self.kind {
            EngineSelectionStatusKind::Activating => match &self.correlation {
                Some(correlation) => match correlation.target_graph_revision {
                    Some(target) => target,
                    None => self.active_graph_revision,
                },
                None => self.active_graph_revision,
            },
            EngineSelectionStatusKind::Ready
            | EngineSelectionStatusKind::Preparing
            | EngineSelectionStatusKind::Failed => self.active_graph_revision,
        }
    }

    fn validate_serialized(&self) -> Result<(), EngineSelectionStatusError> {
        let correlation_valid = self.correlation.as_ref().is_none_or(|correlation| {
            !correlation.request_id.is_none()
                && correlation.source_graph_revision == self.active_graph_revision
                && correlation.intent.matches_context(
                    correlation.patch_id,
                    correlation.source_capability_id.as_ref(),
                    correlation.target_capability_id.as_ref(),
                )
        });
        if !correlation_valid {
            return Err(EngineSelectionStatusError::InvalidSerializedState);
        }
        let valid = match self.kind {
            EngineSelectionStatusKind::Ready => {
                self.correlation.is_none() && self.failure.is_none()
            }
            EngineSelectionStatusKind::Preparing => {
                self.correlation
                    .as_ref()
                    .is_some_and(|correlation| correlation.target_graph_revision.is_none())
                    && self.failure.is_none()
            }
            EngineSelectionStatusKind::Activating => {
                self.correlation
                    .as_ref()
                    .and_then(|correlation| correlation.target_graph_revision)
                    .is_some_and(|target| target > self.active_graph_revision)
                    && self.failure.is_none()
            }
            EngineSelectionStatusKind::Failed => {
                self.correlation
                    .as_ref()
                    .is_some_and(|correlation| correlation.target_graph_revision.is_none())
                    && self.failure.is_some()
            }
        };
        if valid {
            Ok(())
        } else {
            Err(EngineSelectionStatusError::InvalidSerializedState)
        }
    }
}

impl<'de> Deserialize<'de> for EngineSelectionStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializedStatus {
            kind: EngineSelectionStatusKind,
            active_graph_revision: GraphRevision,
            correlation: Option<EngineSelectionCorrelation>,
            failure: Option<EngineSelectionFailure>,
        }

        let serialized = SerializedStatus::deserialize(deserializer)?;
        let status = Self {
            kind: serialized.kind,
            active_graph_revision: serialized.active_graph_revision,
            correlation: serialized.correlation,
            failure: serialized.failure,
        };
        status.validate_serialized().map_err(D::Error::custom)?;
        Ok(status)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineSelectionStatusError {
    MissingRequestIdentity,
    MissingCorrelation,
    MissingTargetRevision,
    TargetRevisionNotNewer,
    UnexpectedTargetRevision,
    InvalidSerializedState,
    InvalidTransition,
    IntentMismatch,
}

impl fmt::Display for EngineSelectionStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingRequestIdentity => "engine-selection correlation requires a request id",
            Self::MissingCorrelation => "engine-selection transition requires correlation metadata",
            Self::MissingTargetRevision => {
                "engine-selection activation requires a target graph revision"
            }
            Self::TargetRevisionNotNewer => {
                "engine-selection target graph revision must be newer than the source"
            }
            Self::UnexpectedTargetRevision => {
                "engine-selection preparation request must not name a target graph revision"
            }
            Self::InvalidSerializedState => {
                "serialized engine-selection lifecycle violates its correlation invariants"
            }
            Self::InvalidTransition => "engine-selection lifecycle transition is invalid",
            Self::IntentMismatch => {
                "structural edit intent does not match its source and target capabilities"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EngineSelectionStatusError {}

#[cfg(test)]
mod tests {
    use super::{
        EngineSelectionEffect, EngineSelectionEffectKind, EngineSelectionFailure,
        EngineSelectionRequestId, EngineSelectionRequestIdError, EngineSelectionStatus,
        EngineSelectionStatusError, EngineSelectionStatusKind,
    };
    use crate::kernel::PatchId;
    use crate::real_time::GraphRevision;
    use crate::synth::CapabilityId;

    fn capability_id(value: &str) -> CapabilityId {
        CapabilityId::new(value).unwrap()
    }

    fn preparing() -> EngineSelectionStatus {
        EngineSelectionStatus::preparing(
            GraphRevision::INITIAL,
            EngineSelectionRequestId::FIRST,
            PatchId::new(1).unwrap(),
            capability_id("instrument.source"),
            capability_id("instrument.target"),
        )
        .unwrap()
    }

    #[test]
    fn engine_selection_values_have_unique_exhaustive_stable_serialized_names() {
        assert_eq!(EngineSelectionFailure::surface_descriptor().len(), 11);
        assert_eq!(EngineSelectionStatusKind::surface_descriptor().len(), 4);
        for failures in EngineSelectionFailure::surface_descriptor().windows(2) {
            assert_ne!(failures[0], failures[1]);
        }
        for failure in EngineSelectionFailure::surface_descriptor() {
            assert_eq!(
                serde_json::to_string(failure).unwrap(),
                format!("\"{}\"", failure.name())
            );
        }
        for kind in EngineSelectionStatusKind::surface_descriptor() {
            assert_eq!(
                serde_json::to_string(kind).unwrap(),
                format!("\"{}\"", kind.name())
            );
        }
        for kind in EngineSelectionEffectKind::surface_descriptor() {
            assert_eq!(
                serde_json::to_string(kind).unwrap(),
                format!("\"{}\"", kind.name())
            );
        }
    }

    #[test]
    fn engine_selection_request_id_reserves_zero_and_reports_exhaustion() {
        assert!(EngineSelectionRequestId::NONE.is_none());
        assert_eq!(
            EngineSelectionRequestId::new(0),
            Err(EngineSelectionRequestIdError::Zero)
        );
        assert_eq!(
            EngineSelectionRequestId::NONE.checked_next().unwrap(),
            EngineSelectionRequestId::FIRST
        );
        assert_eq!(
            EngineSelectionRequestId::new(u64::MAX)
                .unwrap()
                .checked_next(),
            Err(EngineSelectionRequestIdError::Exhausted)
        );
        assert_eq!(
            serde_json::to_string(&EngineSelectionRequestId::FIRST).unwrap(),
            "1"
        );
    }

    #[test]
    fn engine_selection_status_enforces_the_declared_lifecycle_and_revision_boundary() {
        let preparing = preparing();
        assert_eq!(preparing.kind(), EngineSelectionStatusKind::Preparing);
        assert_eq!(preparing.active_graph_revision(), GraphRevision::INITIAL);
        assert!(preparing.is_in_flight());
        assert_eq!(preparing.failure(), None);
        assert_eq!(
            preparing.activating(GraphRevision::INITIAL),
            Err(EngineSelectionStatusError::TargetRevisionNotNewer)
        );

        let target = GraphRevision::INITIAL.checked_next().unwrap();
        let activating = preparing.activating(target).unwrap();
        assert_eq!(activating.kind(), EngineSelectionStatusKind::Activating);
        assert_eq!(activating.active_graph_revision(), GraphRevision::INITIAL);
        assert_eq!(
            activating.correlation().unwrap().target_graph_revision(),
            Some(target)
        );
        let ready = activating.acknowledged().unwrap();
        assert_eq!(ready, EngineSelectionStatus::ready(target));
        assert!(!ready.is_in_flight());

        let failed = preparing
            .failed(EngineSelectionFailure::AssetUnavailable)
            .unwrap();
        assert_eq!(failed.kind(), EngineSelectionStatusKind::Failed);
        assert_eq!(failed.active_graph_revision(), GraphRevision::INITIAL);
        assert_eq!(
            failed.failure(),
            Some(EngineSelectionFailure::AssetUnavailable)
        );
        assert_eq!(
            failed.acknowledged(),
            Err(EngineSelectionStatusError::InvalidTransition)
        );
    }

    #[test]
    fn engine_selection_status_serialization_contains_only_stable_control_metadata() {
        let json = serde_json::to_value(preparing()).unwrap();
        assert_eq!(json["kind"], "preparing");
        assert_eq!(json["activeGraphRevision"], 1);
        assert_eq!(json["correlation"]["requestId"], 1);
        assert_eq!(json["correlation"]["patchId"], 1);
        assert_eq!(
            json["correlation"]["sourceCapabilityId"],
            "instrument.source"
        );
        assert_eq!(
            json["correlation"]["targetCapabilityId"],
            "instrument.target"
        );
        assert_eq!(json["correlation"]["sourceGraphRevision"], 1);
        assert!(json["correlation"]["targetGraphRevision"].is_null());
        assert!(json["failure"].is_null());
    }

    #[test]
    fn engine_selection_effect_requires_the_revision_shape_declared_by_its_kind() {
        let preparing = preparing();
        let request = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::PrepareRequested,
            preparing.correlation().unwrap(),
        )
        .unwrap();
        assert_eq!(request.kind(), EngineSelectionEffectKind::PrepareRequested);
        assert_eq!(request.target_graph_revision(), None);
        assert_eq!(
            EngineSelectionEffect::from_correlation(
                EngineSelectionEffectKind::CandidateCommitted,
                preparing.correlation().unwrap(),
            ),
            Err(EngineSelectionStatusError::MissingTargetRevision)
        );

        let activating = preparing
            .activating(GraphRevision::INITIAL.checked_next().unwrap())
            .unwrap();
        let committed = EngineSelectionEffect::from_correlation(
            EngineSelectionEffectKind::CandidateCommitted,
            activating.correlation().unwrap(),
        )
        .unwrap();
        assert_eq!(
            committed.kind(),
            EngineSelectionEffectKind::CandidateCommitted
        );
        assert_eq!(committed.target_graph_revision().unwrap().value(), 2);
        assert_eq!(
            serde_json::to_value(committed).unwrap()["kind"],
            "candidateCommitted"
        );
    }
}
