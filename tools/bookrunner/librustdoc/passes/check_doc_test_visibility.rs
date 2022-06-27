// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! Looks for items missing (or incorrectly having) doctests.
//!
//! This pass is overloaded and runs two different lints.
//!
//! - MISSING_DOC_CODE_EXAMPLES: this lint is **UNSTABLE** and looks for public items missing doctests.
//! - PRIVATE_DOC_TESTS: this lint is **STABLE** and looks for private items with doctests.

use crate::html::markdown::{Ignore, LangString};

pub(crate) struct Tests {
    pub(crate) found_tests: usize,
}

impl crate::doctest::Tester for Tests {
    fn add_test(&mut self, _: String, config: LangString, _: usize) {
        if config.rust && config.ignore == Ignore::None {
            self.found_tests += 1;
        }
    }
}
