// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Graph data structure to store the results of points-to analysis.

use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::{Body, Place, ProjectionElem},
    ty::List,
};
use rustc_mir_dataflow::{fmt::DebugWithContext, JoinSemiLattice};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A node in the points-to graph, which could be a place on the stack or a heap allocation.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PlaceOrAlloc<'tcx> {
    Alloc(usize),
    Place(Place<'tcx>),
}

/// A node tagged with a DefId, to differentiate between places across different functions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct GlobalPlaceOrAlloc<'tcx> {
    def_id: DefId,
    place_or_alloc: PlaceOrAlloc<'tcx>,
}

impl<'tcx> GlobalPlaceOrAlloc<'tcx> {
    /// Check if the node has a given DefId.
    pub fn has_def_id(&self, def_id: DefId) -> bool {
        self.def_id == def_id
    }

    /// Remove DefId from the node.
    pub fn without_def_id(&self) -> PlaceOrAlloc<'tcx> {
        self.place_or_alloc
    }
}

impl<'tcx> From<Place<'tcx>> for PlaceOrAlloc<'tcx> {
    fn from(value: Place<'tcx>) -> Self {
        PlaceOrAlloc::Place(value)
    }
}

impl<'tcx> PlaceOrAlloc<'tcx> {
    /// Generate a new alloc with increasing allocation id.
    pub fn new_alloc() -> Self {
        static NEXT_ALLOC_ID: AtomicUsize = AtomicUsize::new(0);
        PlaceOrAlloc::Alloc(NEXT_ALLOC_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Tag the node with a DefId.
    pub fn with_def_id(&self, def_id: DefId) -> GlobalPlaceOrAlloc<'tcx> {
        GlobalPlaceOrAlloc { def_id, place_or_alloc: *self }
    }
}

/// Graph data structure that stores the current results of the point-to analysis. The graph is
/// directed, so having an edge between two places means that one is pointing to the other. For
/// example, `a = &b` would translate to `a --> b` and `a = b` to `a --> {all pointees of b}`.
///
/// Note that the aliasing is stored between places with no projections, which is sound but can be
/// imprecise. I.e., if two places have an edge in the graph, could mean that some scalar sub-places
/// (e.g. _1.0) of the places alias, too, but not the deref ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointsToGraph<'tcx> {
    /// A hash map of node --> {nodes} edges.
    edges: HashMap<GlobalPlaceOrAlloc<'tcx>, HashSet<GlobalPlaceOrAlloc<'tcx>>>,
}

impl<'tcx> PointsToGraph<'tcx> {
    /// Create a new graph, adding all existing places without projections from a body.
    pub fn new(body: &Body, def_id: DefId) -> Self {
        let places = (0..body.local_decls.len()).map(|local| {
            let place: PlaceOrAlloc =
                Place { local: local.into(), projection: List::empty() }.into();
            (place.with_def_id(def_id), HashSet::new())
        });
        Self { edges: HashMap::from_iter(places) }
    }

    /// Collect all nodes which have incoming edges from `nodes`.
    pub fn follow(
        &self,
        nodes: &HashSet<GlobalPlaceOrAlloc<'tcx>>,
    ) -> HashSet<GlobalPlaceOrAlloc<'tcx>> {
        nodes.iter().flat_map(|node| self.edges.get(node).cloned().unwrap_or_default()).collect()
    }

    /// For each node in `from`, add an edge to each node in `to`.
    pub fn extend(
        &mut self,
        from: &HashSet<GlobalPlaceOrAlloc<'tcx>>,
        to: &HashSet<GlobalPlaceOrAlloc<'tcx>>,
    ) {
        for node in from.iter() {
            let node_pointees = self.edges.entry(*node).or_default();
            node_pointees.extend(to.iter());
        }
    }

    /// Collect all scalar places to which a given place can alias. This is needed to resolve all
    /// deref-like projections.
    pub fn follow_from_place(
        &self,
        place: Place<'tcx>,
        current_def_id: DefId,
    ) -> HashSet<GlobalPlaceOrAlloc<'tcx>> {
        let place_or_alloc: PlaceOrAlloc =
            Place { local: place.local, projection: List::empty() }.into();
        let mut node_set = HashSet::from([place_or_alloc.with_def_id(current_def_id)]);
        for projection in place.projection {
            match projection {
                ProjectionElem::Deref => {
                    node_set = self.follow(&node_set);
                }
                ProjectionElem::Field(..)
                | ProjectionElem::Index(..)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. }
                | ProjectionElem::Downcast(..)
                | ProjectionElem::OpaqueCast(..)
                | ProjectionElem::Subtype(..) => {
                    /* There operations are no-ops w.r.t aliasing since we are tracking it on per-object basis. */
                }
            }
        }
        node_set
    }

    /// Dump the graph into a file using the graphviz format for later visualization.
    pub fn dump(&self, file_path: &str) {
        let nodes: Vec<String> = self
            .edges
            .keys()
            .map(|from| format!("\t\"{:?}:{:?}\"", from.def_id, from.place_or_alloc))
            .collect();
        let nodes_str = nodes.join("\n");
        let edges: Vec<String> = self
            .edges
            .iter()
            .flat_map(|(from, to)| {
                let from = format!("\"{:?}:{:?}\"", from.def_id, from.place_or_alloc);
                to.iter().map(move |to| {
                    let to = format!("\"{:?}:{:?}\"", to.def_id, to.place_or_alloc);
                    format!("\t{} -> {}", from.clone(), to)
                })
            })
            .collect();
        let edges_str = edges.join("\n");
        std::fs::write(file_path, format!("digraph {{\n{}\n{}\n}}", nodes_str, edges_str)).unwrap();
    }

    /// Find a transitive closure of the graph starting from a given place.
    pub fn transitive_closure(
        &self,
        target: &GlobalPlaceOrAlloc<'tcx>,
    ) -> HashSet<GlobalPlaceOrAlloc<'tcx>> {
        let mut result = HashSet::new();
        let mut queue = VecDeque::from([*target]);
        while !queue.is_empty() {
            let next_target = queue.pop_front().unwrap();
            if !result.contains(&next_target) {
                let outgoing_edges = self.edges.get(&next_target).unwrap();
                queue.extend(outgoing_edges.iter());
                result.insert(next_target);
            }
        }
        result
    }

    /// Retrieve all places to which a given place is pointing to.
    pub fn pointees_of(
        &self,
        target: &GlobalPlaceOrAlloc<'tcx>,
    ) -> HashSet<GlobalPlaceOrAlloc<'tcx>> {
        self.edges
            .get(&target)
            .expect(format!("unable to retrieve {:?} from points-to graph", target).as_str())
            .clone()
    }
}

/// Since we are performing the analysis using a dataflow, we need to implement a proper monotonous
/// join operation. In our case, this is a simple union of two graphs. This "lattice" is finite,
/// because in the worst case all places will alias to all places, in which case the join will be a
/// no-op.
impl<'tcx> JoinSemiLattice for PointsToGraph<'tcx> {
    fn join(&mut self, other: &Self) -> bool {
        let mut updated = false;
        // Check every node in the other graph.
        for (from, to) in other.edges.iter() {
            // If node already exists in the original graph.
            if self.edges.contains_key(from) {
                // Check if there are any edges that are in the other graph but not in the original
                // graph.
                if to.difference(self.edges.get(from).unwrap()).count() != 0 {
                    updated = true;
                }
                // Add all edges to the original graph.
                self.edges.get_mut(from).unwrap().extend(to.iter());
            } else {
                // If node does not exist, add the node and all edges from it.
                self.edges.insert(*from, to.clone());
                updated = true;
            }
        }
        updated
    }
}

/// This is a requirement for the fixpoint solver, and there is no derive macro for this, so
/// implement it manually.
impl<'tcx, C> DebugWithContext<C> for PointsToGraph<'tcx> {}
