//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use std::sync::Arc;

use strategy::traits::*;
use test_runner::*;

/// Return type from `Strategy::prop_recursive()`.
pub struct Recursive<B, F> {
    pub(super) base: Arc<B>,
    pub(super) recurse: Arc<F>,
    pub(super) depth: u32,
    pub(super) desired_size: u32,
    pub(super) expected_branch_size: u32,
}

impl<B : fmt::Debug, F> fmt::Debug for Recursive<B, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Recursive")
            .field("base", &self.base)
            .field("recurse", &"<function>")
            .field("depth", &self.depth)
            .field("desired_size", &self.desired_size)
            .field("expected_branch_size", &self.expected_branch_size)
            .finish()
    }
}

impl<B, F> Clone for Recursive<B, F> {
    fn clone(&self) -> Self {
        Recursive {
            base: Arc::clone(&self.base),
            recurse: Arc::clone(&self.recurse),
            depth: self.depth,
            desired_size: self.desired_size,
            expected_branch_size: self.expected_branch_size,
        }
    }
}

impl<T : fmt::Debug + 'static,
     F : Fn (Arc<BoxedStrategy<T>>) -> BoxedStrategy<T>>
Strategy for Recursive<BoxedStrategy<T>, F> {
    type Value = Box<ValueTree<Value = T>>;

    fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
        // Since the generator is stateless, we can't implement any "absolutely
        // X many items" rule. We _can_, however, with extremely high
        // probability, obtain a value near what we want by using decaying
        // probabilities of branching as we go down the tree.
        //
        // We are given a target size S and a branch size K (branch size =
        // expected number of items immediately below each branch). We select
        // some probability P for each level.
        //
        // A single level l is thus expected to hold PlK branches. Each of
        // those will have P(l+1)K child branches of their own, so there are
        // PlP(l+1)K² second-level branches. The total branches in the tree is
        // thus (Σ PlK^l) for l from 0 to infinity. Each level is expected to
        // hold K items, so the total number of items is simply K times the
        // number of branches, or (K Σ PlK^l). So we want to find a P sequence
        // such that (lim (K Σ PlK^l) = S), or more simply,
        // (lim Σ PlK^l = S/K).
        //
        // Let Q be a second probability sequence such that Pl = Ql/K^l. This
        // changes the formulation to (lim Σ Ql = S/K). The series Σ0.5^(l+1)
        // converges on 1.0, so we can let Ql = S/K * 0.5^(l+1), and so
        // Pl = S/K^(l+1) * 0.5^(l+1) = S / (2K) ^ (l+1)
        //
        // We don't actually have infinite levels here since we _can_ easily
        // cap to a fixed max depth, so this will be a minor underestimate. We
        // also clamp all probabilities to 0.9 to ensure that we can't end up
        // with levels which are always pure branches, which further
        // underestimates size.

        let mut branch_probabilities = Vec::new();
        let mut k2 = u64::from(self.expected_branch_size) * 2;
        for _ in 0..self.depth {
            branch_probabilities.push(f64::from(self.desired_size) / k2 as f64);
            k2 = k2.saturating_mul(u64::from(self.expected_branch_size) * 2);
        }

        let mut strat = Arc::clone(&self.base);
        while let Some(branch_probability) = branch_probabilities.pop() {
            let recursive_choice = Arc::new((self.recurse)(Arc::clone(&strat)));
            let non_recursive_choice = strat;
            strat = Arc::new(
                ::bool::weighted(branch_probability.min(0.9))
                    .prop_ind_flat_map(move |branch| if branch {
                        Arc::clone(&recursive_choice)
                    } else {
                        Arc::clone(&non_recursive_choice)
                    }).boxed());
        }

        strat.new_value(runner)
    }
}

#[cfg(test)]
mod test {
    use std::cmp::max;

    use super::*;

    #[test]
    fn test_recursive() {
        #[derive(Clone, Debug)]
        enum Tree {
            Leaf,
            Branch(Vec<Tree>),
        }

        impl Tree {
            fn stats(&self) -> (u32, u32) {
                match *self {
                    Tree::Leaf => (0, 1),
                    Tree::Branch(ref children) => {
                        let mut depth = 0;
                        let mut count = 0;
                        for child in children {
                            let (d, c) = child.stats();
                            depth = max(d, depth);
                            count += c;
                        }

                        (depth + 1, count + 1)
                    }
                }
            }
        }

        let mut max_depth = 0;
        let mut max_count = 0;

        let strat = Just(Tree::Leaf).prop_recursive(
            4, 64, 16,
            |element| ::collection::vec(element, 8..16)
                .prop_map(Tree::Branch).boxed());


        let mut runner = TestRunner::default();
        for _ in 0..65536 {
            let tree = strat.new_value(&mut runner).unwrap().current();
            let (depth, count) = tree.stats();
            assert!(depth <= 4, "Got depth {}", depth);
            assert!(count <= 128, "Got count {}", count);
            max_depth = max(depth, max_depth);
            max_count = max(count, max_count);
        }

        assert!(max_depth >= 3, "Only got max depth {}", max_depth);
        assert!(max_count > 48, "Only got max count {}", max_count);
    }
}
