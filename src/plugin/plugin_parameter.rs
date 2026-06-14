// path: src/plugin/plugin_parameter.rs
//
// PluginParameter — a single automatable parameter exposed to a DAW host.
//
// A `PluginParameter` binds a host-visible parameter slot (identified by a
// stable `ParameterId`) to a named engine mapping string and a current value
// constrained within a `ParameterRange`.  The numeric ID is stable across
// plugin versions so host automation data is never broken by internal
// refactors.

use crate::plugin::parameter_id::ParameterId;
use crate::plugin::parameter_range::ParameterRange;

/// A single automatable parameter exposed to a DAW host.
///
/// # Identity and stability
///
/// A `PluginParameter`'s `id` is its stable numeric handle. DAW projects
/// store automation data keyed by this number; it must never change across
/// plugin versions.
///
/// # Engine mapping
///
/// `engine_mapping` is a string key used by the engine to route the
/// parameter value to the correct internal target (e.g. `"filter.cutoff"`,
/// `"osc.detune"`).  The format is an opaque contract between the plugin
/// wrapper and the engine — the host never interprets it.
///
/// # Current value
///
/// `current_value` is always constrained within `range`.  Use
/// [`PluginParameter::set_value`] to update it safely.
///
/// ```
/// use crest_synth::plugin::parameter_id::ParameterId;
/// use crest_synth::plugin::parameter_range::ParameterRange;
/// use crest_synth::plugin::plugin_parameter::PluginParameter;
///
/// let range = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
/// let mut param = PluginParameter::new(
///     ParameterId::new(1),
///     "Filter Cutoff".to_string(),
///     "filter.cutoff".to_string(),
///     range,
/// );
///
/// param.set_value(0.75);
/// assert_eq!(param.current_value(), 0.75);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PluginParameter {
    /// Stable host-facing numeric ID.
    id: ParameterId,
    /// Human-readable name shown in the DAW's automation lane.
    name: String,
    /// Engine-side mapping key (opaque to the host).
    engine_mapping: String,
    /// Allowed value range and default.
    range: ParameterRange,
    /// Current parameter value; always within `range`.
    current_value: f64,
}

impl PluginParameter {
    /// Create a new `PluginParameter` initialised to `range.default_value`.
    ///
    /// `id` — stable numeric identifier; never renumber across versions.
    /// `name` — human-readable label (e.g. `"Filter Cutoff"`).
    /// `engine_mapping` — key forwarded to the engine (e.g. `"filter.cutoff"`).
    /// `range` — allowed value range including the default value.
    pub fn new(
        id: ParameterId,
        name: String,
        engine_mapping: String,
        range: ParameterRange,
    ) -> Self {
        let current_value = range.default_value;
        Self {
            id,
            name,
            engine_mapping,
            range,
            current_value,
        }
    }

    /// Return the stable numeric parameter ID.
    #[inline]
    pub fn id(&self) -> ParameterId {
        self.id
    }

    /// Return the human-readable parameter name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the engine-side mapping key.
    #[inline]
    pub fn engine_mapping(&self) -> &str {
        &self.engine_mapping
    }

    /// Return the allowed value range.
    #[inline]
    pub fn range(&self) -> &ParameterRange {
        &self.range
    }

    /// Return the current parameter value (always within `range`).
    #[inline]
    pub fn current_value(&self) -> f64 {
        self.current_value
    }

    /// Set the parameter to `value`, clamping it into `[range.min, range.max]`.
    ///
    /// `NaN` values are clamped to `range.min`.
    #[inline]
    pub fn set_value(&mut self, value: f64) {
        let clamped = if value.is_nan() {
            self.range.min
        } else {
            value.clamp(self.range.min, self.range.max)
        };
        self.current_value = clamped;
    }

    /// Set the parameter from a host-normalised `[0.0, 1.0]` value.
    ///
    /// Maps the normalised value linearly to `[range.min, range.max]`.
    #[inline]
    pub fn set_normalised(&mut self, normal: f64) {
        let n = normal.clamp(0.0, 1.0);
        self.current_value = self.range.min + n * (self.range.max - self.range.min);
    }

    /// Return the current value as a normalised `[0.0, 1.0]` float.
    #[inline]
    pub fn normalised_value(&self) -> f64 {
        let span = self.range.max - self.range.min;
        if span.abs() < f64::EPSILON {
            return 0.0;
        }
        (self.current_value - self.range.min) / span
    }

    /// Reset the parameter to its range default.
    #[inline]
    pub fn reset(&mut self) {
        self.current_value = self.range.default_value;
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_range() -> ParameterRange {
        ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap()
    }

    fn make_param(id: u32) -> PluginParameter {
        PluginParameter::new(
            ParameterId::new(id),
            format!("Param {id}"),
            format!("engine.param_{id}"),
            make_range(),
        )
    }

    // ─── PluginParameter construction ────────────────────────────────────────

    #[test]
    fn parameter_initialises_to_default() {
        let p = make_param(0);
        assert_eq!(p.current_value(), 0.5);
    }

    #[test]
    fn parameter_stores_id() {
        let p = make_param(7);
        assert_eq!(p.id().get(), 7);
    }

    #[test]
    fn parameter_stores_name() {
        let p = make_param(0);
        assert_eq!(p.name(), "Param 0");
    }

    #[test]
    fn parameter_stores_engine_mapping() {
        let p = make_param(0);
        assert_eq!(p.engine_mapping(), "engine.param_0");
    }

    #[test]
    fn parameter_range_is_accessible() {
        let p = make_param(0);
        assert_eq!(p.range().min, 0.0);
        assert_eq!(p.range().max, 1.0);
        assert_eq!(p.range().default_value, 0.5);
    }

    // ─── set_value ───────────────────────────────────────────────────────────

    #[test]
    fn set_value_updates_current_value() {
        let mut p = make_param(0);
        p.set_value(0.9);
        assert_eq!(p.current_value(), 0.9);
    }

    #[test]
    fn set_value_clamps_above_max() {
        let mut p = make_param(0);
        p.set_value(5.0);
        assert_eq!(p.current_value(), 1.0);
    }

    #[test]
    fn set_value_clamps_below_min() {
        let mut p = make_param(0);
        p.set_value(-5.0);
        assert_eq!(p.current_value(), 0.0);
    }

    #[test]
    fn set_value_nan_clamps_to_min() {
        let mut p = make_param(0);
        p.set_value(f64::NAN);
        assert_eq!(p.current_value(), 0.0);
    }

    #[test]
    fn set_value_at_min_boundary() {
        let mut p = make_param(0);
        p.set_value(0.0);
        assert_eq!(p.current_value(), 0.0);
    }

    #[test]
    fn set_value_at_max_boundary() {
        let mut p = make_param(0);
        p.set_value(1.0);
        assert_eq!(p.current_value(), 1.0);
    }

    // ─── set_normalised / normalised_value ───────────────────────────────────

    #[test]
    fn set_normalised_zero_gives_min() {
        let mut p = make_param(0);
        p.set_normalised(0.0);
        assert!((p.current_value() - 0.0).abs() < 1e-12);
    }

    #[test]
    fn set_normalised_one_gives_max() {
        let mut p = make_param(0);
        p.set_normalised(1.0);
        assert!((p.current_value() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn set_normalised_half_gives_midpoint() {
        let mut p = make_param(0);
        p.set_normalised(0.5);
        assert!((p.current_value() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn normalised_value_round_trip() {
        let mut p = make_param(0);
        p.set_value(0.75);
        assert!((p.normalised_value() - 0.75).abs() < 1e-12);
    }

    #[test]
    fn normalised_value_at_default() {
        let p = make_param(0);
        // default is 0.5 in [0,1] → normalised = 0.5
        assert!((p.normalised_value() - 0.5).abs() < 1e-12);
    }

    // ─── reset ───────────────────────────────────────────────────────────────

    #[test]
    fn reset_returns_to_default() {
        let mut p = make_param(0);
        p.set_value(0.9);
        p.reset();
        assert_eq!(p.current_value(), 0.5);
    }

    // ─── identity / clone ────────────────────────────────────────────────────

    #[test]
    fn parameter_ids_are_stable_across_clones() {
        let p = make_param(99);
        let q = p.clone();
        assert_eq!(p.id(), q.id());
    }

    #[test]
    fn two_parameters_with_different_ids_are_distinguishable() {
        let p0 = make_param(0);
        let p1 = make_param(1);
        assert_ne!(p0.id(), p1.id());
    }

    #[test]
    fn cloned_parameter_is_independent() {
        let p = make_param(0);
        let mut q = p.clone();
        q.set_value(0.9);
        assert_eq!(p.current_value(), 0.5); // original unchanged
        assert_eq!(q.current_value(), 0.9);
    }

    // ─── negative / wide ranges ──────────────────────────────────────────────

    #[test]
    fn negative_range_set_value_works() {
        let range = ParameterRange::try_new(-100.0, -10.0, -50.0, None).unwrap();
        let mut p = PluginParameter::new(
            ParameterId::new(0),
            "Pan".to_string(),
            "osc.pan".to_string(),
            range,
        );
        assert_eq!(p.current_value(), -50.0);
        p.set_value(-20.0);
        assert_eq!(p.current_value(), -20.0);
        p.set_value(0.0); // above max → clamped
        assert_eq!(p.current_value(), -10.0);
    }

    #[test]
    fn set_normalised_clamps_out_of_range_normal() {
        let mut p = make_param(0);
        p.set_normalised(-1.0); // below 0 → clamped to 0 → min
        assert_eq!(p.current_value(), 0.0);
        p.set_normalised(2.0); // above 1 → clamped to 1 → max
        assert_eq!(p.current_value(), 1.0);
    }
}
