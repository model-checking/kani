//! This pass is only used for the UNIT TESTS and DEBUGGING NEEDS
//! around dependency graph construction. It serves two purposes; it
//! will dump graphs in graphviz form to disk, and it searches for
//! `#[rustc_if_this_changed]` and `#[rustc_then_this_would_need]`
//! annotations. These annotations can be used to test whether paths
//! exist in the graph. These checks run after codegen, so they view the
//! the final state of the dependency graph. Note that there are
//! similar assertions found in `persist::dirty_clean` which check the
//! **initial** state of the dependency graph, just after it has been
//! loaded from disk.
//!
//! In this code, we report errors on each `rustc_if_this_changed`
//! annotation. If a path exists in all cases, then we would report
//! "all path(s) exist". Otherwise, we report: "no path to `foo`" for
//! each case where no path exists. `ui` tests can then be
//! used to check when paths exist or do not.
//!
//! The full form of the `rustc_if_this_changed` annotation is
//! `#[rustc_if_this_changed("foo")]`, which will report a
//! source node of `foo(def_id)`. The `"foo"` is optional and
//! defaults to `"Hir"` if omitted.
//!
//! Example:
//!
//! ```
//! #[rustc_if_this_changed(Hir)]
//! fn foo() { }
//!
//! #[rustc_then_this_would_need(codegen)] //~ ERROR no path from `foo`
//! fn bar() { }
//!
//! #[rustc_then_this_would_need(codegen)] //~ ERROR OK
//! fn baz() { foo(); }
//! ```

use rustc_ast as ast;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::graph::implementation::{Direction, NodeIndex, INCOMING, OUTGOING};
use rustc_graphviz as dot;
use rustc_hir as hir;
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{self, NestedVisitorMap, Visitor};
use rustc_middle::dep_graph::{
    DepGraphQuery, DepKind, DepNode, DepNodeExt, DepNodeFilter, EdgeFilter,
};
use rustc_middle::hir::map::Map;
use rustc_middle::ty::TyCtxt;
use rustc_span::symbol::{sym, Symbol};
use rustc_span::Span;

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};

#[allow(missing_docs)]
pub fn assert_dep_graph(tcx: TyCtxt<'_>) {
    tcx.dep_graph.with_ignore(|| {
        if tcx.sess.opts.debugging_opts.dump_dep_graph {
            tcx.dep_graph.with_query(dump_graph);
        }

        if !tcx.sess.opts.debugging_opts.query_dep_graph {
            return;
        }

        // if the `rustc_attrs` feature is not enabled, then the
        // attributes we are interested in cannot be present anyway, so
        // skip the walk.
        if !tcx.features().rustc_attrs {
            return;
        }

        // Find annotations supplied by user (if any).
        let (if_this_changed, then_this_would_need) = {
            let mut visitor =
                IfThisChanged { tcx, if_this_changed: vec![], then_this_would_need: vec![] };
            visitor.process_attrs(hir::CRATE_HIR_ID);
            tcx.hir().visit_all_item_likes(&mut visitor.as_deep_visitor());
            (visitor.if_this_changed, visitor.then_this_would_need)
        };

        if !if_this_changed.is_empty() || !then_this_would_need.is_empty() {
            assert!(
                tcx.sess.opts.debugging_opts.query_dep_graph,
                "cannot use the `#[{}]` or `#[{}]` annotations \
                    without supplying `-Z query-dep-graph`",
                sym::rustc_if_this_changed,
                sym::rustc_then_this_would_need
            );
        }

        // Check paths.
        check_paths(tcx, &if_this_changed, &then_this_would_need);
    })
}

type Sources = Vec<(Span, DefId, DepNode)>;
type Targets = Vec<(Span, Symbol, hir::HirId, DepNode)>;

struct IfThisChanged<'tcx> {
    tcx: TyCtxt<'tcx>,
    if_this_changed: Sources,
    then_this_would_need: Targets,
}

impl<'tcx> IfThisChanged<'tcx> {
    fn argument(&self, attr: &ast::Attribute) -> Option<Symbol> {
        let mut value = None;
        for list_item in attr.meta_item_list().unwrap_or_default() {
            match list_item.ident() {
                Some(ident) if list_item.is_word() && value.is_none() => value = Some(ident.name),
                _ =>
                // FIXME better-encapsulate meta_item (don't directly access `node`)
                {
                    span_bug!(list_item.span(), "unexpected meta-item {:?}", list_item)
                }
            }
        }
        value
    }

    fn process_attrs(&mut self, hir_id: hir::HirId) {
        let def_id = self.tcx.hir().local_def_id(hir_id);
        let def_path_hash = self.tcx.def_path_hash(def_id.to_def_id());
        let attrs = self.tcx.hir().attrs(hir_id);
        for attr in attrs {
            if attr.has_name(sym::rustc_if_this_changed) {
                let dep_node_interned = self.argument(attr);
                let dep_node = match dep_node_interned {
                    None => {
                        DepNode::from_def_path_hash(self.tcx, def_path_hash, DepKind::hir_owner)
                    }
                    Some(n) => {
                        match DepNode::from_label_string(self.tcx, n.as_str(), def_path_hash) {
                            Ok(n) => n,
                            Err(()) => {
                                self.tcx.sess.span_fatal(
                                    attr.span,
                                    &format!("unrecognized DepNode variant {:?}", n),
                                );
                            }
                        }
                    }
                };
                self.if_this_changed.push((attr.span, def_id.to_def_id(), dep_node));
            } else if attr.has_name(sym::rustc_then_this_would_need) {
                let dep_node_interned = self.argument(attr);
                let dep_node = match dep_node_interned {
                    Some(n) => {
                        match DepNode::from_label_string(self.tcx, n.as_str(), def_path_hash) {
                            Ok(n) => n,
                            Err(()) => {
                                self.tcx.sess.span_fatal(
                                    attr.span,
                                    &format!("unrecognized DepNode variant {:?}", n),
                                );
                            }
                        }
                    }
                    None => {
                        self.tcx.sess.span_fatal(attr.span, "missing DepNode variant");
                    }
                };
                self.then_this_would_need.push((
                    attr.span,
                    dep_node_interned.unwrap(),
                    hir_id,
                    dep_node,
                ));
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for IfThisChanged<'tcx> {
    type Map = Map<'tcx>;

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::OnlyBodies(self.tcx.hir())
    }

    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        self.process_attrs(item.hir_id());
        intravisit::walk_item(self, item);
    }

    fn visit_trait_item(&mut self, trait_item: &'tcx hir::TraitItem<'tcx>) {
        self.process_attrs(trait_item.hir_id());
        intravisit::walk_trait_item(self, trait_item);
    }

    fn visit_impl_item(&mut self, impl_item: &'tcx hir::ImplItem<'tcx>) {
        self.process_attrs(impl_item.hir_id());
        intravisit::walk_impl_item(self, impl_item);
    }

    fn visit_field_def(&mut self, s: &'tcx hir::FieldDef<'tcx>) {
        self.process_attrs(s.hir_id);
        intravisit::walk_field_def(self, s);
    }
}

fn check_paths<'tcx>(tcx: TyCtxt<'tcx>, if_this_changed: &Sources, then_this_would_need: &Targets) {
    // Return early here so as not to construct the query, which is not cheap.
    if if_this_changed.is_empty() {
        for &(target_span, _, _, _) in then_this_would_need {
            tcx.sess.span_err(target_span, "no `#[rustc_if_this_changed]` annotation detected");
        }
        return;
    }
    tcx.dep_graph.with_query(|query| {
        for &(_, source_def_id, ref source_dep_node) in if_this_changed {
            let dependents = query.transitive_predecessors(source_dep_node);
            for &(target_span, ref target_pass, _, ref target_dep_node) in then_this_would_need {
                if !dependents.contains(&target_dep_node) {
                    tcx.sess.span_err(
                        target_span,
                        &format!(
                            "no path from `{}` to `{}`",
                            tcx.def_path_str(source_def_id),
                            target_pass
                        ),
                    );
                } else {
                    tcx.sess.span_err(target_span, "OK");
                }
            }
        }
    });
}

fn dump_graph(query: &DepGraphQuery) {
    let path: String = env::var("RUST_DEP_GRAPH").unwrap_or_else(|_| "dep_graph".to_string());

    let nodes = match env::var("RUST_DEP_GRAPH_FILTER") {
        Ok(string) => {
            // Expect one of: "-> target", "source -> target", or "source ->".
            let edge_filter =
                EdgeFilter::new(&string).unwrap_or_else(|e| bug!("invalid filter: {}", e));
            let sources = node_set(&query, &edge_filter.source);
            let targets = node_set(&query, &edge_filter.target);
            filter_nodes(&query, &sources, &targets)
        }
        Err(_) => query.nodes().into_iter().collect(),
    };
    let edges = filter_edges(&query, &nodes);

    {
        // dump a .txt file with just the edges:
        let txt_path = format!("{}.txt", path);
        let mut file = BufWriter::new(File::create(&txt_path).unwrap());
        for &(ref source, ref target) in &edges {
            write!(file, "{:?} -> {:?}\n", source, target).unwrap();
        }
    }

    {
        // dump a .dot file in graphviz format:
        let dot_path = format!("{}.dot", path);
        let mut v = Vec::new();
        dot::render(&GraphvizDepGraph(nodes, edges), &mut v).unwrap();
        fs::write(dot_path, v).unwrap();
    }
}

#[allow(missing_docs)]
pub struct GraphvizDepGraph<'q>(FxHashSet<&'q DepNode>, Vec<(&'q DepNode, &'q DepNode)>);

impl<'a, 'q> dot::GraphWalk<'a> for GraphvizDepGraph<'q> {
    type Node = &'q DepNode;
    type Edge = (&'q DepNode, &'q DepNode);
    fn nodes(&self) -> dot::Nodes<'_, &'q DepNode> {
        let nodes: Vec<_> = self.0.iter().cloned().collect();
        nodes.into()
    }
    fn edges(&self) -> dot::Edges<'_, (&'q DepNode, &'q DepNode)> {
        self.1[..].into()
    }
    fn source(&self, edge: &(&'q DepNode, &'q DepNode)) -> &'q DepNode {
        edge.0
    }
    fn target(&self, edge: &(&'q DepNode, &'q DepNode)) -> &'q DepNode {
        edge.1
    }
}

impl<'a, 'q> dot::Labeller<'a> for GraphvizDepGraph<'q> {
    type Node = &'q DepNode;
    type Edge = (&'q DepNode, &'q DepNode);
    fn graph_id(&self) -> dot::Id<'_> {
        dot::Id::new("DependencyGraph").unwrap()
    }
    fn node_id(&self, n: &&'q DepNode) -> dot::Id<'_> {
        let s: String = format!("{:?}", n)
            .chars()
            .map(|c| if c == '_' || c.is_alphanumeric() { c } else { '_' })
            .collect();
        debug!("n={:?} s={:?}", n, s);
        dot::Id::new(s).unwrap()
    }
    fn node_label(&self, n: &&'q DepNode) -> dot::LabelText<'_> {
        dot::LabelText::label(format!("{:?}", n))
    }
}

// Given an optional filter like `"x,y,z"`, returns either `None` (no
// filter) or the set of nodes whose labels contain all of those
// substrings.
fn node_set<'q>(
    query: &'q DepGraphQuery,
    filter: &DepNodeFilter,
) -> Option<FxHashSet<&'q DepNode>> {
    debug!("node_set(filter={:?})", filter);

    if filter.accepts_all() {
        return None;
    }

    Some(query.nodes().into_iter().filter(|n| filter.test(n)).collect())
}

fn filter_nodes<'q>(
    query: &'q DepGraphQuery,
    sources: &Option<FxHashSet<&'q DepNode>>,
    targets: &Option<FxHashSet<&'q DepNode>>,
) -> FxHashSet<&'q DepNode> {
    if let Some(sources) = sources {
        if let Some(targets) = targets {
            walk_between(query, sources, targets)
        } else {
            walk_nodes(query, sources, OUTGOING)
        }
    } else if let Some(targets) = targets {
        walk_nodes(query, targets, INCOMING)
    } else {
        query.nodes().into_iter().collect()
    }
}

fn walk_nodes<'q>(
    query: &'q DepGraphQuery,
    starts: &FxHashSet<&'q DepNode>,
    direction: Direction,
) -> FxHashSet<&'q DepNode> {
    let mut set = FxHashSet::default();
    for &start in starts {
        debug!("walk_nodes: start={:?} outgoing?={:?}", start, direction == OUTGOING);
        if set.insert(start) {
            let mut stack = vec![query.indices[start]];
            while let Some(index) = stack.pop() {
                for (_, edge) in query.graph.adjacent_edges(index, direction) {
                    let neighbor_index = edge.source_or_target(direction);
                    let neighbor = query.graph.node_data(neighbor_index);
                    if set.insert(neighbor) {
                        stack.push(neighbor_index);
                    }
                }
            }
        }
    }
    set
}

fn walk_between<'q>(
    query: &'q DepGraphQuery,
    sources: &FxHashSet<&'q DepNode>,
    targets: &FxHashSet<&'q DepNode>,
) -> FxHashSet<&'q DepNode> {
    // This is a bit tricky. We want to include a node only if it is:
    // (a) reachable from a source and (b) will reach a target. And we
    // have to be careful about cycles etc.  Luckily efficiency is not
    // a big concern!

    #[derive(Copy, Clone, PartialEq)]
    enum State {
        Undecided,
        Deciding,
        Included,
        Excluded,
    }

    let mut node_states = vec![State::Undecided; query.graph.len_nodes()];

    for &target in targets {
        node_states[query.indices[target].0] = State::Included;
    }

    for source in sources.iter().map(|&n| query.indices[n]) {
        recurse(query, &mut node_states, source);
    }

    return query
        .nodes()
        .into_iter()
        .filter(|&n| {
            let index = query.indices[n];
            node_states[index.0] == State::Included
        })
        .collect();

    fn recurse(query: &DepGraphQuery, node_states: &mut [State], node: NodeIndex) -> bool {
        match node_states[node.0] {
            // known to reach a target
            State::Included => return true,

            // known not to reach a target
            State::Excluded => return false,

            // backedge, not yet known, say false
            State::Deciding => return false,

            State::Undecided => {}
        }

        node_states[node.0] = State::Deciding;

        for neighbor_index in query.graph.successor_nodes(node) {
            if recurse(query, node_states, neighbor_index) {
                node_states[node.0] = State::Included;
            }
        }

        // if we didn't find a path to target, then set to excluded
        if node_states[node.0] == State::Deciding {
            node_states[node.0] = State::Excluded;
            false
        } else {
            assert!(node_states[node.0] == State::Included);
            true
        }
    }
}

fn filter_edges<'q>(
    query: &'q DepGraphQuery,
    nodes: &FxHashSet<&'q DepNode>,
) -> Vec<(&'q DepNode, &'q DepNode)> {
    query
        .edges()
        .into_iter()
        .filter(|&(source, target)| nodes.contains(source) && nodes.contains(target))
        .collect()
}
