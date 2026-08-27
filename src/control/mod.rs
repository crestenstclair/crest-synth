pub mod app_event;
pub mod app_state;
pub mod engine_selection;
pub mod event_record;
pub mod graphical_shell_projection;
pub mod interaction_state;
pub mod patch_control_id;
pub mod patch_page_projection;
pub mod sample_browser_state;
pub mod saved_session;
pub mod semantic_action;
pub mod semantic_focus;
pub mod semantic_graphical_view_model;
pub mod semantic_resolver;
mod serialized_state;
pub mod state_projector;
pub mod state_snapshot;
pub mod text_projection;
pub mod top_level_context;

pub use app_event::{AppEvent, AppEventPayloadShape, AppEventSurfaceDescriptor, Direction};
pub use app_state::{
    AppState, ApplyOutcome, EventRejection, FocusRepairStatus, SemanticActionAvailability,
    StateAccepted,
};
pub use engine_selection::{
    EngineSelectionCorrelation, EngineSelectionEffect, EngineSelectionEffectKind,
    EngineSelectionFailure, EngineSelectionRequestId, EngineSelectionRequestIdError,
    EngineSelectionStatus, EngineSelectionStatusError, EngineSelectionStatusKind,
    StructuralEditIntent,
};
pub use event_record::{
    AudioEffect, EmittedEvent, EventDirection, EventInput, EventOutcome, EventRecord,
    EventRecordError, EventSource, MidiInput, MidiKind, PatchInput,
};
pub use graphical_shell_projection::{
    GraphicalShellProjection, GraphicalShellProjectionError, ShellContextLine, ShellFooter,
    ShellIdentityHeader, ShellMainRegion, ShellSideRegion, ShellWorkspace,
};
pub use interaction_state::{
    InteractionState, PatchSubordinateSession, Selection, SelectionSection,
};
pub use patch_control_id::PatchControlId;
pub use patch_page_projection::{
    PatchPageEffectSlot, PatchPageEngine, PatchPageEngineChoice, PatchPageEnvelopeRow,
    PatchPageIdentity, PatchPageOccupancyChoice, PatchPageParameterRow, PatchPageParameterValue,
    PatchPageProjection, PatchPageProjectionError, PatchPageSection, PatchPageSlotOccupancy,
    EMPTY_OCCUPANCY_CHOICE_ID,
};
pub use sample_browser_state::{SampleAssetLifecycle, SampleBrowserState, SamplePreviewState};
pub use saved_session::{
    PreparedSavedSession, SavedSession, SavedSessionError, SavedSessionRestoreError,
    SAVED_SESSION_VERSION,
};
pub use semantic_action::{InteractionMode, SemanticAction, SemanticActionKind, ValidAction};
pub use semantic_focus::{
    FocusCapabilityId, FocusPath, FocusPathError, MixerControlId, ModalControlId,
    PatchChoiceSubject, PatchDetailSubject, ReturnPath, SemanticControlId, SurfaceId,
};
pub use semantic_graphical_view_model::{
    SemanticBrowserMetadata, SemanticBrowserMetadataStatus, SemanticControlKind,
    SemanticControlValue, SemanticControlViewModel, SemanticError, SemanticErrorCode,
    SemanticFocusRepairStatus, SemanticGraphicalViewModel, SemanticGraphicalViewModelError,
    SemanticLifecycleStatus, SemanticNumericRange, SemanticSurfaceControlSummaryViewModel,
    SemanticSurfaceRole, SemanticSurfaceSectionViewModel, SemanticSurfaceSummary,
    SemanticSurfaceViewModel, SemanticVisualizationData, SemanticVisualizationViewModel,
    SemanticWaveformLandmark, SemanticWaveformPair,
};
pub use semantic_resolver::{ResolvedChoiceOption, ResolvedChoiceSource, SemanticResolver};
pub use state_projector::{StateProjectionError, StateProjector};
pub use state_snapshot::StateSnapshot;
pub use text_projection::TextProjection;
pub use top_level_context::TopLevelContext;
pub mod app_loop;
pub use app_loop::{
    AppLoop, DispatchResult, MidiFanOutResult, StructuralAdvanceError, StructuralProgress,
};
pub mod state_tree;
pub use state_tree::{StateTree, StateTreeError};
pub mod event_log;
pub use event_log::{EventCoverage, EventLog, EventLogError};
