mod support;

use crest_synth::control::app_state::EventRejection;
use crest_synth::control::event_record::{EventOutcome, EventSource};
use crest_synth::testing::demo_scene_report::DemoCoverageGroup;
use serde_json::Value;
use std::collections::BTreeSet;

const COVERAGE_GROUPS: [DemoCoverageGroup; 8] = [
    DemoCoverageGroup::Events,
    DemoCoverageGroup::Directions,
    DemoCoverageGroup::MidiKinds,
    DemoCoverageGroup::EditableParameters,
    DemoCoverageGroup::SerializedProperties,
    DemoCoverageGroup::Rejections,
    DemoCoverageGroup::Projections,
    DemoCoverageGroup::AudioEffects,
];

fn stable_control_surface(tree: &Value) -> Vec<Value> {
    ["/patches", "/global", "/selection"]
        .into_iter()
        .map(|pointer| {
            tree.pointer(pointer)
                .expect("required StateTree branch exists")
                .clone()
        })
        .collect()
}

fn source_name(source: EventSource) -> &'static str {
    match source {
        EventSource::Startup => "startup",
        EventSource::Keyboard => "keyboard",
        EventSource::AutomaticMidi => "automatic-midi",
        EventSource::DemoScene => "demo-scene",
        EventSource::System => "system",
    }
}

#[test]
fn exhaustive_scene_proves_exact_coverage_boundaries_and_restoration() {
    let first = support::run_demo();
    let second = support::run_demo();
    let report = &first.report;

    assert!(
        report.is_complete(),
        "coverage report contains a gap: {:?}",
        report.coverage()
    );
    assert_eq!(report.coverage().missing_count(), 0);
    assert_eq!(report.coverage().unexpected_count(), 0);
    assert_eq!(
        report.coverage().expected_count(),
        report.coverage().exercised_count()
    );
    assert_eq!(report.event_log().dropped_records(), 0);
    assert_eq!(
        report.event_log().total_observed(),
        report.event_log().records().len() as u64
    );

    for group in COVERAGE_GROUPS {
        let coverage = report.coverage().group(group);
        assert_eq!(coverage.expected(), coverage.exercised(), "{group:?}");
        assert!(coverage.missing().is_empty(), "{group:?}");
        assert!(coverage.unexpected().is_empty(), "{group:?}");
    }

    let expected = first
        .expected_coverage
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed = report
        .event_log()
        .coverage()
        .exercised()
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(expected, observed);
    assert_eq!(
        report.event_log().coverage().expected(),
        report.event_log().coverage().exercised()
    );

    for parameter in [
        "gainDb",
        "pan",
        "reverbSend",
        "delaySend",
        "masterGainDb",
        "reverbRoomSize",
        "reverbDamping",
        "reverbReturn",
        "delayMilliseconds",
        "delayFeedback",
        "delayReturn",
    ] {
        assert!(
            report
                .coverage()
                .group(DemoCoverageGroup::EditableParameters)
                .exercised()
                .iter()
                .any(|identifier| identifier.ends_with(parameter)),
            "missing exact parameter exercise for {parameter}"
        );
    }

    let boundary_rejections = report
        .event_log()
        .records()
        .iter()
        .filter(|record| {
            record.outcome() == EventOutcome::Rejected
                && record.rejection() == Some(EventRejection::ParameterAtBoundary)
        })
        .count();
    assert!(
        boundary_rejections >= 22,
        "each of eleven typed parameters needs lower and upper boundary evidence; observed {boundary_rejections}"
    );
    for (index, record) in report.event_log().records().iter().enumerate() {
        if record.rejection() == Some(EventRejection::ParameterAtBoundary) {
            assert!(
                report.event_log().records()[index + 1..]
                    .iter()
                    .any(|later| {
                        later.outcome() == EventOutcome::Accepted
                            && matches!(
                                later.input(),
                                crest_synth::control::event_record::EventInput::Adjust { .. }
                            )
                    }),
                "a boundary rejection must not terminate later valid edits"
            );
        }
    }

    let sources = report
        .event_log()
        .records()
        .iter()
        .map(|record| source_name(record.source()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        sources,
        BTreeSet::from([
            "startup",
            "keyboard",
            "automatic-midi",
            "demo-scene",
            "system",
        ])
    );

    let final_tree: Value = serde_json::from_str(report.final_state_tree().json())
        .expect("final StateTree is valid JSON");
    assert_eq!(
        stable_control_surface(&first.baseline),
        stable_control_surface(&final_tree),
        "every reversible value and selection must return to baseline"
    );

    assert!(report
        .checkpoints()
        .iter()
        .all(|checkpoint| checkpoint.audio_measurement().is_finite()));
    assert_eq!(first.expected_coverage, second.expected_coverage);
    assert_eq!(first.baseline, second.baseline);
    assert_eq!(
        report.to_json().expect("first report serializes"),
        second.report.to_json().expect("second report serializes"),
        "fresh identical services must produce byte-identical reports"
    );

    println!("CREST_ACCEPTANCE exhaustive_demo_scene passed");
}
