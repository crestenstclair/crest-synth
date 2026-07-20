mod support;

use crest_synth::testing::{
    BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation,
    DemoCoverageGroup,
};
use std::collections::BTreeSet;

#[test]
fn typed_descriptors_and_discovered_serialized_leaves_are_bidirectionally_exact() {
    let run = support::run_demo();
    let serialized = run
        .report
        .coverage()
        .group(DemoCoverageGroup::SerializedProperties);
    let projections = run.report.coverage().group(DemoCoverageGroup::Projections);
    let expected = serialized
        .expected()
        .iter()
        .chain(projections.expected())
        .cloned()
        .collect::<BTreeSet<_>>();
    let exercised = serialized
        .exercised()
        .iter()
        .chain(projections.exercised())
        .cloned()
        .collect::<BTreeSet<_>>();

    assert!(!expected.is_empty());
    assert_eq!(expected, exercised);
    assert!(serialized.missing().is_empty());
    assert!(serialized.unexpected().is_empty());
    assert!(projections.missing().is_empty());
    assert!(projections.unexpected().is_empty());
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.stateTree.")));
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.eventRecord.")));
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.eventLog.")));
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.textProjection.")));

    let harness = BehavioralMutationHarness::new();
    let healthy = harness.run(BehavioralMutationCase::OmittedStateTreeLeaf, false);
    let mutant = harness.run(BehavioralMutationCase::OmittedStateTreeLeaf, true);
    let (
        BehavioralMutationObservation::OmittedStateTreeLeaf(healthy),
        BehavioralMutationObservation::OmittedStateTreeLeaf(mutant),
    ) = (healthy.into_observation(), mutant.into_observation())
    else {
        panic!("the omitted-leaf case must retain its typed observation schema");
    };

    assert!(healthy.schema_surface_equal);
    assert!(healthy.required_leaf_count > 0);
    assert_eq!(healthy.missing_leaf_count, 0);
    assert_eq!(healthy.unexpected_leaf_count, 0);
    assert!(!mutant.schema_surface_equal);
    assert_eq!(mutant.required_leaf_count, healthy.required_leaf_count);
    assert_eq!(mutant.missing_leaf_count, 1);
    assert_eq!(mutant.unexpected_leaf_count, 0);

    println!("CREST_ACCEPTANCE schema_surface passed");
}
