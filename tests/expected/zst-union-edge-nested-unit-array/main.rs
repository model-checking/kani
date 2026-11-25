// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem::{size_of, size_of_val};
use std::ptr;

// The ZST arm is deliberately first, including for the nested shapes.
#[repr(C)]
union Edge {
    zst: [[(); 2]; 3],
    bytes: [u8; 16],
}

#[kani::proof]
fn copy_nested_unit_array() {
    let mut src = Edge { zst: [[(); 2]; 3] };
    let mut dst = Edge { zst: [[(); 2]; 3] };
    assert_eq!(size_of::<[[(); 2]; 3]>(), 0);
    assert_eq!(size_of::<Edge>(), 16);
    unsafe {
        ptr::write_bytes((&mut src as *mut Edge).cast::<u8>(), 0x5a, size_of::<Edge>());
        ptr::write(&mut dst, ptr::read(&src));
        assert_eq!(dst.zst[2].len(), 2);
        assert_eq!(size_of_val(&dst.zst[2][1]), 0);
        assert_eq!(dst.zst.len(), 3);
        // The byte arm spans the full extent; these are initialized data bytes.
        assert_eq!(dst.bytes, [0x5a; 16]);
        kani::cover!(
            dst.bytes[15] == 0x5a && dst.zst[2].len() == 2,
            "copy_nested_unit_array post-copy data and projection"
        );
    }
}
