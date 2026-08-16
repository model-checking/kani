// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Utilities for optimizing the order in which we codegen harnesses.
//!
//! When compiling with more than a single thread, the order in which we codegen harnesses can have
//! a non-negligible impact on performance. Specifically, if we handle harnesses that will generate
//! a lot of code near the end of compilation, the main compiler thread can get stuck waiting for
//! worker threads to export that code, slowing down overall compilation.
//!
//! To combat that, [order_harnesses] reorders the harnesses of a codegen unit so the ones expected
//! to generate the most code come first, leaving the thread pool as much time as possible to export
//! them off the critical path. The estimate is produced by a [CodegenHeuristic]; this module
//! currently provides the simple [MostReachableItems] heuristic, but more complex heuristics might
//! be able to improve on it or avoid other kinds of pitfalls.
//!
//! Ordering is intentionally cheap on memory: we run reachability once per harness to obtain the
//! heuristic rating, but retain only the resulting `usize` rather than the reachable set or call
//! graph. Reachability is recomputed during codegen (reusing the transformer's warmed body cache),
//! so peak memory matches codegen of the same unit in its original order.

use crate::kani_middle::codegen_units::Harness;
use crate::kani_middle::reachability::collect_reachable_items;
use crate::kani_middle::transform::BodyTransformation;
use rustc_middle::ty::TyCtxt;
use rustc_public::mir::mono::MonoItem;

/// A heuristic that rates each harness so [order_harnesses] can codegen the highest-rated first.
pub trait CodegenHeuristic {
    /// Rate a harness given the items reachable from it. *Higher is codegen'd earlier.*
    fn rate(reachable: &[MonoItem]) -> usize;
}

/// Rates a harness by **the raw number of items found during reachability analysis**, so that the
/// harnesses with the most reachable items are codegen'd first.
///
/// The number of reachable items seems to be a good proxy for the amount of code we will generate
/// and thus how long both codegen and the goto-file exporting will take. Putting the harnesses that
/// will take the longest first ensures their (slow) export runs on a worker thread while the main
/// thread keeps codegening the remaining harnesses, rather than the main thread stalling on them at
/// the very end of compilation.
pub struct MostReachableItems;

impl CodegenHeuristic for MostReachableItems {
    fn rate(reachable: &[MonoItem]) -> usize {
        reachable.len()
    }
}

/// Return the harnesses of a codegen unit ordered so that the ones the heuristic `H` rates highest
/// come first. `transformer` must be the (shared) transformer for the harnesses' codegen unit; it
/// is used to run reachability and its body cache is warmed as a side effect.
pub fn order_harnesses<'a, H: CodegenHeuristic>(
    harnesses: &'a [Harness],
    tcx: TyCtxt,
    transformer: &mut BodyTransformation,
) -> Vec<&'a Harness> {
    // Skip the extra reachability pass entirely when the order can't matter.
    if harnesses.len() < 2 {
        return harnesses.iter().collect();
    }

    let mut rated: Vec<(&Harness, usize)> = harnesses
        .iter()
        .map(|harness| {
            let (reachable, _call_graph) =
                collect_reachable_items(tcx, transformer, &[MonoItem::Fn(*harness)]);
            (harness, H::rate(&reachable))
        })
        .collect();
    order_highest_rated_first(&mut rated, |&(_, rating)| rating);
    rated.into_iter().map(|(harness, _)| harness).collect()
}

/// Sort `items` in place so that the ones the `rating` function scores highest come first.
///
/// Extracted from [order_harnesses] so the ordering logic can be unit tested without constructing
/// compiler-internal harness/reachability values (see the tests below).
fn order_highest_rated_first<T>(items: &mut [T], rating: impl Fn(&T) -> usize) {
    // `sort_unstable_by_key` is ascending, so rating via `usize::MAX - ...` puts higher-rated items
    // first. We don't care about stability, and for cheap rating fns like the one for
    // `MostReachableItems`, caching the keys isn't likely to make a difference.
    items.sort_unstable_by_key(|item| usize::MAX - rating(item));
}

#[cfg(test)]
mod tests {
    use super::order_highest_rated_first;

    /// The core ordering used by every [CodegenHeuristic](super::CodegenHeuristic): items rated
    /// higher must come first. Mirrors how `MostReachableItems` rates a harness by its reachable
    /// item count, but with plain integers so we don't need a compiler context.
    #[test]
    fn orders_highest_rated_first() {
        let mut items = vec![("a", 3), ("b", 10), ("c", 1), ("d", 7)];
        order_highest_rated_first(&mut items, |&(_, rating)| rating);
        let order: Vec<_> = items.iter().map(|&(name, _)| name).collect();
        assert_eq!(order, ["b", "d", "a", "c"]);
    }

    /// The `usize::MAX - rating` trick must stay correct at the extremes of the rating range.
    #[test]
    fn handles_rating_extremes() {
        let mut items = vec![("mid", 5), ("max", usize::MAX), ("min", 0)];
        order_highest_rated_first(&mut items, |&(_, rating)| rating);
        let order: Vec<_> = items.iter().map(|&(name, _)| name).collect();
        assert_eq!(order, ["max", "mid", "min"]);
    }

    /// Empty and already-sorted inputs must be handled without panicking or reordering.
    #[test]
    fn handles_trivial_inputs() {
        let mut empty: Vec<(&str, usize)> = vec![];
        order_highest_rated_first(&mut empty, |&(_, rating)| rating);
        assert!(empty.is_empty());

        let mut sorted = vec![("a", 9), ("b", 4), ("c", 4), ("d", 0)];
        order_highest_rated_first(&mut sorted, |&(_, rating)| rating);
        // Ratings are non-increasing, so the highest stays first and the lowest stays last.
        assert_eq!(sorted.first().unwrap().0, "a");
        assert_eq!(sorted.last().unwrap().0, "d");
    }
}
