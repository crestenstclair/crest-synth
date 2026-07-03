// path: src/plugin/plugin_parameter.rs

//! `PluginParameter` — a single host-automatable parameter exposed by a
//! `PluginInstance`.
//!
//! Each `PluginParameter` maps 1:1 to an engine parameter (`engine_mapping`
//! names the engine-side target it forwards to) and carries a stable
//! numeric `ParameterId` so that automation recorded by a host survives
//! plugin version upgrades. `PluginParameter` is a plain data entity: it
//! holds no dependency on any other class, performs no I/O, and never
//! touches the audio thread directly — `PluginInstance` is responsible for
//! publishing the resulting value across the `ParameterBridge`.

use crate::kernel::parameter_id::ParameterId;
use crate::kernel::parameter_range::ParameterRange;

/// Error returned when constructing or updating a `PluginParameter` with a
/// value outside its declared `ParameterRange`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PluginParameterValueError {
    value: f64,
    range: ParameterRange,
}

impl std::fmt::Display for PluginParameterValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "value {} is outside parameter range [{}, {}]",
            self.value,
            self.range.min(),
            self.range.max()
        )
    }
}

impl std::error::Error for PluginParameterValueError {}

/// A single plugin-exposed parameter: a stable ID, a display name, the
/// engine-side mapping it forwards to, its valid range, and its current
/// value.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginParameter {
    id: ParameterId,
    name: String,
    engine_mapping: String,
    range: ParameterRange,
    current_value: f64,
}

impl PluginParameter {
    /// Constructs a `PluginParameter`.
    ///
    /// Returns `Err` if `current_value` falls outside `range`.
    ///
    /// ```
    /// use crest_synth::kernel::parameter_id::ParameterId;
    /// use crest_synth::kernel::parameter_range::ParameterRange;
    /// use crest_synth::plugin::plugin_parameter::PluginParameter;
    ///
    /// let range = ParameterRange::try_new(0.0, 1.0).unwrap();
    /// let param = PluginParameter::try_new(
    ///     ParameterId::new(0),
    ///     "Cutoff",
    ///     "engine.filter.cutoff",
    ///     range,
    ///     0.5,
    /// )
    /// .unwrap();
    /// assert_eq!(param.current_value(), 0.5);
    /// ```
    pub fn try_new(
        id: ParameterId,
        name: impl Into<String>,
        engine_mapping: impl Into<String>,
        range: ParameterRange,
        current_value: f64,
    ) -> Result<Self, PluginParameterValueError> {
        if !range.contains(current_value) {
            return Err(PluginParameterValueError {
                value: current_value,
                range,
            });
        }
        Ok(Self {
            id,
            name: name.into(),
            engine_mapping: engine_mapping.into(),
            range,
            current_value,
        })
    }

    /// The parameter's stable numeric ID.
    #[inline]
    pub fn id(&self) -> ParameterId {
        self.id
    }

    /// The parameter's display name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The engine-side parameter this one maps to 1:1.
    #[inline]
    pub fn engine_mapping(&self) -> &str {
        &self.engine_mapping
    }

    /// The valid `[min, max]` range for this parameter's value.
    #[inline]
    pub fn range(&self) -> ParameterRange {
        self.range
    }

    /// The parameter's current value.
    #[inline]
    pub fn current_value(&self) -> f64 {
        self.current_value
    }

    /// Sets the parameter's current value.
    ///
    /// Returns `Err` (leaving the current value unchanged) if `value` falls
    /// outside `range`.
    pub fn set_value(&mut self, value: f64) -> Result<(), PluginParameterValueError> {
        if !self.range.contains(value) {
            return Err(PluginParameterValueError {
                value,
                range: self.range,
            });
        }
        self.current_value = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range() -> ParameterRange {
        ParameterRange::try_new(0.0, 1.0).unwrap()
    }

    #[test]
    fn try_new_accepts_a_value_within_range() {
        let param =
            PluginParameter::try_new(ParameterId::new(1), "Cutoff", "engine.filter.cutoff", range(), 0.5)
                .unwrap();
        assert_eq!(param.id(), ParameterId::new(1));
        assert_eq!(param.name(), "Cutoff");
        assert_eq!(param.engine_mapping(), "engine.filter.cutoff");
        assert!((param.current_value() - 0.5).abs() < f64::EPSILON);
        assert_eq!(param.range(), range());
    }

    #[test]
    fn try_new_accepts_values_on_the_boundary() {
        assert!(PluginParameter::try_new(ParameterId::new(1), "Cutoff", "m", range(), 0.0).is_ok());
        assert!(PluginParameter::try_new(ParameterId::new(1), "Cutoff", "m", range(), 1.0).is_ok());
    }

    #[test]
    fn try_new_rejects_a_value_outside_range() {
        let result =
            PluginParameter::try_new(ParameterId::new(1), "Cutoff", "engine.filter.cutoff", range(), 5.0);
        assert!(result.is_err());
    }

    #[test]
    fn set_value_updates_current_value_when_in_range() {
        let mut param =
            PluginParameter::try_new(ParameterId::new(2), "Resonance", "engine.filter.q", range(), 0.2)
                .unwrap();
        param.set_value(0.9).unwrap();
        assert!((param.current_value() - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn set_value_rejects_out_of_range_and_leaves_value_unchanged() {
        let mut param =
            PluginParameter::try_new(ParameterId::new(2), "Resonance", "engine.filter.q", range(), 0.2)
                .unwrap();
        let result = param.set_value(2.0);
        assert!(result.is_err());
        assert!((param.current_value() - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn distinct_parameters_are_distinguished_by_id() {
        let a =
            PluginParameter::try_new(ParameterId::new(1), "Cutoff", "engine.filter.cutoff", range(), 0.5)
                .unwrap();
        let b =
            PluginParameter::try_new(ParameterId::new(2), "Cutoff", "engine.filter.cutoff", range(), 0.5)
                .unwrap();
        assert_ne!(a.id(), b.id());
    }
}
