// path: src/loop/scene.rs

//! `Scene`: a named, ordered, serializable event sequence -- the unit of
//! control-plane demonstration and evaluation.
//!
//! A `Scene` is pure data: a name plus an ordered list of [`SceneStep`]s. It
//! carries no playback logic of its own -- the reducer/player that replays a
//! scene, and the `SnapshotCodec` port that turns a `Scene` into bytes and
//! back, are separate concerns living elsewhere. `Scene`'s only obligation
//! is to preserve its steps' order and content exactly, so that whatever
//! codec is layered over it can round-trip a scene file losslessly: every
//! field `Scene` exposes is exactly the field a codec needs to reconstruct
//! an equal `Scene` from bytes, and none are lost, reordered, or defaulted
//! away.

use crate::r#loop::scene_step::SceneStep;

/// A named, ordered sequence of [`SceneStep`]s.
///
/// Order matters: [`Scene::steps`] returns steps in the exact sequence they
/// were constructed with, and equality is order-sensitive, since the whole
/// point of a `Scene` is to describe a script that replays exactly as
/// demonstrated.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    name: String,
    steps: Vec<SceneStep>,
}

impl Scene {
    /// Constructs a new named `Scene` from an ordered list of steps.
    ///
    /// No step ordering or deduplication is performed here: `steps` is
    /// stored exactly as given, since the caller's ordering *is* the
    /// scene's recorded script.
    pub fn new(name: impl Into<String>, steps: Vec<SceneStep>) -> Self {
        Self {
            name: name.into(),
            steps,
        }
    }

    /// The scene's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The scene's steps, in their original recorded order.
    pub fn steps(&self) -> &[SceneStep] {
        &self.steps
    }

    /// The number of steps in this scene.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// True if this scene has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixer::mixer_view_event::MixerViewEvent;
    use crate::r#loop::scene_step::AppEvent;

    // `SceneStep` carries an `AppEvent` plus a `render_blocks` count, not a
    // label -- so these tests use `render_blocks` as the per-step
    // distinguishing marker (a fixed event is enough, since `Scene` itself
    // is agnostic to what a step's event is).
    fn step(render_blocks: u32) -> SceneStep {
        SceneStep::new(AppEvent::Mixer(MixerViewEvent::NavUp), render_blocks)
    }

    #[test]
    fn new_stores_name_and_steps() {
        let scene = Scene::new("Intro", vec![step(0), step(10)]);
        assert_eq!(scene.name(), "Intro");
        assert_eq!(scene.len(), 2);
        assert_eq!(scene.steps()[0].render_blocks(), 0);
        assert_eq!(scene.steps()[1].render_blocks(), 10);
    }

    #[test]
    fn new_accepts_owned_string_name() {
        let scene = Scene::new(String::from("Verse"), vec![]);
        assert_eq!(scene.name(), "Verse");
    }

    #[test]
    fn is_empty_true_for_no_steps() {
        let scene = Scene::new("Empty", vec![]);
        assert!(scene.is_empty());
        assert_eq!(scene.len(), 0);
    }

    #[test]
    fn is_empty_false_when_steps_present() {
        let scene = Scene::new("NotEmpty", vec![step(0)]);
        assert!(!scene.is_empty());
    }

    #[test]
    fn steps_preserve_recorded_order() {
        let scene = Scene::new("Ordered", vec![step(0), step(1), step(2)]);
        let render_blocks: Vec<u32> = scene.steps().iter().map(|s| s.render_blocks()).collect();
        assert_eq!(render_blocks, vec![0, 1, 2]);
    }

    #[test]
    fn equality_is_order_sensitive() {
        let forward = Scene::new("S", vec![step(0), step(1)]);
        let reversed = Scene::new("S", vec![step(1), step(0)]);
        assert_ne!(forward, reversed);
    }

    #[test]
    fn equality_compares_name_and_steps() {
        let a = Scene::new("S", vec![step(0)]);
        let b = Scene::new("S", vec![step(0)]);
        let different_name = Scene::new("T", vec![step(0)]);
        assert_eq!(a, b);
        assert_ne!(a, different_name);
    }

    #[test]
    fn clone_round_trips_to_an_equal_scene() {
        let original = Scene::new("RoundTrip", vec![step(0), step(5)]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(cloned.name(), "RoundTrip");
        assert_eq!(cloned.steps().len(), 2);
    }
}
