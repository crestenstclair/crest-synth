//! `MixBus` — a summing bus in the mixer's signal topology.
//!
//! Bus 0 is always the master bus: the final summing point that carries its
//! own insert chain and limiter before output. All other buses are aux
//! buses: they receive post-fader (by default) or pre-fader send taps from
//! channel strips and route exclusively back to the master bus. Aux buses
//! never feed one another — that would create an unanalyzable topology and
//! risk feedback loops.
//!
//! This type is domain/control-plane state. It is read and mutated on the
//! non-real-time thread; any resulting parameter changes cross to the audio
//! thread via the ParameterBridge or EventRing, never by sharing this struct
//! across threads directly.

use std::fmt;

/// Identity of a mix bus. Bus 0 is reserved for the master bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BusId(u32);

impl BusId {
    /// The one and only master bus identity.
    pub const MASTER: BusId = BusId(0);

    /// Construct a `BusId` from a raw value. `0` always denotes the master
    /// bus; any other value denotes an aux bus.
    pub const fn new(value: u32) -> Self {
        BusId(value)
    }

    /// The raw numeric identity.
    pub const fn value(self) -> u32 {
        self.0
    }

    /// True if this identity names the master bus.
    pub const fn is_master(self) -> bool {
        self.0 == Self::MASTER.0
    }
}

impl fmt::Display for BusId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_master() {
            write!(f, "BusId(master)")
        } else {
            write!(f, "BusId({})", self.0)
        }
    }
}

/// A validated, non-negative linear amplitude/gain multiplier.
///
/// Bounded to `[MIN, MAX]` (roughly +12 dB of headroom above unity) so that
/// runaway values can never reach the audio thread through a return-level
/// change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Amplitude(f32);

impl Amplitude {
    /// Silence.
    pub const MIN: f32 = 0.0;
    /// Roughly +12 dB of headroom above unity gain.
    pub const MAX: f32 = 4.0;
    /// Unity gain (0 dB).
    pub const UNITY: Amplitude = Amplitude(1.0);

    /// Validate and construct an `Amplitude`.
    ///
    /// Rejects NaN and anything outside `[MIN, MAX]`.
    pub fn try_new(value: f32) -> Result<Self, AmplitudeError> {
        if value.is_nan() || !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(AmplitudeError::OutOfRange(value));
        }
        Ok(Amplitude(value))
    }

    /// The validated linear gain value.
    pub const fn value(self) -> f32 {
        self.0
    }
}

impl Default for Amplitude {
    fn default() -> Self {
        Self::UNITY
    }
}

/// Errors constructing an [`Amplitude`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AmplitudeError {
    /// The supplied value was NaN or outside `[Amplitude::MIN, Amplitude::MAX]`.
    OutOfRange(f32),
}

impl fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AmplitudeError::OutOfRange(v) => write!(
                f,
                "amplitude {v} is out of range [{}, {}]",
                Amplitude::MIN,
                Amplitude::MAX
            ),
        }
    }
}

impl std::error::Error for AmplitudeError {}

/// Whether a bus is the master summing point or an auxiliary send/return bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusKind {
    /// The single, unremovable final summing point (`BusId::MASTER`).
    Master,
    /// An auxiliary bus that routes exclusively to the master bus.
    Aux,
}

/// Command: change a bus's return level (the gain applied to its summed
/// input before it is mixed into whatever it feeds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetReturnLevel {
    pub level: Amplitude,
}

/// Event: a bus's return level changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReturnLevelChanged {
    pub level: Amplitude,
}

/// Errors raised while handling operations against a [`MixBus`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MixBusError {
    /// An attempt was made to remove the master bus.
    CannotRemoveMaster,
    /// An attempt was made to route an aux bus somewhere other than the
    /// master bus (including another aux bus), or to construct an aux bus
    /// using the reserved master identity.
    AuxMustRouteToMaster { attempted: BusId },
}

impl fmt::Display for MixBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MixBusError::CannotRemoveMaster => {
                write!(f, "bus 0 is the master bus and cannot be removed")
            }
            MixBusError::AuxMustRouteToMaster { attempted } => write!(
                f,
                "aux buses feed the master bus, never each other (attempted route to {attempted})"
            ),
        }
    }
}

impl std::error::Error for MixBusError {}

/// A summing bus: the master bus or one auxiliary send/return bus.
///
/// Signal topology invariant: aux buses receive send taps from channel
/// strips and route only to the master bus; the master bus is the terminal
/// summing point that feeds the limiter and then output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixBus {
    id: BusId,
    kind: BusKind,
    return_level: Amplitude,
}

impl MixBus {
    /// Construct the master bus (`BusId::MASTER`) at unity return level.
    pub fn new_master() -> Self {
        MixBus {
            id: BusId::MASTER,
            kind: BusKind::Master,
            return_level: Amplitude::UNITY,
        }
    }

    /// Construct an auxiliary bus. Fails if `id` is `BusId::MASTER` — that
    /// identity is reserved for the one master bus.
    pub fn new_aux(id: BusId, return_level: Amplitude) -> Result<Self, MixBusError> {
        if id.is_master() {
            return Err(MixBusError::AuxMustRouteToMaster { attempted: id });
        }
        Ok(MixBus {
            id,
            kind: BusKind::Aux,
            return_level,
        })
    }

    /// This bus's identity.
    pub fn id(&self) -> BusId {
        self.id
    }

    /// Whether this is the master bus or an aux bus.
    pub fn kind(&self) -> BusKind {
        self.kind
    }

    /// The current return level.
    pub fn return_level(&self) -> Amplitude {
        self.return_level
    }

    /// The bus this bus routes its summed signal to. The master bus has no
    /// downstream bus (`None`); every aux bus routes to the master bus.
    pub fn routes_to(&self) -> Option<BusId> {
        match self.kind {
            BusKind::Master => None,
            BusKind::Aux => Some(BusId::MASTER),
        }
    }

    /// Whether this bus may be removed from the mixer. The master bus may
    /// never be removed.
    pub fn can_remove(&self) -> bool {
        self.kind != BusKind::Master
    }

    /// Validate a removal request against this bus, without performing it.
    pub fn check_removable(&self) -> Result<(), MixBusError> {
        if self.can_remove() {
            Ok(())
        } else {
            Err(MixBusError::CannotRemoveMaster)
        }
    }

    /// Handle [`SetReturnLevel`], producing the resulting event without
    /// mutating state (command/event separation).
    pub fn handle_set_return_level(
        &self,
        command: SetReturnLevel,
    ) -> Result<ReturnLevelChanged, MixBusError> {
        Ok(ReturnLevelChanged {
            level: command.level,
        })
    }

    /// Apply a previously produced [`ReturnLevelChanged`] event to this bus.
    pub fn apply(&mut self, event: ReturnLevelChanged) {
        self.return_level = event.level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_id_zero_is_master() {
        assert!(BusId::MASTER.is_master());
        assert!(BusId::new(0).is_master());
        assert!(!BusId::new(1).is_master());
    }

    #[test]
    fn amplitude_accepts_in_range_values() {
        assert!(Amplitude::try_new(0.0).is_ok());
        assert!(Amplitude::try_new(1.0).is_ok());
        assert!(Amplitude::try_new(Amplitude::MAX).is_ok());
    }

    #[test]
    fn amplitude_rejects_out_of_range_and_nan() {
        assert!(Amplitude::try_new(-0.01).is_err());
        assert!(Amplitude::try_new(Amplitude::MAX + 0.01).is_err());
        assert!(Amplitude::try_new(f32::NAN).is_err());
    }

    #[test]
    fn new_master_is_master_kind_and_unremovable() {
        let master = MixBus::new_master();
        assert_eq!(master.kind(), BusKind::Master);
        assert_eq!(master.id(), BusId::MASTER);
        assert!(!master.can_remove());
        assert_eq!(
            master.check_removable(),
            Err(MixBusError::CannotRemoveMaster)
        );
        assert_eq!(master.routes_to(), None);
    }

    #[test]
    fn new_aux_routes_only_to_master() {
        let aux = MixBus::new_aux(BusId::new(1), Amplitude::UNITY).unwrap();
        assert_eq!(aux.kind(), BusKind::Aux);
        assert!(aux.can_remove());
        assert_eq!(aux.check_removable(), Ok(()));
        assert_eq!(aux.routes_to(), Some(BusId::MASTER));
    }

    #[test]
    fn new_aux_rejects_master_identity() {
        let result = MixBus::new_aux(BusId::MASTER, Amplitude::UNITY);
        assert_eq!(
            result,
            Err(MixBusError::AuxMustRouteToMaster {
                attempted: BusId::MASTER
            })
        );
    }

    #[test]
    fn set_return_level_produces_matching_event() {
        let bus = MixBus::new_master();
        let level = Amplitude::try_new(0.5).unwrap();
        let event = bus
            .handle_set_return_level(SetReturnLevel { level })
            .unwrap();
        assert_eq!(event.level, level);
    }

    #[test]
    fn apply_return_level_changed_updates_state() {
        let mut bus = MixBus::new_aux(BusId::new(2), Amplitude::UNITY).unwrap();
        let level = Amplitude::try_new(0.25).unwrap();
        bus.apply(ReturnLevelChanged { level });
        assert_eq!(bus.return_level(), level);
    }

    #[test]
    fn command_then_apply_round_trips_return_level() {
        let mut bus = MixBus::new_aux(BusId::new(3), Amplitude::UNITY).unwrap();
        let level = Amplitude::try_new(2.0).unwrap();
        let event = bus
            .handle_set_return_level(SetReturnLevel { level })
            .unwrap();
        bus.apply(event);
        assert_eq!(bus.return_level(), level);
    }

    #[test]
    fn aux_buses_never_route_to_each_other() {
        let aux_a = MixBus::new_aux(BusId::new(1), Amplitude::UNITY).unwrap();
        let aux_b = MixBus::new_aux(BusId::new(2), Amplitude::UNITY).unwrap();
        // Every aux bus's routing target is the master, never another aux.
        assert_eq!(aux_a.routes_to(), Some(BusId::MASTER));
        assert_eq!(aux_b.routes_to(), Some(BusId::MASTER));
        assert_ne!(aux_a.routes_to(), Some(aux_b.id()));
    }
}
