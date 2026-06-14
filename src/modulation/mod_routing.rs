// path: src/modulation/mod_routing.rs

use crate::modulation::mod_destination_type::ModDestinationType;
use crate::modulation::mod_source_type::ModSourceType;

/// A single modulation routing: one source mapped to one destination with a
/// signed depth.
///
/// `ModRouting` is a pure data value with no audio-thread allocation and no
/// locks. A collection of `ModRouting`s forms a modulation matrix row set.
///
/// # Invariants
/// - `depth` must be in `[-1.0, 1.0]` and must not be NaN.
///
/// # Examples
///
/// ```
/// use crest_synth::modulation::mod_routing::ModRouting;
/// use crest_synth::modulation::mod_source_type::ModSourceType;
/// use crest_synth::modulation::mod_destination_type::ModDestinationType;
///
/// let routing = ModRouting::try_new(
///     ModSourceType::Lfo,
///     ModDestinationType::FilterCutoff,
///     0.5,
/// ).unwrap();
/// assert_eq!(routing.source(), ModSourceType::Lfo);
/// assert_eq!(routing.destination(), ModDestinationType::FilterCutoff);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModRouting {
    source: ModSourceType,
    destination: ModDestinationType,
    /// Signed modulation depth in `[-1.0, 1.0]`.
    depth: f64,
}

/// Error returned when `ModRouting` construction violates an invariant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModRoutingError {
    /// `depth` was NaN or outside `[-1.0, 1.0]`.
    DepthOutOfRange(f64),
}

impl std::fmt::Display for ModRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModRoutingError::DepthOutOfRange(v) => {
                write!(f, "ModRouting: depth must be in [-1.0, 1.0], got {v}")
            }
        }
    }
}

impl std::error::Error for ModRoutingError {}

impl ModRouting {
    /// Construct a validated `ModRouting`.
    ///
    /// Returns `Err` when `depth` is NaN or outside `[-1.0, 1.0]`.
    pub fn try_new(
        source: ModSourceType,
        destination: ModDestinationType,
        depth: f64,
    ) -> Result<Self, ModRoutingError> {
        if depth.is_nan() || !(-1.0..=1.0).contains(&depth) {
            return Err(ModRoutingError::DepthOutOfRange(depth));
        }
        Ok(Self {
            source,
            destination,
            depth,
        })
    }

    /// Returns the modulation source.
    #[inline]
    pub fn source(self) -> ModSourceType {
        self.source
    }

    /// Returns the modulation destination.
    #[inline]
    pub fn destination(self) -> ModDestinationType {
        self.destination
    }

    /// Returns the signed depth in `[-1.0, 1.0]`.
    #[inline]
    pub fn depth(self) -> f64 {
        self.depth
    }

    /// Returns `true` if this routing carries per-note expression to a
    /// per-voice destination — i.e. both the source is a per-note expression
    /// dimension and the destination is per-voice.
    ///
    /// Such routings must never be aggregated at the patch level; they must
    /// be delivered to each voice independently.
    #[inline]
    pub fn is_per_note_expression_routing(self) -> bool {
        self.source.is_per_note_expression() && self.destination.is_per_voice()
    }
}

impl Default for ModRouting {
    /// Returns a unity-depth routing from LFO to filter cutoff.
    fn default() -> Self {
        Self {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(source: ModSourceType, dest: ModDestinationType, depth: f64) -> ModRouting {
        ModRouting::try_new(source, dest, depth).unwrap()
    }

    #[test]
    fn valid_positive_depth_accepted() {
        let r = make(ModSourceType::Lfo, ModDestinationType::FilterCutoff, 0.75);
        assert_eq!(r.depth(), 0.75);
    }

    #[test]
    fn valid_negative_depth_accepted() {
        let r = make(ModSourceType::Envelope, ModDestinationType::Amplitude, -0.5);
        assert_eq!(r.depth(), -0.5);
    }

    #[test]
    fn zero_depth_accepted() {
        let r = make(
            ModSourceType::Velocity,
            ModDestinationType::FilterCutoff,
            0.0,
        );
        assert_eq!(r.depth(), 0.0);
    }

    #[test]
    fn boundary_depths_accepted() {
        assert!(
            ModRouting::try_new(ModSourceType::Lfo, ModDestinationType::Amplitude, 1.0).is_ok()
        );
        assert!(
            ModRouting::try_new(ModSourceType::Lfo, ModDestinationType::Amplitude, -1.0).is_ok()
        );
    }

    #[test]
    fn depth_above_one_rejected() {
        let e = ModRouting::try_new(ModSourceType::Lfo, ModDestinationType::FilterCutoff, 1.001);
        assert_eq!(e, Err(ModRoutingError::DepthOutOfRange(1.001)));
    }

    #[test]
    fn depth_below_neg_one_rejected() {
        let e = ModRouting::try_new(ModSourceType::Lfo, ModDestinationType::FilterCutoff, -1.001);
        assert_eq!(e, Err(ModRoutingError::DepthOutOfRange(-1.001)));
    }

    #[test]
    fn nan_depth_rejected() {
        let e = ModRouting::try_new(
            ModSourceType::Lfo,
            ModDestinationType::FilterCutoff,
            f64::NAN,
        );
        assert!(matches!(e, Err(ModRoutingError::DepthOutOfRange(_))));
    }

    #[test]
    fn source_accessor() {
        let r = make(ModSourceType::Velocity, ModDestinationType::Amplitude, 0.8);
        assert_eq!(r.source(), ModSourceType::Velocity);
    }

    #[test]
    fn destination_accessor() {
        let r = make(
            ModSourceType::Velocity,
            ModDestinationType::OscillatorPitch,
            0.3,
        );
        assert_eq!(r.destination(), ModDestinationType::OscillatorPitch);
    }

    #[test]
    fn per_note_expression_routing_detected() {
        let r = make(
            ModSourceType::PerNoteBendX,
            ModDestinationType::OscillatorPitch,
            1.0,
        );
        assert!(r.is_per_note_expression_routing());
    }

    #[test]
    fn non_per_note_source_is_not_expression_routing() {
        let r = make(ModSourceType::Lfo, ModDestinationType::OscillatorPitch, 1.0);
        assert!(!r.is_per_note_expression_routing());
    }

    #[test]
    fn per_note_source_to_global_dest_is_not_expression_routing() {
        let r = make(
            ModSourceType::PerNoteBendX,
            ModDestinationType::LfoRate(0),
            0.5,
        );
        assert!(!r.is_per_note_expression_routing());
    }

    #[test]
    fn copy_semantics() {
        let a = make(
            ModSourceType::Envelope,
            ModDestinationType::FilterCutoff,
            0.6,
        );
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn error_display_contains_value() {
        let e = ModRoutingError::DepthOutOfRange(2.0);
        assert!(e.to_string().contains("2"));
    }

    #[test]
    fn default_routing_is_valid() {
        let r = ModRouting::default();
        assert!((-1.0..=1.0).contains(&r.depth()));
    }
}
