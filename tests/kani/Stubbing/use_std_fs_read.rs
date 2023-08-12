// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness -Z stubbing
//
//! This tests whether we can correctly account for `use` statements with `std`
//! functions like `std::fs::read` when resolving paths in `kani::stub`
//! attributes.

use std::fs::read;

fn mock_read<P>(_: P) -> std::io::Result<Vec<u8>> {
    Ok(vec![42])
}

#[kani::proof]
#[kani::stub(read, mock_read)]
fn harness() {
    assert_eq!(read("ignored").unwrap()[0], 42);
}
