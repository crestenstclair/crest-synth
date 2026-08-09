//! The `demo-live-patch-editor` scene: the functional Patch editor's live
//! demonstration, whose subject is the **second** installed Patch.
//!
//! Phase 3 proved the effect-slot journey for `patches.first()` only, because
//! the vocabulary had no patch-switching gesture. The gesture landed; this
//! scene is what it buys. Nothing here resolves a subject through
//! `patches.first()` — the subject is reached by dispatching
//! `SemanticAction::SelectPatch` through the production reducer and is then
//! asserted by id, and every editable target, effect-slot position, and
//! audible probe is derived from **that** Patch's own descriptors.

#[cfg(test)]
mod nfr_004_projection_throughput {
    //! NFR-004's measurement, owned by WP06 (F-23, F-30).
    //!
    //! F-30 re-measured the real number: the **full** projection pipeline per
    //! accepted event, release, 89 MIXER rows, is **3.00 ms** (2997 µs), not
    //! the 2.92 ms F-23 first reported — that earlier figure timed
    //! `SemanticGraphicalViewModel::project` alone. WP06 measures against
    //! 3.00 ms and grades honestly.
    //!
    //! F-29 is the cheap half of the remedy and is measured here beside it:
    //! each availability probe used to clone `AppState` twice, and the second
    //! clone is redundant because `AppState::apply` never writes a field on a
    //! rejected event. Hoisting one scratch clone per row takes a sweep from
    //! `2·|vocabulary|` clones to `1 + |vocabulary| + |accepted|`.
    //!
    //! Run it deliberately — it is a measurement, not a gate:
    //!
    //! ```text
    //! cargo test --release --lib -- --ignored --nocapture nfr_004
    //! ```

    use crate::control::app_event::AppEvent;
    use crate::control::app_state::{AppState, SemanticActionAvailability};
    use crate::control::{SemanticAction, StateProjector, TopLevelContext};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::patch_output::PatchOutput;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use std::time::Instant;

    /// Installs the production SoundFont registry and effect registry with two
    /// Patches, exactly the shape the live fixture starts from.
    fn installed_state() -> AppState {
        let provider = crate::adapter::production_instruments::production_soundfont_capability()
            .expect("the production SoundFont capability composes");
        let registry = provider
            .registry()
            .expect("the production registry composes");
        let effects = crate::adapter::production_effects::production_effect_registry()
            .expect("the production effect registry composes");
        let mut state = AppState::new_with_effects(
            registry,
            effects,
            GlobalParameters::new(0.0).expect("0 dB is a valid master gain"),
        );
        let patch = |id: u32, channel: u8| {
            Patch::new(
                PatchId::new(id).expect("a one-based fixture id is valid"),
                format!("Patch {id}"),
                crate::testing::automatic_midi_test::create_soundfont_config(
                    &provider,
                    SoundFontInstrument::new(0, id as u8, false).expect("fixture preset is valid"),
                )
                .expect("the fixture instrument config composes"),
                MidiChannel::new(channel).expect("fixture channel is valid"),
                PatchOutput::new(
                    MixerTrackId::new(channel).expect("fixture track is valid"),
                    0.0,
                )
                .expect("fixture output is valid"),
            )
        };
        state
            .apply(AppEvent::InstallPatches(vec![patch(1, 0), patch(2, 1)]))
            .expect("installing the fixture Patches is accepted");
        state
    }

    /// The pre-F-29 sweep, written out literally so the comparison measures
    /// the change rather than a description of it: one clone to obtain a
    /// `&mut`, one more inside `apply`, for every action in the vocabulary.
    fn baseline_sweep(state: &AppState) -> usize {
        SemanticAction::surface_descriptor()
            .iter()
            .filter(|action| {
                let mut candidate = state.clone();
                candidate.apply_semantic_action((*action).clone()).is_ok()
            })
            .count()
    }

    /// The post-F-29 sweep: one scratch clone per row, reused across refusals.
    fn hoisted_sweep(state: &AppState) -> usize {
        let mut availability = SemanticActionAvailability::new(state);
        SemanticAction::surface_descriptor()
            .iter()
            .filter(|action| availability.accepts(action))
            .count()
    }

    fn median_micros(mut samples: Vec<f64>) -> f64 {
        samples.sort_by(|left, right| left.partial_cmp(right).expect("timings are finite"));
        samples[samples.len() / 2]
    }

    #[test]
    #[ignore = "a measurement, not a gate; run with --release --ignored --nocapture"]
    fn nfr_004_full_projection_pipeline_per_accepted_event() {
        let mut mixer = installed_state();
        mixer
            .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
            .expect("MIXER is selectable");
        let mut patch = installed_state();
        patch
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .expect("PATCH is selectable");

        for (name, state) in [("MIXER Main", &mixer), ("PATCH Main", &patch)] {
            let projector = StateProjector::new();
            let (_, _, _, shell, _) = projector
                .project_with_shell(state)
                .expect("the fixture state projects");
            let rows: usize = shell
                .semantic_model()
                .surfaces()
                .iter()
                .map(|surface| surface.controls().len())
                .sum();

            // Warm.
            for _ in 0..3 {
                let _ = projector.project_with_shell(state);
            }
            let mut samples = Vec::new();
            for _ in 0..15 {
                let started = Instant::now();
                let projected = projector.project_with_shell(state);
                samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
                assert!(projected.is_ok(), "the fixture state projects");
            }
            let full = median_micros(samples);

            let mut sweep_baseline = Vec::new();
            let mut sweep_hoisted = Vec::new();
            for _ in 0..2_000 {
                let started = Instant::now();
                let accepted = baseline_sweep(state);
                sweep_baseline.push(started.elapsed().as_secs_f64() * 1_000_000.0);
                let started = Instant::now();
                let hoisted = hoisted_sweep(state);
                sweep_hoisted.push(started.elapsed().as_secs_f64() * 1_000_000.0);
                assert_eq!(
                    accepted, hoisted,
                    "the hoisted sweep must answer the identical question",
                );
            }
            let baseline = median_micros(sweep_baseline);
            let hoisted = median_micros(sweep_hoisted);

            println!(
                "NFR-004 {name}: rows={rows} full project_with_shell={full:.0}us \
                 per-row availability sweep baseline={baseline:.2}us hoisted={hoisted:.2}us \
                 (projected saving over {rows} rows = {:.0}us)",
                (baseline - hoisted) * rows as f64,
            );
        }
    }

    /// The property F-29's hoist rests on, proved rather than assumed: a
    /// rejected `apply_semantic_action` leaves the state it was given
    /// byte-for-byte identical, so one scratch may be reused across refusals.
    ///
    /// This is the whole correctness argument. If any reducer path ever
    /// mutates before rejecting, this test fails and the hoist is unsound.
    #[test]
    fn a_refused_semantic_action_never_touches_the_state_it_was_given() {
        let base = installed_state();
        let mut states = vec![base.clone()];
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let mut state = base.clone();
            state
                .apply(AppEvent::SelectContext(context))
                .expect("both contexts are selectable");
            states.push(state.clone());
            for surface in [
                crate::control::SurfaceId::PatchUtility,
                crate::control::SurfaceId::MixerInspector,
            ] {
                let mut entered = state.clone();
                if entered.apply(AppEvent::EnterSurface(surface)).is_ok() {
                    states.push(entered);
                }
            }
            let mut adjusting = state;
            if adjusting
                .apply(AppEvent::SetInteractionMode(
                    crate::control::InteractionMode::Adjust,
                ))
                .is_ok()
            {
                states.push(adjusting);
            }
        }

        let mut refusals = 0_usize;
        for state in &states {
            for action in SemanticAction::surface_descriptor() {
                let mut candidate = state.clone();
                if candidate.apply_semantic_action(action.clone()).is_err() {
                    refusals += 1;
                    assert_eq!(
                        &candidate, state,
                        "a refused {action:?} must leave the candidate identical",
                    );
                }
            }
        }
        assert!(
            refusals > 0,
            "the fixture must actually exercise refusals for this to prove anything",
        );
    }

    /// The hoisted sweep and the one-shot probe must answer identically for
    /// every action on every reachable fixture state. Same reducer, same
    /// question — this is what keeps the per-row list from drifting.
    #[test]
    fn the_hoisted_sweep_agrees_with_the_one_shot_probe_everywhere() {
        let base = installed_state();
        let mut states = vec![base.clone()];
        for context in [TopLevelContext::Patch, TopLevelContext::Mixer] {
            let mut state = base.clone();
            state
                .apply(AppEvent::SelectContext(context))
                .expect("both contexts are selectable");
            states.push(state);
        }
        for state in &states {
            let one_shot: Vec<bool> = SemanticAction::surface_descriptor()
                .iter()
                .map(|action| state.accepts_semantic_action(action))
                .collect();
            let mut availability = SemanticActionAvailability::new(state);
            let swept: Vec<bool> = SemanticAction::surface_descriptor()
                .iter()
                .map(|action| availability.accepts(action))
                .collect();
            assert_eq!(one_shot, swept);
        }
    }
}
