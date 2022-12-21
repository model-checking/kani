// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test ensures:
//!   1. Assess is able to correctly build and report on a _workspace_
//!   2. Assess is able to correctly count the number of packages (2),
//!      in the presence of an integration test (which might otherwise
//!      look like three crates:
//!       'assess-artifact', 'subpackage', and 'integ')

#[test]
fn an_unsupported_test_from_the_lib() {
    // unsupported feature: try instrinsic
    assert!(std::panic::catch_unwind(|| panic!("test")).is_err());
}

#[test]
fn a_supported_test_from_the_lib() {
    assert!(1 == 1);
}
