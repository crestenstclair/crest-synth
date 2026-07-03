// path: src/mixer/mixer_controller.rs

//! `MixerController` — application-level mixer operations: channel strip
//! CRUD, solo group handling, and bus management.
//!
//! This is a non-real-time orchestration layer over the `ChannelStrip` and
//! `MixBus` aggregates. It owns no audio-thread state and performs no I/O:
//! it validates and dispatches commands to the aggregates it manages and
//! exposes read-only queries (such as solo-derived audibility) that a
//! real-time snapshot publisher can consult when building whatever crosses
//! the ParameterBridge.
//!
//! Solo handling is computed, not stored: whether a strip is currently
//! audible is derived from the live `solo`/`mute` state of every strip in
//! the mixer, so there is exactly one source of truth (the individual
//! `ChannelStrip`s) and no separate "solo group" flag that could drift out
//! of sync with the strips it summarizes.

use std::collections::HashMap;
use std::fmt;

use crate::mixer::channel_strip::{
    ChannelStrip, ChannelStripCommand, ChannelStripError, ChannelStripEvent,
};
use crate::mixer::mix_bus::{
    Amplitude as BusAmplitude, BusId, MixBus, MixBusError, ReturnLevelChanged, SetReturnLevel,
};

/// Identifies one channel strip owned by a [`MixerController`].
///
/// Assigned by the controller when a strip is added; stable for the
/// lifetime of the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StripId(u32);

impl StripId {
    /// The raw numeric identity.
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for StripId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StripId({})", self.0)
    }
}

/// Errors raised while handling an operation against a [`MixerController`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MixerControllerError {
    /// No channel strip is registered under this id.
    StripNotFound(StripId),
    /// No mix bus is registered under this id.
    BusNotFound(BusId),
    /// The channel strip aggregate rejected the command.
    ChannelStrip(ChannelStripError),
    /// The mix bus aggregate rejected the command.
    MixBus(MixBusError),
}

impl fmt::Display for MixerControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MixerControllerError::StripNotFound(id) => {
                write!(f, "no channel strip registered under {id}")
            }
            MixerControllerError::BusNotFound(id) => {
                write!(f, "no mix bus registered under {id}")
            }
            MixerControllerError::ChannelStrip(e) => write!(f, "{e}"),
            MixerControllerError::MixBus(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MixerControllerError {}

impl From<ChannelStripError> for MixerControllerError {
    fn from(e: ChannelStripError) -> Self {
        MixerControllerError::ChannelStrip(e)
    }
}

impl From<MixBusError> for MixerControllerError {
    fn from(e: MixBusError) -> Self {
        MixerControllerError::MixBus(e)
    }
}

/// Application-level mixer operations: channel strip CRUD, solo group
/// handling, and bus management.
///
/// Owns the full set of [`ChannelStrip`]s and [`MixBus`]es for a session.
/// The master bus (`BusId::MASTER`) is always present and can never be
/// removed, matching the `MixBus` invariant.
#[derive(Debug, Clone, PartialEq)]
pub struct MixerController {
    strips: HashMap<StripId, ChannelStrip>,
    buses: HashMap<BusId, MixBus>,
    next_strip_id: u32,
}

impl MixerController {
    /// Creates a controller with no channel strips and only the master bus,
    /// matching the invariant that bus 0 is always present.
    pub fn new() -> Self {
        Self::with_state(HashMap::new(), Self::initial_buses())
    }

    /// Full constructor: builds a controller from pre-existing strip and
    /// bus state (e.g. when restoring a session). Callers that don't care
    /// about initial state should use [`MixerController::new`].
    pub fn with_state(
        strips: HashMap<StripId, ChannelStrip>,
        buses: HashMap<BusId, MixBus>,
    ) -> Self {
        let next_strip_id = strips
            .keys()
            .map(|id| id.value())
            .max()
            .map_or(0, |max| max + 1);
        Self {
            strips,
            buses,
            next_strip_id,
        }
    }

    fn initial_buses() -> HashMap<BusId, MixBus> {
        let mut buses = HashMap::new();
        buses.insert(BusId::MASTER, MixBus::new_master());
        buses
    }

    // ---- Channel strip CRUD ----

    /// Adds a new channel strip at default settings and returns its id.
    pub fn add_strip(&mut self) -> StripId {
        let id = StripId(self.next_strip_id);
        self.next_strip_id += 1;
        self.strips.insert(id, ChannelStrip::new());
        id
    }

    /// Removes a channel strip. Fails if no strip is registered under `id`.
    pub fn remove_strip(&mut self, id: StripId) -> Result<(), MixerControllerError> {
        self.strips
            .remove(&id)
            .map(|_| ())
            .ok_or(MixerControllerError::StripNotFound(id))
    }

    /// Returns a reference to the channel strip registered under `id`.
    pub fn strip(&self, id: StripId) -> Result<&ChannelStrip, MixerControllerError> {
        self.strips
            .get(&id)
            .ok_or(MixerControllerError::StripNotFound(id))
    }

    /// Iterates over every registered `(StripId, &ChannelStrip)` pair.
    pub fn strips(&self) -> impl Iterator<Item = (&StripId, &ChannelStrip)> {
        self.strips.iter()
    }

    // ---- Bus management ----

    /// Adds a new auxiliary bus. Fails if `id` names the reserved master
    /// bus identity — the master bus already exists from construction.
    pub fn add_aux_bus(
        &mut self,
        id: BusId,
        return_level: BusAmplitude,
    ) -> Result<(), MixerControllerError> {
        let bus = MixBus::new_aux(id, return_level)?;
        self.buses.insert(id, bus);
        Ok(())
    }

    /// Removes a bus. Fails if no bus is registered under `id`, or if `id`
    /// names the master bus (bus 0 can never be removed).
    pub fn remove_bus(&mut self, id: BusId) -> Result<(), MixerControllerError> {
        let bus = self
            .buses
            .get(&id)
            .ok_or(MixerControllerError::BusNotFound(id))?;
        bus.check_removable()?;
        self.buses.remove(&id);
        Ok(())
    }

    /// Returns a reference to the bus registered under `id`.
    pub fn bus(&self, id: BusId) -> Result<&MixBus, MixerControllerError> {
        self.buses
            .get(&id)
            .ok_or(MixerControllerError::BusNotFound(id))
    }

    /// Iterates over every registered `(BusId, &MixBus)` pair.
    pub fn buses(&self) -> impl Iterator<Item = (&BusId, &MixBus)> {
        self.buses.iter()
    }

    /// Changes a bus's return level.
    pub fn set_return_level(
        &mut self,
        id: BusId,
        level: BusAmplitude,
    ) -> Result<ReturnLevelChanged, MixerControllerError> {
        let bus = self
            .buses
            .get_mut(&id)
            .ok_or(MixerControllerError::BusNotFound(id))?;
        let event = bus.handle_set_return_level(SetReturnLevel { level })?;
        bus.apply(event);
        Ok(event)
    }

    // ---- Solo group handling ----

    /// Sets a channel strip's solo flag (the `setSolo` operation).
    pub fn set_solo(
        &mut self,
        strip: StripId,
        solo: bool,
    ) -> Result<Vec<ChannelStripEvent>, MixerControllerError> {
        let strip_ref = self
            .strips
            .get_mut(&strip)
            .ok_or(MixerControllerError::StripNotFound(strip))?;
        let events = strip_ref.handle(ChannelStripCommand::SetSolo { solo })?;
        Ok(events)
    }

    /// True if any registered strip currently has its solo flag set.
    pub fn any_solo_active(&self) -> bool {
        self.strips.values().any(ChannelStrip::solo)
    }

    /// Whether `id`'s channel strip should be audible right now, applying
    /// solo-in-place semantics: when any strip in the mixer is soloed, only
    /// soloed strips are audible; otherwise a strip is audible unless it is
    /// muted. This never mutates a strip's own mute/solo state — it is a
    /// pure derivation from the current strip states.
    pub fn is_audible(&self, id: StripId) -> Result<bool, MixerControllerError> {
        let strip = self.strip(id)?;
        if self.any_solo_active() {
            Ok(strip.solo())
        } else {
            Ok(!strip.mute())
        }
    }
}

impl Default for MixerController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_controller_has_only_master_bus_and_no_strips() {
        let controller = MixerController::new();
        assert_eq!(controller.strips().count(), 0);
        assert_eq!(controller.buses().count(), 1);
        assert!(controller.bus(BusId::MASTER).is_ok());
    }

    #[test]
    fn add_strip_assigns_unique_ids() {
        let mut controller = MixerController::new();
        let a = controller.add_strip();
        let b = controller.add_strip();
        assert_ne!(a, b);
        assert_eq!(controller.strips().count(), 2);
    }

    #[test]
    fn remove_strip_removes_registered_strip() {
        let mut controller = MixerController::new();
        let id = controller.add_strip();
        assert!(controller.remove_strip(id).is_ok());
        assert_eq!(controller.strips().count(), 0);
    }

    #[test]
    fn remove_strip_fails_for_unknown_id() {
        let mut controller = MixerController::new();
        let id = controller.add_strip();
        controller.remove_strip(id).unwrap();
        assert_eq!(
            controller.remove_strip(id),
            Err(MixerControllerError::StripNotFound(id))
        );
    }

    #[test]
    fn add_aux_bus_registers_new_bus() {
        let mut controller = MixerController::new();
        let id = BusId::new(1);
        controller.add_aux_bus(id, BusAmplitude::UNITY).unwrap();
        assert_eq!(controller.buses().count(), 2);
        assert!(controller.bus(id).is_ok());
    }

    #[test]
    fn add_aux_bus_rejects_master_identity() {
        let mut controller = MixerController::new();
        let result = controller.add_aux_bus(BusId::MASTER, BusAmplitude::UNITY);
        assert!(result.is_err());
    }

    #[test]
    fn remove_bus_rejects_master_bus() {
        let mut controller = MixerController::new();
        let result = controller.remove_bus(BusId::MASTER);
        assert_eq!(
            result,
            Err(MixerControllerError::MixBus(
                MixBusError::CannotRemoveMaster
            ))
        );
        assert_eq!(controller.buses().count(), 1);
    }

    #[test]
    fn remove_bus_removes_registered_aux_bus() {
        let mut controller = MixerController::new();
        let id = BusId::new(2);
        controller.add_aux_bus(id, BusAmplitude::UNITY).unwrap();
        assert!(controller.remove_bus(id).is_ok());
        assert_eq!(controller.buses().count(), 1);
    }

    #[test]
    fn remove_bus_fails_for_unknown_id() {
        let mut controller = MixerController::new();
        let result = controller.remove_bus(BusId::new(9));
        assert_eq!(
            result,
            Err(MixerControllerError::BusNotFound(BusId::new(9)))
        );
    }

    #[test]
    fn set_return_level_updates_bus_state() {
        let mut controller = MixerController::new();
        let level = BusAmplitude::try_new(0.5).unwrap();
        let event = controller.set_return_level(BusId::MASTER, level).unwrap();
        assert_eq!(event.level, level);
        assert_eq!(controller.bus(BusId::MASTER).unwrap().return_level(), level);
    }

    #[test]
    fn set_solo_toggles_strip_solo_and_emits_event() {
        let mut controller = MixerController::new();
        let id = controller.add_strip();
        let events = controller.set_solo(id, true).unwrap();
        assert_eq!(events, vec![ChannelStripEvent::SoloChanged { solo: true }]);
        assert!(controller.strip(id).unwrap().solo());
    }

    #[test]
    fn set_solo_fails_for_unknown_strip() {
        let mut controller = MixerController::new();
        let id = controller.add_strip();
        controller.remove_strip(id).unwrap();
        assert_eq!(
            controller.set_solo(id, true),
            Err(MixerControllerError::StripNotFound(id))
        );
    }

    #[test]
    fn no_solo_active_means_audibility_follows_mute() {
        let mut controller = MixerController::new();
        let a = controller.add_strip();
        let b = controller.add_strip();
        controller
            .strips
            .get_mut(&b)
            .unwrap()
            .handle(ChannelStripCommand::SetMute { mute: true })
            .unwrap();

        assert!(!controller.any_solo_active());
        assert!(controller.is_audible(a).unwrap());
        assert!(!controller.is_audible(b).unwrap());
    }

    #[test]
    fn solo_in_place_silences_non_soloed_strips_even_if_unmuted() {
        let mut controller = MixerController::new();
        let a = controller.add_strip();
        let b = controller.add_strip();
        controller.set_solo(a, true).unwrap();

        assert!(controller.any_solo_active());
        assert!(controller.is_audible(a).unwrap());
        assert!(!controller.is_audible(b).unwrap());
    }

    #[test]
    fn unsoloing_last_soloed_strip_restores_mute_based_audibility() {
        let mut controller = MixerController::new();
        let a = controller.add_strip();
        controller.set_solo(a, true).unwrap();
        controller.set_solo(a, false).unwrap();

        assert!(!controller.any_solo_active());
        assert!(controller.is_audible(a).unwrap());
    }
}
