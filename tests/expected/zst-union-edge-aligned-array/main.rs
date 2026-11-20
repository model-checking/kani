// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem::{align_of, size_of, size_of_val};
use std::ptr;

#[repr(align(8))]
#[derive(Clone, Copy)]
struct Z;

// The ZST arm is deliberately first, including for the nested shapes.
#[repr(C)]
union Edge {
    zst: [Z; 2],
    bytes: [u8; 16],
}

#[kani::proof]
fn copy_aligned_array() {
    let mut src = Edge { zst: [Z; 2] };
    let mut dst = Edge { zst: [Z; 2] };
    assert_eq!(size_of::<[Z; 2]>(), 0);
    assert_eq!(size_of::<Edge>(), 16);
    unsafe {
        ptr::write_bytes((&mut src as *mut Edge).cast::<u8>(), 0x5a, size_of::<Edge>());
        ptr::write(&mut dst, ptr::read(&src));
        assert_eq!(dst.zst.len(), 2);
        assert_eq!(size_of_val(&dst.zst[1]), 0);
        assert_eq!(align_of::<[Z; 2]>(), 8);
        assert_eq!(align_of::<Edge>(), 8);
        // The byte arm spans the full extent; these are initialized data bytes.
        assert_eq!(dst.bytes, [0x5a; 16]);
        kani::cover!(
            dst.bytes[15] == 0x5a && dst.zst.len() == 2,
            "copy_aligned_array post-copy data and projection"
        );
    }
}
