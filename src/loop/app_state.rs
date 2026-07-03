// path: src/loop/app_state.rs

use crate::r#loop::app_event::AppEvent;

/// Decides whether a given [`AppEvent`] may be applied to the current
/// [`AppState`]. Injected into `apply_with` so business rules for what is
/// "applicable" can evolve (Open/Closed) without `AppState` itself changing,
/// and so tests can substitute a validator without touching production code
/// (Dependency Inversion).
pub trait EventValidator {
    /// Returns `Ok(())` if `event` may be applied to `state`, or `Err(reason)`
    /// describing why it must be rejected.
    fn validate(&self, state: &AppState, event: &AppEvent) -> Result<(), String>;
}

/// The default validator: every event is applicable except one that would
/// overflow the frame clock. Real per-domain rejection rules (e.g. "no
/// patch selected", "MPE zones would overlap") belong in a dedicated
/// `EventValidator` supplied via [`AppState::apply_with`], not here.
#[derive(Debug, Default, Clone, Copy)]
pub struct PermissiveEventValidator;

impl EventValidator for PermissiveEventValidator {
    fn validate(&self, state: &AppState, _event: &AppEvent) -> Result<(), String> {
        if state.frame == u64::MAX {
            Err("frame counter exhausted".to_string())
        } else {
            Ok(())
        }
    }
}

/// The outcome of applying a single [`AppEvent`] to [`AppState`]: either the
/// event-sequence clock advanced by exactly one frame, or the event was
/// rejected and the state was left untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStateEvent {
    Applied { frame: u64 },
    Rejected { reason: String },
}

/// The whole control-plane state as one value.
///
/// `patch set / mixer / editor view state / active session` each live in
/// their own aggregates (`Patch`, `MixBus`/`ChannelStrip`, `EditorState`,
/// `Session`); `AppState` is the root clock that sequences every
/// control-plane mutation. `apply` is the ONLY way to advance it -- there
/// are no setters and no direct field mutation anywhere in the control
/// plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    frame: u64,
}

impl AppState {
    /// A fresh control plane at frame zero.
    pub fn new() -> Self {
        AppState { frame: 0 }
    }

    /// The event-sequence clock. This is a logical counter of applied
    /// events, not a wall-clock timestamp.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Applies `event` using the default [`PermissiveEventValidator`].
    ///
    /// Pure and deterministic: the same `AppState` and the same `event`
    /// always produce the same `(AppState, AppStateEvent)` pair. Never
    /// panics -- an inapplicable event yields `AppStateEvent::Rejected` and
    /// the returned state is identical to `self`.
    pub fn apply(&self, event: AppEvent) -> (AppState, AppStateEvent) {
        self.apply_with(&PermissiveEventValidator, event)
    }

    /// Applies `event` using an explicit `validator`. This is the
    /// injection seam `apply` delegates to by default; callers with
    /// domain-specific applicability rules supply their own
    /// [`EventValidator`] instead of a hidden default.
    pub fn apply_with<V: EventValidator>(
        &self,
        validator: &V,
        event: AppEvent,
    ) -> (AppState, AppStateEvent) {
        match validator.validate(self, &event) {
            Ok(()) => match self.frame.checked_add(1) {
                Some(next_frame) => (
                    AppState { frame: next_frame },
                    AppStateEvent::Applied { frame: next_frame },
                ),
                None => (
                    self.clone(),
                    AppStateEvent::Rejected {
                        reason: "frame counter exhausted".to_string(),
                    },
                ),
            },
            Err(reason) => (self.clone(), AppStateEvent::Rejected { reason }),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests deliberately never construct an `AppEvent` value: this
    // module owns only the reducer shell (frame bookkeeping + the
    // `EventValidator` seam), not `AppEvent`'s own constructors/variants,
    // which are that value object's concern.

    #[test]
    fn new_state_starts_at_frame_zero() {
        assert_eq!(AppState::new().frame(), 0);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(AppState::default(), AppState::new());
    }

    #[test]
    fn states_with_equal_frames_are_equal() {
        assert_eq!(AppState::new(), AppState::new());
    }
}
