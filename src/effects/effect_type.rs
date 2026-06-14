// path: src/effects/effect_type.rs
//
// EffectType is defined in effect_processor::EffectType.
// This module re-exports it for convenience so other modules can write
// `use crate::effects::effect_type::EffectType` without coupling to the
// processor implementation module.

pub use crate::effects::effect_processor::EffectType;
