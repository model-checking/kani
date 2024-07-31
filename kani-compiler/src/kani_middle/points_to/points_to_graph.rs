// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Graph data structure to store the results of points-to analysis.

use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::{Location, Place, ProjectionElem},
    ty::List,
};
use rustc_mir_dataflow::{fmt::DebugWithContext, JoinSemiLattice};
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the points-to graph, which could be a place on the stack or a heap allocation.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LocalMemLoc<'tcx> {
    /// Using a combination of DefId + Location implements allocation-site abstraction.
    Alloc(DefId, Location),
    Place(Place<'tcx>),
}

/// A node tagged with a DefId, to differentiate between places across different functions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GlobalMemLoc<'tcx> {
    Local(DefId, LocalMemLoc<'tcx>),
    Global(DefId),
}

impl<'tcx> GlobalMemLoc<'tcx> {
    /// Returns DefId of the memory location.
    pub fn def_id(&self) -> DefId {
        match self {
            GlobalMemLoc::Local(def_id, _) | GlobalMemLoc::Global(def_id) => *def_id,
        }
    }

    /// Returns LocalMemLoc of the memory location if available.
    pub fn maybe_local_mem_loc(&self) -> Option<LocalMemLoc<'tcx>> {
        match self {
            GlobalMemLoc::Local(_, mem_loc) => Some(*mem_loc),
            GlobalMemLoc::Global(_) => None,
        }
    }
}

impl<'tcx> From<Place<'tcx>> for LocalMemLoc<'tcx> {
    fn from(value: Place<'tcx>) -> Self {
        LocalMemLoc::Place(value)
    }
}

impl<'tcx> LocalMemLoc<'tcx> {
    /// Register a new heap allocation site.
    pub fn new_alloc(def_id: DefId, location: Location) -> Self {
        LocalMemLoc::Alloc(def_id, location)
    }

    /// Tag the node with a DefId.
    pub fn with_def_id(&self, def_id: DefId) -> GlobalMemLoc<'tcx> {
        GlobalMemLoc::Local(def_id, *self)
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
    edges: HashMap<GlobalMemLoc<'tcx>, HashSet<GlobalMemLoc<'tcx>>>,
}

impl<'tcx> PointsToGraph<'tcx> {
    pub fn empty() -> Self {
        Self { edges: HashMap::new() }
    }

    /// Collect all nodes which have incoming edges from `nodes`.
    pub fn follow(&self, nodes: &HashSet<GlobalMemLoc<'tcx>>) -> HashSet<GlobalMemLoc<'tcx>> {
        nodes.iter().flat_map(|node| self.edges.get(node).cloned().unwrap_or_default()).collect()
    }

    /// For each node in `from`, add an edge to each node in `to`.
    pub fn extend(&mut self, from: &HashSet<GlobalMemLoc<'tcx>>, to: &HashSet<GlobalMemLoc<'tcx>>) {
        for node in from.iter() {
            let node_pointees = self.edges.entry(*node).or_default();
            node_pointees.extend(to.iter());
        }
    }

    /// Collect all scalar places to which a given place can alias. This is needed to resolve all
    /// dereference projections.
    pub fn follow_from_place(
        &self,
        place: Place<'tcx>,
        current_def_id: DefId,
    ) -> HashSet<GlobalMemLoc<'tcx>> {
        let place_or_alloc: LocalMemLoc =
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
        let mut nodes: Vec<String> =
            self.edges.keys().map(|from| format!("\t\"{:?}\"", from)).collect();
        nodes.sort();
        let nodes_str = nodes.join("\n");

        let mut edges: Vec<String> = self
            .edges
            .iter()
            .flat_map(|(from, to)| {
                let from = format!("\"{:?}\"", from);
                to.iter().map(move |to| {
                    let to = format!("\"{:?}\"", to);
                    format!("\t{} -> {}", from.clone(), to)
                })
            })
            .collect();
        edges.sort();
        let edges_str = edges.join("\n");

        std::fs::write(file_path, format!("digraph {{\n{}\n{}\n}}", nodes_str, edges_str)).unwrap();
    }

    /// Find a transitive closure of the graph starting from a set of given locations; this also
    /// includes statics.
    pub fn transitive_closure(&self, targets: HashSet<GlobalMemLoc<'tcx>>) -> PointsToGraph<'tcx> {
        let mut result = PointsToGraph::empty();
        // Working queue.
        let mut queue = VecDeque::from_iter(targets);
        // Add all statics, as they can be accessed at any point.
        let statics = self.edges.keys().filter(|node| matches!(node, GlobalMemLoc::Global(_)));
        queue.extend(statics);
        // Add all entries.
        while !queue.is_empty() {
            let next_target = queue.pop_front().unwrap();
            result.edges.entry(next_target).or_insert_with(|| {
                let outgoing_edges =
                    self.edges.get(&next_target).cloned().unwrap_or(HashSet::new());
                queue.extend(outgoing_edges.iter());
                outgoing_edges.clone()
            });
        }
        result
    }

    /// Retrieve all places to which a given place is pointing to.
    pub fn pointees_of(&self, target: &GlobalMemLoc<'tcx>) -> HashSet<GlobalMemLoc<'tcx>> {
        self.edges.get(&target).unwrap_or(&HashSet::new()).clone()
    }

    // Merge the other graph into self, consuming it.
    pub fn consume(&mut self, other: PointsToGraph<'tcx>) {
        for (from, to) in other.edges {
            let existing_to = self.edges.entry(from).or_default();
            existing_to.extend(to);
        }
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
                let difference: HashSet<_> =
                    to.difference(self.edges.get(from).unwrap()).cloned().collect();
                if !difference.is_empty() {
                    updated = true;
                    // Add all edges to the original graph.
                    self.edges.get_mut(from).unwrap().extend(difference);
                }
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
