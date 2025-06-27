// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can compile a crate that depends on bzip, and the analysis will only
//! fail if a missing symbol is reachable.

use bzip2::Compression;
use bzip2::read::BzEncoder;

#[kani::proof]
fn check_missing_extern_fn() {
    // Call bzip compressor
    let data: [u8; 10] = kani::any();
    let compressor = BzEncoder::new(&data[..], Compression::best());
    assert_eq!(compressor.total_in(), data.len().try_into().unwrap());
}

#[kani::proof]
fn check_unreachable_extern_fn() {
    let positive = kani::any_where(|v: &i8| *v > 0);
    if positive == 0 {
        // This should be unreachable so verification should succeed.
        check_missing_extern_fn();
    }
}
