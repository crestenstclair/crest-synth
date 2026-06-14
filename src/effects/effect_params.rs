// path: src/effects/effect_params.rs
//
// EffectParams is defined in effect_processor::EffectParams.
// This module re-exports it for convenience so other modules can write
// `use crate::effects::effect_params::EffectParams` without coupling to the
// processor implementation module.

pub use crate::effects::effect_processor::EffectParams;
