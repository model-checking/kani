// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[test]
fn an_unsupported_test_from_the_subpackage() {
    // unsupported feature: try instrinsic
    assert!(std::panic::catch_unwind(|| panic!("test")).is_err());
}

#[test]
fn a_supported_test_from_the_subpackage() {
    assert!(1 == 1);
}
