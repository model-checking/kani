// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Graph data structure to store the results of points-to analysis.

use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::{Location, Place, ProjectionElem},
    ty::{Instance, List, TyCtxt},
};
use rustc_mir_dataflow::{JoinSemiLattice, fmt::DebugWithContext};
use rustc_smir::rustc_internal;
use stable_mir::mir::{
    Place as StablePlace,
    mono::{Instance as StableInstance, StaticDef},
};
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the points-to graph, which could be a place on the stack, a heap allocation, or a static.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MemLoc<'tcx> {
    /// Notice that the type of `Place` here is not restricted to references or pointers. For
    /// example, we propagate aliasing information for values derived from casting a pointer to a
    /// usize in order to ensure soundness, as it could later be casted back to a pointer.
    Stack(Instance<'tcx>, Place<'tcx>),
    /// Using a combination of the instance of the function where the allocation took place and the
    /// location of the allocation inside this function implements allocation-site abstraction.
    Heap(Instance<'tcx>, Location),
    Static(DefId),
}

impl<'tcx> MemLoc<'tcx> {
    /// Create a memory location representing a new heap allocation site.
    pub fn new_heap_allocation(instance: Instance<'tcx>, location: Location) -> Self {
        MemLoc::Heap(instance, location)
    }

    /// Create a memory location representing a new stack allocation.
    pub fn new_stack_allocation(instance: Instance<'tcx>, place: Place<'tcx>) -> Self {
        MemLoc::Stack(instance, place)
    }

    /// Create a memory location representing a new static allocation.
    pub fn new_static_allocation(static_def: DefId) -> Self {
        MemLoc::Static(static_def)
    }

    /// Create a memory location representing a new stack allocation from StableMIR values.
    pub fn from_stable_stack_allocation(
        instance: StableInstance,
        place: StablePlace,
        tcx: TyCtxt<'tcx>,
    ) -> Self {
        let internal_instance = rustc_internal::internal(tcx, instance);
        let internal_place = rustc_internal::internal(tcx, place);
        Self::new_stack_allocation(internal_instance, internal_place)
    }

    /// Create a memory location representing a new static allocation from StableMIR values.
    pub fn from_stable_static_allocation(static_def: StaticDef, tcx: TyCtxt<'tcx>) -> Self {
        let static_def_id = rustc_internal::internal(tcx, static_def);
        Self::new_static_allocation(static_def_id)
    }
}

/// Data structure to keep track of both successors and ancestors of the node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NodeData<'tcx> {
    successors: HashSet<MemLoc<'tcx>>,
    ancestors: HashSet<MemLoc<'tcx>>,
}

impl NodeData<'_> {
    /// Merge two instances of NodeData together, return true if the original one was updated and
    /// false otherwise.
    fn merge(&mut self, other: Self) -> bool {
        let successors_before = self.successors.len();
        let ancestors_before = self.ancestors.len();
        self.successors.extend(other.successors);
        self.ancestors.extend(other.ancestors);
        let successors_after = self.successors.len();
        let ancestors_after = self.ancestors.len();
        successors_before != successors_after || ancestors_before != ancestors_after
    }
}

/// Graph data structure that stores the current results of the point-to analysis. The graph is
/// directed, so having an edge between two places means that one is pointing to the other.
///
/// For example:
/// - `a = &b` would translate to `a --> b`
/// - `a = b` would translate to `a --> {all pointees of b}` (if `a` and `b` are pointers /
///   references)
///
/// Note that the aliasing is not field-sensitive, since the nodes in the graph are places with no
/// projections, which is sound but can be imprecise.
///
/// For example:
/// ```
/// let ref_pair = (&a, &b); // Will add `ref_pair --> (a | b)` edges into the graph.
/// let first = ref_pair.0; // Will add `first -> (a | b)`, which is an overapproximation.
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointsToGraph<'tcx> {
    /// A hash map of node --> {nodes} edges.
    nodes: HashMap<MemLoc<'tcx>, NodeData<'tcx>>,
}

impl<'tcx> PointsToGraph<'tcx> {
    pub fn empty() -> Self {
        Self { nodes: HashMap::new() }
    }

    /// Collect all nodes which have incoming edges from `nodes`.
    pub fn successors(&self, nodes: &HashSet<MemLoc<'tcx>>) -> HashSet<MemLoc<'tcx>> {
        nodes
            .iter()
            .flat_map(|node| self.nodes.get(node).cloned().unwrap_or_default().successors)
            .collect()
    }

    /// Collect all nodes which have outgoing edges to `nodes`.
    pub fn ancestors(&self, nodes: &HashSet<MemLoc<'tcx>>) -> HashSet<MemLoc<'tcx>> {
        nodes
            .iter()
            .flat_map(|node| self.nodes.get(node).cloned().unwrap_or_default().ancestors)
            .collect()
    }

    /// For each node in `from`, add an edge to each node in `to` (and the reverse for ancestors).
    pub fn extend(&mut self, from: &HashSet<MemLoc<'tcx>>, to: &HashSet<MemLoc<'tcx>>) {
        for node in from.iter() {
            let node_pointees = self.nodes.entry(*node).or_default();
            node_pointees.successors.extend(to.iter());
        }
        for node in to.iter() {
            let node_pointees = self.nodes.entry(*node).or_default();
            node_pointees.ancestors.extend(from.iter());
        }
    }

    /// Collect all places to which a given place can alias.
    ///
    /// We automatically resolve dereference projections here (by finding successors for each
    /// dereference projection we encounter), which is valid as long as we do it for every place we
    /// add to the graph.
    pub fn resolve_place(
        &self,
        place: Place<'tcx>,
        instance: Instance<'tcx>,
    ) -> HashSet<MemLoc<'tcx>> {
        let place_without_projections = Place { local: place.local, projection: List::empty() };
        let mut node_set =
            HashSet::from([MemLoc::new_stack_allocation(instance, place_without_projections)]);
        for projection in place.projection {
            match projection {
                ProjectionElem::Deref => {
                    node_set = self.successors(&node_set);
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

    /// Stable interface for `resolve_place`.
    pub fn resolve_place_stable(
        &self,
        place: StablePlace,
        instance: StableInstance,
        tcx: TyCtxt<'tcx>,
    ) -> HashSet<MemLoc<'tcx>> {
        let internal_place = rustc_internal::internal(tcx, place);
        let internal_instance = rustc_internal::internal(tcx, instance);
        self.resolve_place(internal_place, internal_instance)
    }

    /// Dump the graph into a file using the graphviz format for later visualization.
    pub fn dump(&self, file_path: &str) {
        let mut nodes: Vec<String> =
            self.nodes.keys().map(|from| format!("\t\"{:?}\"", from)).collect();
        nodes.sort();
        let nodes_str = nodes.join("\n");

        let mut edges: Vec<String> = self
            .nodes
            .iter()
            .flat_map(|(from, to)| {
                let from = format!("\"{:?}\"", from);
                to.successors.iter().map(move |to| {
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
    pub fn transitive_closure(&self, targets: HashSet<MemLoc<'tcx>>) -> PointsToGraph<'tcx> {
        let mut result = PointsToGraph::empty();
        // Working queue.
        let mut queue = VecDeque::from_iter(targets);
        // Add all statics, as they can be accessed at any point.
        let statics = self.nodes.keys().filter(|node| matches!(node, MemLoc::Static(_)));
        queue.extend(statics);
        // Add all entries.
        while let Some(next_target) = queue.pop_front() {
            result.nodes.entry(next_target).or_insert_with(|| {
                let outgoing_edges = self.nodes.get(&next_target).cloned().unwrap_or_default();
                queue.extend(outgoing_edges.successors.iter());
                outgoing_edges.clone()
            });
        }
        result
    }
}

/// Since we are performing the analysis using a dataflow, we need to implement a proper monotonous
/// join operation. In our case, this is a simple union of two graphs. This "lattice" is finite,
/// because in the worst case all places will alias to all places, in which case the join will be a
/// no-op.
impl JoinSemiLattice for PointsToGraph<'_> {
    fn join(&mut self, other: &Self) -> bool {
        let mut updated = false;
        // Check every node in the other graph.
        for (node, data) in other.nodes.iter() {
            let existing_node = self.nodes.entry(*node).or_default();
            let changed = existing_node.merge(data.clone());
            updated |= changed;
        }
        updated
    }
}

/// This is a requirement for the fixpoint solver, and there is no derive macro for this, so
/// implement it manually.
impl<C> DebugWithContext<C> for PointsToGraph<'_> {}
