pub mod app_event;
pub mod app_state;
pub mod engine_selection;
pub mod event_record;
pub mod interaction_state;
pub mod patch_control_id;
pub mod patch_page_projection;
mod serialized_state;
pub mod state_projector;
pub mod state_snapshot;
pub mod text_projection;
pub mod top_level_context;

pub use app_event::{AppEvent, AppEventPayloadShape, AppEventSurfaceDescriptor, Direction};
pub use app_state::{AppState, ApplyOutcome, EventRejection, StateAccepted};
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
pub use interaction_state::{InteractionState, Selection, SelectionSection};
pub use patch_control_id::PatchControlId;
pub use patch_page_projection::{
    PatchPageEngine, PatchPageEngineChoice, PatchPageEnvelopeRow, PatchPageIdentity,
    PatchPageParameterRow, PatchPageParameterValue, PatchPageProjection, PatchPageProjectionError,
    PatchPageSection,
};
pub use state_projector::{StateProjectionError, StateProjector};
pub use state_snapshot::StateSnapshot;
pub use text_projection::TextProjection;
pub use top_level_context::TopLevelContext;
pub mod app_loop;
pub use app_loop::{AppLoop, DispatchResult, StructuralAdvanceError, StructuralProgress};
pub mod state_tree;
pub use state_tree::{StateTree, StateTreeError};
pub mod event_log;
pub use event_log::{EventCoverage, EventLog, EventLogError};
