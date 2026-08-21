# Phase 7 requirement-to-proof matrix

This is an evidence index for the three Phase 7 delta specifications. Product
authority remains `DESIGN.md`; the scenario wording remains in the OpenSpec
delta. Every proof below crosses a production reducer, projection, adapter,
preparation, or render seam. Test fixtures supply ports and bytes, but no test
mutates a view model or a callback-owned engine to manufacture acceptance.

## Patch detail and choice workflows

| Spec scenario | Production-path proof |
| --- | --- |
| Different capabilities reuse the same shell | `app_state::a_braids_detail_state_projects_even_though_its_row_is_absent_from_the_main_order`; `semantic_graphical_view_model::detail_controls_project_descriptor_owned_editability`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Descriptor dependencies change | `phase7_sample_workflows::sample_detail_and_browser_project_generic_sections_rows_status_and_visualizations`; `phase7_sample_workflows::detail_numeric_and_choice_edits_use_the_reducer_and_return_to_exact_rows` |
| Return from instrument detail | `interaction_state::one_choice_session_replaces_detail_and_returns_through_both_exact_origins`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Origin disappears during reprojection | `app_state::an_engine_swap_under_an_open_detail_entry_leaves_the_surface`; `app_state::clearing_the_subject_slot_under_an_open_detail_entry_leaves_the_surface` |
| Navigate across a visualization | `phase7_sample_workflows::sample_detail_and_browser_project_generic_sections_rows_status_and_visualizations`; `semantic_graphical_view_model::detail_controls_project_descriptor_owned_editability` |
| Open engine choices | `phase7_acceptance::phase7_open_engine_choices_uses_registry_order_and_choose_returns_exactly` |
| Open route choices | `phase7_acceptance::phase7_route_choice_has_all_tracks_and_cancel_is_exact_and_unchanged` |
| Edit+Up targets a scalar | `phase7_sample_workflows::detail_numeric_and_choice_edits_use_the_reducer_and_return_to_exact_rows`; `keyboard_input_translator::shift_vertical_maps_single_level_open_and_return_without_leaking_modifier_identity` |
| Choose an option | `phase7_acceptance::phase7_open_engine_choices_uses_registry_order_and_choose_returns_exactly`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Cancel an option | `phase7_acceptance::phase7_route_choice_has_all_tracks_and_cancel_is_exact_and_unchanged`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Attempt to leave the modal spatially | `phase7_acceptance::phase7_route_choice_has_all_tracks_and_cancel_is_exact_and_unchanged`; `phase7_sample_workflows::browser_focus_is_stable_trapped_nonwrapping_and_preview_stops_on_navigation_and_cancel` |
| Choice prepares successfully | `phase7_sample_workflows::coordinator_advances_assignment_through_activation_before_committing_the_asset`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Choice preparation fails | `app_state::app_state_engine_failure_and_stale_or_mismatched_outcomes_preserve_source_and_recover`; `phase7_sample_workflows::asset_assignment_is_correlated_failure_safe_and_ready_only_after_activation` |
| Reflow an open detail | `phase7_sample_workflows::sample_detail_and_browser_project_generic_sections_rows_status_and_visualizations`; serialized documents pass `webview_projection_shell` T022, while live 1920×1080/1280×800 T024 remains incomplete under tasks 11.5–11.6 |
| Reflow an open modal | `phase7_sample_workflows::sample_detail_and_browser_project_generic_sections_rows_status_and_visualizations`; serialized modal/browser documents pass T022, while live 1920×1080/1280×800 T024 remains incomplete under tasks 11.5–11.6 |

## Sample capability and browser

| Spec scenario | Production-path proof |
| --- | --- |
| Admit a stereo fixture | `wav_sample_decoder::decodes_admitted_mono_pcm16_and_stereo_float32`; `wav_sample_decoder::decodes_admitted_pcm24_and_pcm32_without_dependency_types_escaping` |
| Reject an unsupported asset | `wav_sample_decoder::renamed_non_wave_and_non_finite_float_are_typed_rejections`; `wav_sample_decoder::rejects_rifx_rf64_channels_rates_depth_and_duration_from_headers` |
| Resolve a valid library asset | `filesystem_sample_catalog::listing_is_stable_folder_first_and_ids_are_library_relative` |
| Reject library escape | `filesystem_sample_catalog::traversal_and_symlink_escape_are_typed_and_never_read` |
| Play at the root pitch | `sample_preparer::root_and_octave_ratios_use_linear_interpolation_and_preserve_channels` |
| Exceed polyphony | `sample_preparer::seventeenth_note_steals_oldest_releasing_before_oldest_active` |
| Render a forward loop | `sample_preparer::forward_loop_wraps_crossfades_and_never_reads_outside_pcm`; `sample_preparer::maximum_forward_crossfade_blends_the_last_loop_neighbor_without_out_of_range_reads` |
| Reject invalid landmarks | `sample_capability::configs_accept_same_kind_relative_asset_and_validate_playback_contract` |
| Prepare within limits | `sample_preparer::preparer_deduplicates_pcm_by_stable_asset_and_device_rate`; `phase7_sample_workflows::coordinator_advances_assignment_through_activation_before_committing_the_asset` |
| Exceed the graph PCM budget | `sample_preparation::graph_budget_deduplicates_same_preparation_identity`; `saved_session::restore_prepares_before_commit_and_keeps_unavailable_invalid_and_over_budget_typed` |
| Commit a valid sample | `phase7_sample_workflows::coordinator_advances_assignment_through_activation_before_committing_the_asset`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| A newer request supersedes preparation | `phase7_sample_workflows::asset_assignment_is_correlated_failure_safe_and_ready_only_after_activation`; `phase7_sample_workflows::preview_rejects_start_outside_browser_and_stale_preparation_without_mutation` |
| Cancel the browser | `phase7_sample_workflows::browser_focus_is_stable_trapped_nonwrapping_and_preview_stops_on_navigation_and_cancel`; `phase7_sample_workflows::asset_projection_keeps_active_and_requested_distinct_and_names_every_lifecycle` |
| Navigate a folder | `phase7_sample_workflows::browser_focus_is_stable_trapped_nonwrapping_and_preview_stops_on_navigation_and_cancel`; `filesystem_sample_catalog::listing_is_stable_folder_first_and_ids_are_library_relative` |
| Catalog refresh preserves focus | `phase7_sample_workflows::catalog_refresh_preserves_stable_row_focus_and_reprojects_typed_metadata` |
| Hold after preview is ready | `phase7_sample_workflows::preview_emits_no_audio_before_activation_and_never_changes_the_assigned_asset`; `audio_renderer::audition_commands_mix_only_into_the_correlated_origin_track_and_publish_playhead` |
| Release before preview is ready | `phase7_sample_workflows::release_before_preview_activation_suppresses_both_start_and_stop_commands` |
| Focus changes during preview | `phase7_sample_workflows::active_preview_stops_exactly_once_on_navigation_assignment_and_browser_cancel` |
| Edit a landmark | `phase7_sample_workflows::detail_numeric_and_choice_edits_use_the_reducer_and_return_to_exact_rows`; `sample_capability::configs_accept_same_kind_relative_asset_and_validate_playback_contract` |
| Asset is unavailable on restore | `saved_session::restore_prepares_before_commit_and_keeps_unavailable_invalid_and_over_budget_typed` |

## Live detail and assets demo

| Spec scenario | Production-path proof |
| --- | --- |
| Invoke the Phase 7 target | `crest_synth::tests::phase7_target_alias_negative_and_production_sample_fixtures_are_exact`; physical completion remains task 13.9 |
| Invoke the cumulative alias | `crest_synth::tests::phase7_target_alias_negative_and_production_sample_fixtures_are_exact` |
| Complete a detail and modal journey | `phase7_live_scene::phase7_live_scene_is_cumulative_semantic_bounded_and_has_a_falsifying_negative`; `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Preview without assignment | `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Recover from an asset error | `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Commit the valid asset | `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report` |
| Isolate the Sample target | `sample_preparer::normalized_range_release_and_patch_identity_are_enforced`; `audio_renderer::audition_commands_mix_only_into_the_correlated_origin_track_and_publish_playhead`; the cumulative headless report requires the same target Patch/track correlation |
| Route the committed Sample | `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report`; `live_mixer_routing_measurement` predicates retained by `LiveDemoReport::complete` |
| A required action is defeated | `phase7_live_scene::phase7_controlled_negative_reaches_teardown_but_fails_named_preview_predicates`; `crest_synth::tests::phase7_target_alias_negative_and_production_sample_fixtures_are_exact` |
| Complete teardown | `phase7_live_scene::phase7_headless_scene_correlates_worker_renderer_projection_and_lossless_report`; `phase7_callback_safety::sample_dispatch_render_audition_swap_and_retirement_are_callback_allocation_and_destruction_free` |

The deterministic target for this index is `cargo test --all-targets`; the
physical-device/window rows are intentionally not upgraded from incomplete
until `make demo-live-detail-and-assets` and its retained alias pass on the
production host.
