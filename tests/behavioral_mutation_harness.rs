use crest_synth::testing::{
    BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn schema(observation: &BehavioralMutationObservation) -> BTreeSet<String> {
    serde_json::to_value(observation)
        .expect("observation serializes")
        .as_object()
        .expect("observation is a JSON object")
        .keys()
        .cloned()
        .collect()
}

fn expected_schema(case: BehavioralMutationCase) -> BTreeSet<String> {
    let fields: &[&str] = match case {
        BehavioralMutationCase::DroppedAdjustment => &[
            "case",
            "adjustment_dispatched",
            "adjust_event_recorded",
            "selected_value_exact",
            "unrelated_values_unchanged",
            "projection_values_exact",
            "baseline_restored",
        ],
        BehavioralMutationCase::CrossPatchParameterLeak => &[
            "case",
            "edited_patch_id",
            "comparison_patch_id",
            "patch_ids_distinct",
            "parameter",
            "parameter_cases_exercised",
            "edited_value_before",
            "edited_value_after",
            "comparison_value_before",
            "comparison_value_after",
            "published_edited_value",
            "published_comparison_value",
            "edited_stem_energy_before",
            "edited_stem_energy_after",
            "comparison_stem_energy_before",
            "comparison_stem_energy_after",
            "edited_value_changed",
            "comparison_value_unchanged",
            "state_values_exact",
            "published_values_exact",
            "edited_patch_audio_changed",
            "unedited_patch_audio_unchanged",
            "all_channel_parameters_isolated",
            "dry_path_isolated",
            "reverb_path_isolated",
            "delay_path_isolated",
            "baseline_restored",
        ],
        BehavioralMutationCase::PatchMisroute => &[
            "case",
            "command_patch_matches_event",
            "target_patch_received_command",
            "target_stem_changed",
            "untargeted_stems_unchanged",
            "patch_routing_exact",
        ],
        BehavioralMutationCase::OmittedStateTreeLeaf => &[
            "case",
            "schema_surface_equal",
            "required_leaf_count",
            "missing_leaf_count",
            "unexpected_leaf_count",
            "state_values_exact",
            "projection_values_exact",
        ],
        BehavioralMutationCase::DryToWetBypass => &[
            "case",
            "dry_input_energy",
            "zero_send_reverb_input_energy",
            "zero_send_delay_input_energy",
            "zero_send_wet_output_energy",
            "nonzero_send_reverb_input_energy",
            "nonzero_send_delay_input_energy",
            "nonzero_send_wet_output_energy",
            "identical_effect_state",
            "dry_bypass_absent",
            "finite_audio",
            "baseline_restored",
        ],
        BehavioralMutationCase::ZeroRenderer => &[
            "case",
            "control_trace_complete",
            "renderer_called",
            "renderer_nonzero",
            "render_peak",
            "finite_audio",
        ],
    };
    fields.iter().map(|field| (*field).to_owned()).collect()
}

fn measurements_differ(left: f64, right: f64) -> bool {
    (left - right).abs() > 1.0e-9
}

#[test]
fn every_named_mutant_falsifies_its_measured_healthy_predicate() {
    let harness = BehavioralMutationHarness::new();

    for case in BehavioralMutationCase::ALL {
        let healthy = harness.run(case, false);
        let mutant = harness.run(case, true);

        assert_eq!(healthy.exit_code(), 0, "{case:?}");
        assert_eq!(mutant.exit_code(), 1, "{case:?}");
        assert_eq!(healthy.observation().case(), case, "{case:?}");
        assert_eq!(mutant.observation().case(), case, "{case:?}");
        assert_eq!(schema(healthy.observation()), expected_schema(case));
        assert_eq!(schema(mutant.observation()), expected_schema(case));
        assert!(healthy.observation().satisfies_witness(), "{case:?}");
        assert!(mutant.observation().falsifies_witness(), "{case:?}");

        match (healthy.into_observation(), mutant.into_observation()) {
            (
                BehavioralMutationObservation::DroppedAdjustment(healthy),
                BehavioralMutationObservation::DroppedAdjustment(mutant),
            ) => {
                assert!(healthy.adjustment_dispatched);
                assert!(healthy.adjust_event_recorded);
                assert!(healthy.selected_value_exact);
                assert!(healthy.projection_values_exact);
                assert!(healthy.baseline_restored);
                assert!(!mutant.adjustment_dispatched);
                assert!(!mutant.adjust_event_recorded);
                assert!(!mutant.selected_value_exact);
                assert!(!mutant.projection_values_exact);
                assert!(!mutant.baseline_restored);
            }
            (
                BehavioralMutationObservation::CrossPatchParameterLeak(healthy),
                BehavioralMutationObservation::CrossPatchParameterLeak(mutant),
            ) => {
                assert_eq!(healthy.parameter_cases_exercised, 4);
                assert!(healthy.state_values_exact);
                assert!(healthy.published_values_exact);
                assert!(healthy.edited_patch_audio_changed);
                assert!(healthy.unedited_patch_audio_unchanged);
                assert!(healthy.all_channel_parameters_isolated);
                assert!(!measurements_differ(
                    healthy.comparison_stem_energy_before,
                    healthy.comparison_stem_energy_after,
                ));

                assert!(mutant.state_values_exact);
                assert!(mutant.published_values_exact);
                assert!(mutant.edited_patch_audio_changed);
                assert!(!mutant.unedited_patch_audio_unchanged);
                assert!(!mutant.all_channel_parameters_isolated);
                assert!(!mutant.dry_path_isolated);
                assert!(!mutant.reverb_path_isolated);
                assert!(!mutant.delay_path_isolated);
                assert!(measurements_differ(
                    mutant.comparison_stem_energy_before,
                    mutant.comparison_stem_energy_after,
                ));
            }
            (
                BehavioralMutationObservation::PatchMisroute(healthy),
                BehavioralMutationObservation::PatchMisroute(mutant),
            ) => {
                assert!(healthy.command_patch_matches_event);
                assert!(healthy.target_patch_received_command);
                assert!(healthy.target_stem_changed);
                assert!(healthy.untargeted_stems_unchanged);
                assert!(healthy.patch_routing_exact);
                assert!(!mutant.command_patch_matches_event);
                assert!(!mutant.target_patch_received_command);
                assert!(!mutant.target_stem_changed);
                assert!(!mutant.patch_routing_exact);
            }
            (
                BehavioralMutationObservation::OmittedStateTreeLeaf(healthy),
                BehavioralMutationObservation::OmittedStateTreeLeaf(mutant),
            ) => {
                assert!(healthy.schema_surface_equal);
                assert_eq!(healthy.missing_leaf_count, 0);
                assert_eq!(healthy.unexpected_leaf_count, 0);
                assert!(!mutant.schema_surface_equal);
                assert_eq!(mutant.required_leaf_count, healthy.required_leaf_count);
                assert_eq!(mutant.missing_leaf_count, 1);
                assert_eq!(mutant.unexpected_leaf_count, 0);
            }
            (
                BehavioralMutationObservation::DryToWetBypass(healthy),
                BehavioralMutationObservation::DryToWetBypass(mutant),
            ) => {
                assert!(healthy.dry_input_energy > 0.0);
                assert!(healthy.zero_send_reverb_input_energy.abs() < 1.0e-12);
                assert!(healthy.zero_send_delay_input_energy.abs() < 1.0e-12);
                assert!(healthy.zero_send_wet_output_energy.abs() < 1.0e-12);
                assert!(healthy.nonzero_send_wet_output_energy > 0.0);
                assert!(healthy.dry_bypass_absent);
                assert!(mutant.zero_send_wet_output_energy > 0.0);
                assert!(!mutant.dry_bypass_absent);
                assert!(mutant.identical_effect_state);
            }
            (
                BehavioralMutationObservation::ZeroRenderer(healthy),
                BehavioralMutationObservation::ZeroRenderer(mutant),
            ) => {
                assert!(healthy.control_trace_complete);
                assert!(healthy.renderer_called);
                assert!(healthy.renderer_nonzero);
                assert!(healthy.render_peak > 0.001);
                assert!(mutant.control_trace_complete);
                assert!(mutant.renderer_called);
                assert!(!mutant.renderer_nonzero);
                assert!(mutant.render_peak.abs() < 1.0e-12);
            }
            (healthy, mutant) => {
                panic!("case returned mismatched schemas: {healthy:?} / {mutant:?}");
            }
        }

        let healthy_json: Value = serde_json::from_str(
            &harness
                .run(case, false)
                .observation_json()
                .expect("healthy observation serializes"),
        )
        .expect("healthy JSON parses");
        let mutant_json: Value = serde_json::from_str(
            &harness
                .run(case, true)
                .observation_json()
                .expect("mutant observation serializes"),
        )
        .expect("mutant JSON parses");
        assert_eq!(
            healthy_json["case"],
            Value::String(case.as_str().to_owned())
        );
        assert_eq!(mutant_json["case"], Value::String(case.as_str().to_owned()));
    }

    println!("CREST_ACCEPTANCE behavioral_mutation_harness passed");
}
