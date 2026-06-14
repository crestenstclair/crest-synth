// path: src/patch/mod.rs

pub mod channel_dispatcher;
pub mod channel_subscription;
pub mod global_mixer;
pub mod mpe_zone;
#[allow(clippy::module_inception)]
pub mod patch;
pub mod patch_id;
pub mod patch_mixer;
pub mod patch_repository;
pub mod voice_pool_config;
