pub mod app_event;
pub mod app_state;
pub mod event_record;
pub mod state_projector;
pub mod state_snapshot;
pub mod text_projection;

pub use app_event::{AppEvent, AppEventPayloadShape, AppEventSurfaceDescriptor, Direction};
pub use app_state::{
    AppState, ApplyOutcome, EventRejection, Selection, SelectionSection, StateAccepted,
};
pub use event_record::{
    AudioEffect, EmittedEvent, EventDirection, EventInput, EventOutcome, EventRecord,
    EventRecordError, EventSource, MidiInput, MidiKind, PatchInput,
};
pub use state_projector::{StateProjectionError, StateProjector};
pub use state_snapshot::StateSnapshot;
pub use text_projection::TextProjection;
pub mod app_loop;
pub mod state_tree;
pub use state_tree::{StateTree, StateTreeError};
pub mod event_log;
pub use event_log::{EventCoverage, EventLog, EventLogError};
