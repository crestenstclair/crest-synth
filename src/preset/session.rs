//! `Session` — the root aggregate of the `Preset` context.
//!
//! A `Session` is a snapshot of everything transport-related in the running
//! synth: tempo and time signature (the wider "everything" described in the
//! resource's purpose — patches, mixer state, aux buses, master bus — is
//! owned by their own aggregates elsewhere; this type only models the state
//! fields declared for `aggregate.Preset.Session`).
//!
//! The single command this aggregate answers, `Restore`, must behave
//! atomically: either every field of the session is replaced by a fully
//! valid snapshot, or none of it is. To make that guarantee mechanically
//! checkable (rather than just asserted in a comment) the aggregate never
//! mutates its live state directly from raw input. A caller first stages a
//! [`SessionSnapshot`] — which can only be constructed from already-valid
//! [`Tempo`] and [`TimeSignature`] values — and only then issues
//! [`SessionCommand::Restore`], which performs the swap. If no snapshot has
//! been staged, `Restore` fails with [`SessionError::NoPendingSnapshot`] and
//! the prior state is left completely untouched.

use std::error::Error;
use std::fmt;

/// Playback tempo, in beats per minute.
///
/// Restricted to a musically sane range so that downstream scheduling code
/// (e.g. clock-derived LFO rates) never has to reason about degenerate
/// values such as zero or negative tempos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo(f32);

impl Tempo {
    /// Minimum supported tempo, in BPM.
    pub const MIN_BPM: f32 = 20.0;
    /// Maximum supported tempo, in BPM.
    pub const MAX_BPM: f32 = 300.0;

    /// Constructs a `Tempo`, validating that `bpm` is a finite value inside
    /// `[MIN_BPM, MAX_BPM]`.
    pub fn try_new(bpm: f32) -> Result<Self, SessionError> {
        if bpm.is_nan() || !(Self::MIN_BPM..=Self::MAX_BPM).contains(&bpm) {
            return Err(SessionError::InvalidTempo(bpm));
        }
        Ok(Self(bpm))
    }

    /// Returns the tempo in beats per minute.
    pub fn bpm(&self) -> f32 {
        self.0
    }
}

/// A musical time signature, e.g. 4/4 or 6/8.
///
/// The denominator must be a power of two (the note value a beat
/// represents) and the numerator must be non-zero (there must be at least
/// one beat per measure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSignature {
    numerator: u8,
    denominator: u8,
}

impl TimeSignature {
    /// Constructs a `TimeSignature`, validating that `numerator` is
    /// non-zero and `denominator` is a non-zero power of two.
    pub fn try_new(numerator: u8, denominator: u8) -> Result<Self, SessionError> {
        let denominator_is_power_of_two = denominator != 0 && denominator.is_power_of_two();
        if numerator == 0 || !denominator_is_power_of_two {
            return Err(SessionError::InvalidTimeSignature {
                numerator,
                denominator,
            });
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// The number of beats per measure.
    pub fn numerator(&self) -> u8 {
        self.numerator
    }

    /// The note value that represents one beat (e.g. `4` for a quarter
    /// note).
    pub fn denominator(&self) -> u8 {
        self.denominator
    }
}

/// A fully-valid candidate replacement for a [`Session`]'s state.
///
/// Because a `SessionSnapshot` can only ever be built from already-valid
/// [`Tempo`] and [`TimeSignature`] values, applying one to a `Session` can
/// never fail partway through: by the time it exists, both of its fields
/// are guaranteed valid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionSnapshot {
    tempo: Tempo,
    time_signature: TimeSignature,
}

impl SessionSnapshot {
    /// Builds a snapshot from already-validated component values.
    pub fn new(tempo: Tempo, time_signature: TimeSignature) -> Self {
        Self {
            tempo,
            time_signature,
        }
    }

    /// The tempo captured in this snapshot.
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    /// The time signature captured in this snapshot.
    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }
}

/// Errors that can arise while constructing session state or handling
/// [`SessionCommand`]s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionError {
    /// A tempo value fell outside `[Tempo::MIN_BPM, Tempo::MAX_BPM]` or was
    /// not finite.
    InvalidTempo(f32),
    /// A time signature had a zero numerator or a denominator that was not
    /// a non-zero power of two.
    InvalidTimeSignature { numerator: u8, denominator: u8 },
    /// `Restore` was issued with no snapshot staged via
    /// [`Session::stage_restore`]. The prior session state is left
    /// completely untouched.
    NoPendingSnapshot,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::InvalidTempo(bpm) => write!(
                f,
                "invalid tempo {bpm} bpm: must be finite and within [{}, {}]",
                Tempo::MIN_BPM,
                Tempo::MAX_BPM
            ),
            SessionError::InvalidTimeSignature {
                numerator,
                denominator,
            } => write!(
                f,
                "invalid time signature {numerator}/{denominator}: numerator must be non-zero \
                 and denominator must be a non-zero power of two"
            ),
            SessionError::NoPendingSnapshot => {
                write!(f, "restore requested with no snapshot staged")
            }
        }
    }
}

impl Error for SessionError {}

/// Commands accepted by [`Session`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommand {
    /// Atomically replace all session state with the previously staged
    /// snapshot (see [`Session::stage_restore`]).
    Restore,
}

/// Events emitted by [`Session`] in response to commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// The session's state was atomically replaced.
    Restored,
}

/// The root aggregate of the `Preset` context: a snapshot of tempo and time
/// signature for the running synth.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    tempo: Tempo,
    time_signature: TimeSignature,
    pending_restore: Option<SessionSnapshot>,
}

impl Session {
    /// Constructs a new `Session` with the given initial tempo and time
    /// signature and no pending restore staged.
    pub fn new(tempo: Tempo, time_signature: TimeSignature) -> Self {
        Self {
            tempo,
            time_signature,
            pending_restore: None,
        }
    }

    /// The session's current tempo.
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    /// The session's current time signature.
    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    /// Stages a fully-valid snapshot to be applied by the next
    /// [`SessionCommand::Restore`]. Staging a new snapshot replaces any
    /// previously staged (and not yet applied) snapshot.
    pub fn stage_restore(&mut self, snapshot: SessionSnapshot) {
        self.pending_restore = Some(snapshot);
    }

    /// Handles a [`SessionCommand`], returning the resulting
    /// [`SessionEvent`] on success.
    pub fn handle(&mut self, command: SessionCommand) -> Result<SessionEvent, SessionError> {
        match command {
            SessionCommand::Restore => self.restore(),
        }
    }

    /// Atomically replaces all session state with the staged snapshot.
    ///
    /// If no snapshot has been staged, returns
    /// [`SessionError::NoPendingSnapshot`] and leaves `self` byte-for-byte
    /// unchanged — there is no partial application.
    fn restore(&mut self) -> Result<SessionEvent, SessionError> {
        let snapshot = self
            .pending_restore
            .take()
            .ok_or(SessionError::NoPendingSnapshot)?;

        // Both fields of `snapshot` were validated at construction time, so
        // this replacement is total: it cannot fail partway through and
        // leave `self` in a mixed old/new state.
        self.tempo = snapshot.tempo();
        self.time_signature = snapshot.time_signature();

        Ok(SessionEvent::Restored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_tempo() -> Tempo {
        Tempo::try_new(120.0).expect("120 bpm is within range")
    }

    fn other_valid_tempo() -> Tempo {
        Tempo::try_new(90.0).expect("90 bpm is within range")
    }

    fn valid_time_signature() -> TimeSignature {
        TimeSignature::try_new(4, 4).expect("4/4 is a valid time signature")
    }

    fn other_valid_time_signature() -> TimeSignature {
        TimeSignature::try_new(6, 8).expect("6/8 is a valid time signature")
    }

    #[test]
    fn tempo_accepts_values_within_range() {
        assert!(Tempo::try_new(Tempo::MIN_BPM).is_ok());
        assert!(Tempo::try_new(Tempo::MAX_BPM).is_ok());
        assert!(Tempo::try_new(120.0).is_ok());
    }

    #[test]
    fn tempo_rejects_values_outside_range() {
        assert_eq!(Tempo::try_new(0.0), Err(SessionError::InvalidTempo(0.0)));
        assert_eq!(
            Tempo::try_new(-10.0),
            Err(SessionError::InvalidTempo(-10.0))
        );
        assert_eq!(
            Tempo::try_new(1000.0),
            Err(SessionError::InvalidTempo(1000.0))
        );
    }

    #[test]
    fn tempo_rejects_nan() {
        assert!(matches!(
            Tempo::try_new(f32::NAN),
            Err(SessionError::InvalidTempo(_))
        ));
    }

    #[test]
    fn time_signature_accepts_valid_values() {
        assert!(TimeSignature::try_new(4, 4).is_ok());
        assert!(TimeSignature::try_new(3, 4).is_ok());
        assert!(TimeSignature::try_new(6, 8).is_ok());
        assert!(TimeSignature::try_new(7, 16).is_ok());
    }

    #[test]
    fn time_signature_rejects_zero_numerator() {
        assert_eq!(
            TimeSignature::try_new(0, 4),
            Err(SessionError::InvalidTimeSignature {
                numerator: 0,
                denominator: 4
            })
        );
    }

    #[test]
    fn time_signature_rejects_non_power_of_two_denominator() {
        assert_eq!(
            TimeSignature::try_new(4, 3),
            Err(SessionError::InvalidTimeSignature {
                numerator: 4,
                denominator: 3
            })
        );
        assert_eq!(
            TimeSignature::try_new(4, 0),
            Err(SessionError::InvalidTimeSignature {
                numerator: 4,
                denominator: 0
            })
        );
    }

    #[test]
    fn new_session_has_no_pending_restore() {
        let session = Session::new(valid_tempo(), valid_time_signature());
        assert_eq!(session.tempo(), valid_tempo());
        assert_eq!(session.time_signature(), valid_time_signature());
    }

    #[test]
    fn restore_without_staged_snapshot_errs_and_leaves_state_untouched() {
        let mut session = Session::new(valid_tempo(), valid_time_signature());
        let before = session.clone();

        let result = session.handle(SessionCommand::Restore);

        assert_eq!(result, Err(SessionError::NoPendingSnapshot));
        assert_eq!(session, before);
    }

    #[test]
    fn restore_applies_staged_snapshot_atomically() {
        let mut session = Session::new(valid_tempo(), valid_time_signature());
        let snapshot = SessionSnapshot::new(other_valid_tempo(), other_valid_time_signature());
        session.stage_restore(snapshot);

        let result = session.handle(SessionCommand::Restore);

        assert_eq!(result, Ok(SessionEvent::Restored));
        assert_eq!(session.tempo(), other_valid_tempo());
        assert_eq!(session.time_signature(), other_valid_time_signature());
    }

    #[test]
    fn restore_consumes_the_staged_snapshot() {
        let mut session = Session::new(valid_tempo(), valid_time_signature());
        let snapshot = SessionSnapshot::new(other_valid_tempo(), other_valid_time_signature());
        session.stage_restore(snapshot);

        session
            .handle(SessionCommand::Restore)
            .expect("first restore succeeds");
        let after_first = session.clone();

        // No snapshot staged this time: the second restore must fail and
        // must not touch the state left by the first restore.
        let second = session.handle(SessionCommand::Restore);

        assert_eq!(second, Err(SessionError::NoPendingSnapshot));
        assert_eq!(session, after_first);
    }

    #[test]
    fn staging_a_new_snapshot_replaces_any_previously_staged_one() {
        let mut session = Session::new(valid_tempo(), valid_time_signature());
        session.stage_restore(SessionSnapshot::new(
            other_valid_tempo(),
            valid_time_signature(),
        ));
        // Overwrite with a different snapshot before ever restoring.
        session.stage_restore(SessionSnapshot::new(
            valid_tempo(),
            other_valid_time_signature(),
        ));

        session
            .handle(SessionCommand::Restore)
            .expect("restore succeeds");

        assert_eq!(session.tempo(), valid_tempo());
        assert_eq!(session.time_signature(), other_valid_time_signature());
    }
}
