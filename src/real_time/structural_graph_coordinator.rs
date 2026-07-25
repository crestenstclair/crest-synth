use crate::real_time::graph_handoff_status::GraphHandoffStatus;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::prepared_graph::{PreparedGraph, PreparedGraphLayout};
use crate::real_time::structural_graph_boundary::ControlStructuralGraphBoundary;
use core::fmt;

/// Control/worker-side one-in-flight structural publication coordinator.
pub struct StructuralGraphCoordinator<Boundary> {
    boundary: Boundary,
    required_layout: PreparedGraphLayout,
    in_flight: Option<InFlightGraph>,
}

impl<Boundary> StructuralGraphCoordinator<Boundary>
where
    Boundary: ControlStructuralGraphBoundary,
{
    pub fn new(boundary: Boundary, active_graph: &PreparedGraph) -> Self {
        Self {
            boundary,
            required_layout: active_graph.layout(),
            in_flight: None,
        }
    }

    /// Publishes one replacement only when no prior graph awaits activation and
    /// off-callback retirement acknowledgement.
    pub fn publish(&mut self, graph: PreparedGraph) -> Result<(), GraphPublicationError> {
        self.poll();
        if self.in_flight.is_some() {
            return Err(GraphPublicationError::new(
                GraphPublicationFailure::ReplacementInFlight,
                graph,
            ));
        }

        let status = self.boundary.read_status_on_control();
        let Some(previous_revision) = status.active_revision() else {
            return Err(GraphPublicationError::new(
                GraphPublicationFailure::NoActiveGraph,
                graph,
            ));
        };
        if graph.revision() <= previous_revision {
            return Err(GraphPublicationError::new(
                GraphPublicationFailure::NonMonotonicRevision,
                graph,
            ));
        }
        if graph.layout() != self.required_layout {
            return Err(GraphPublicationError::new(
                GraphPublicationFailure::IncompatibleLayout,
                graph,
            ));
        }
        let revision = graph.revision();
        if let Err(full) = self.boundary.publish_prepared_on_control(graph) {
            return Err(GraphPublicationError::new(
                GraphPublicationFailure::BoundaryFull,
                full.into_graph(),
            ));
        }
        self.in_flight = Some(InFlightGraph {
            revision,
            previous_revision,
            active_acknowledged: false,
            retired_acknowledged: false,
            retired_collected: false,
        });
        Ok(())
    }

    /// Drains every ready returned graph on control ownership and advances the
    /// one-in-flight acknowledgement state.
    pub fn poll(&mut self) -> CoordinatorProgress {
        let mut collected_count = 0_u64;
        while let Some(revision) = self.boundary.collect_retired_on_control() {
            collected_count = collected_count.saturating_add(1);
            if let Some(in_flight) = &mut self.in_flight {
                if revision == in_flight.previous_revision {
                    in_flight.retired_collected = true;
                }
            }
        }

        let status = self.boundary.read_status_on_control();
        let mut completed_revision = None;
        if let Some(in_flight) = &mut self.in_flight {
            if status.active_revision() == Some(in_flight.revision) {
                in_flight.active_acknowledged = true;
            }
            if status.retired_revision() == Some(in_flight.previous_revision) {
                in_flight.retired_acknowledged = true;
            }
            if in_flight.active_acknowledged
                && in_flight.retired_acknowledged
                && in_flight.retired_collected
            {
                completed_revision = Some(in_flight.revision);
            }
        }
        if completed_revision.is_some() {
            self.in_flight = None;
        }

        CoordinatorProgress {
            status,
            collected_count,
            completed_revision,
        }
    }

    pub fn in_flight_revision(&self) -> Option<GraphRevision> {
        self.in_flight.map(|in_flight| in_flight.revision)
    }

    pub fn status(&self) -> GraphHandoffStatus {
        self.boundary.read_status_on_control()
    }

    pub fn into_boundary(self) -> Boundary {
        self.boundary
    }
}

#[derive(Clone, Copy, Debug)]
struct InFlightGraph {
    revision: GraphRevision,
    previous_revision: GraphRevision,
    active_acknowledged: bool,
    retired_acknowledged: bool,
    retired_collected: bool,
}

/// Fixed-size result of one coordinator poll.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CoordinatorProgress {
    status: GraphHandoffStatus,
    collected_count: u64,
    completed_revision: Option<GraphRevision>,
}

impl CoordinatorProgress {
    pub const fn status(self) -> GraphHandoffStatus {
        self.status
    }

    pub const fn collected_count(self) -> u64 {
        self.collected_count
    }

    pub const fn completed_revision(self) -> Option<GraphRevision> {
        self.completed_revision
    }
}

/// Why a coordinator returned ownership of an unpublished graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphPublicationFailure {
    ReplacementInFlight,
    NoActiveGraph,
    NonMonotonicRevision,
    IncompatibleLayout,
    BoundaryFull,
}

/// Publication failure that preserves the complete candidate graph.
#[must_use = "the complete unpublished graph remains owned by control"]
#[derive(Debug)]
pub struct GraphPublicationError {
    reason: GraphPublicationFailure,
    graph: PreparedGraph,
}

impl GraphPublicationError {
    const fn new(reason: GraphPublicationFailure, graph: PreparedGraph) -> Self {
        Self { reason, graph }
    }

    pub const fn reason(&self) -> GraphPublicationFailure {
        self.reason
    }

    pub const fn graph(&self) -> &PreparedGraph {
        &self.graph
    }

    pub fn into_graph(self) -> PreparedGraph {
        self.graph
    }
}

impl fmt::Display for GraphPublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.reason {
            GraphPublicationFailure::ReplacementInFlight => {
                "a structural graph replacement is already in flight"
            }
            GraphPublicationFailure::NoActiveGraph => {
                "structural publication has no acknowledged active graph"
            }
            GraphPublicationFailure::NonMonotonicRevision => {
                "structural graph revision must advance monotonically"
            }
            GraphPublicationFailure::IncompatibleLayout => {
                "replacement graph layout or callback capacities are incompatible"
            }
            GraphPublicationFailure::BoundaryFull => {
                "the structural graph publication queue is full"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GraphPublicationError {}

#[cfg(test)]
mod tests {
    use super::{GraphPublicationFailure, StructuralGraphCoordinator};
    use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::graph_handoff_status::GraphHandoffStatus;
    use crate::real_time::graph_revision::GraphRevision;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::real_time::prepared_graph::PreparedGraph;
    use crate::real_time::prepared_graph_builder::PreparedGraphBuilder;
    use crate::real_time::structural_graph_boundary::{
        ControlStructuralGraphBoundary, StructuralBoundaryFull,
    };
    use crate::synth::instrument_preparer::InstrumentPreparer;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    struct TestControlBoundary {
        shared: Arc<Mutex<TestBoundaryState>>,
    }

    struct TestBoundaryState {
        prepared: Option<PreparedGraph>,
        retired: VecDeque<PreparedGraph>,
        status: GraphHandoffStatus,
    }

    impl ControlStructuralGraphBoundary for TestControlBoundary {
        fn publish_prepared_on_control(
            &mut self,
            graph: PreparedGraph,
        ) -> Result<(), StructuralBoundaryFull> {
            let mut shared = self.shared.lock().unwrap();
            if shared.prepared.is_some() {
                Err(StructuralBoundaryFull::new(graph))
            } else {
                shared.prepared = Some(graph);
                Ok(())
            }
        }

        fn collect_retired_on_control(&mut self) -> Option<GraphRevision> {
            let graph = self.shared.lock().unwrap().retired.pop_front()?;
            let revision = graph.revision();
            drop(graph);
            Some(revision)
        }

        fn read_status_on_control(&self) -> GraphHandoffStatus {
            self.shared.lock().unwrap().status
        }
    }

    fn globals() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap()
    }

    fn graph(value: u64) -> PreparedGraph {
        graph_with_frames(value, 1)
    }

    fn graph_with_frames(value: u64, max_frames: usize) -> PreparedGraph {
        let provider = HiDefSoundFontCapability::new().unwrap();
        let registry = provider.registry().unwrap();
        let preparers: Vec<Box<dyn InstrumentPreparer>> = Vec::new();
        let revision = GraphRevision::new(value).unwrap();
        let parameters = ParameterSnapshot::for_graph(value, revision, globals(), &[]).unwrap();
        PreparedGraphBuilder::new(&registry, &preparers)
            .build(revision, &[], parameters, 16_000.0, max_frames)
            .unwrap()
    }

    fn coordinator(
        active_revision: u64,
    ) -> (
        StructuralGraphCoordinator<TestControlBoundary>,
        Arc<Mutex<TestBoundaryState>>,
    ) {
        let shared = Arc::new(Mutex::new(TestBoundaryState {
            prepared: None,
            retired: VecDeque::new(),
            status: GraphHandoffStatus::with_active(GraphRevision::new(active_revision).unwrap()),
        }));
        let active_graph = graph(active_revision);
        let coordinator = StructuralGraphCoordinator::new(
            TestControlBoundary {
                shared: Arc::clone(&shared),
            },
            &active_graph,
        );
        (coordinator, shared)
    }

    #[test]
    fn one_in_flight_graph_requires_activation_retirement_and_collection() {
        let (mut coordinator, shared) = coordinator(1);
        coordinator.publish(graph(2)).unwrap();
        assert_eq!(
            coordinator.in_flight_revision(),
            Some(GraphRevision::new(2).unwrap())
        );

        let rejected = coordinator.publish(graph(3)).unwrap_err();
        assert_eq!(
            rejected.reason(),
            GraphPublicationFailure::ReplacementInFlight
        );
        assert_eq!(rejected.graph().revision(), GraphRevision::new(3).unwrap());
        drop(rejected.into_graph());

        let replacement = shared.lock().unwrap().prepared.take().unwrap();
        assert_eq!(replacement.revision(), GraphRevision::new(2).unwrap());
        shared.lock().unwrap().status =
            GraphHandoffStatus::with_active(GraphRevision::new(2).unwrap());
        let activation_only = coordinator.poll();
        assert_eq!(activation_only.completed_revision(), None);
        assert_eq!(
            coordinator.in_flight_revision(),
            Some(replacement.revision())
        );

        let mut completed = GraphHandoffStatus::with_active(replacement.revision());
        completed.record_retired(GraphRevision::new(1).unwrap());
        let mut shared_state = shared.lock().unwrap();
        shared_state.status = completed;
        shared_state.retired.push_back(graph(1));
        drop(shared_state);

        let progress = coordinator.poll();
        assert_eq!(progress.collected_count(), 1);
        assert_eq!(progress.completed_revision(), Some(replacement.revision()));
        assert_eq!(coordinator.in_flight_revision(), None);

        drop(replacement);
        coordinator.publish(graph(3)).unwrap();
        assert_eq!(
            coordinator.in_flight_revision(),
            Some(GraphRevision::new(3).unwrap())
        );
    }

    #[test]
    fn publication_preserves_graph_on_full_missing_or_nonmonotonic_state() {
        let (mut coordinator, shared) = coordinator(1);
        shared.lock().unwrap().prepared = Some(graph(2));
        let full = coordinator.publish(graph(3)).unwrap_err();
        assert_eq!(full.reason(), GraphPublicationFailure::BoundaryFull);
        assert_eq!(full.graph().revision(), GraphRevision::new(3).unwrap());
        drop(full.into_graph());
        shared.lock().unwrap().prepared.take();

        let old = coordinator.publish(graph(1)).unwrap_err();
        assert_eq!(old.reason(), GraphPublicationFailure::NonMonotonicRevision);
        assert_eq!(old.graph().revision(), GraphRevision::new(1).unwrap());
        drop(old.into_graph());

        shared.lock().unwrap().status = GraphHandoffStatus::default();
        let missing = coordinator.publish(graph(4)).unwrap_err();
        assert_eq!(missing.reason(), GraphPublicationFailure::NoActiveGraph);
        assert_eq!(missing.graph().revision(), GraphRevision::new(4).unwrap());
    }

    #[test]
    fn publication_rejects_a_graph_with_different_callback_capacities() {
        let (mut coordinator, shared) = coordinator(1);
        let incompatible = coordinator.publish(graph_with_frames(2, 2)).unwrap_err();

        assert_eq!(
            incompatible.reason(),
            GraphPublicationFailure::IncompatibleLayout
        );
        assert_eq!(incompatible.graph().max_frames(), 2);
        assert!(shared.lock().unwrap().prepared.is_none());
    }
}
