// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[derive(Clone, Copy)]
struct Node {
    next: *const U,
}

// Only the pointer graph should retain this union; reading it would change the root.
#[allow(dead_code)]
union U {
    big: [u8; 16],
    small: Node,
}

#[kani::proof]
fn recursive_arm() {
    // Inferred cast avoids a separately monomorphized union-typed null helper.
    let node = Node { next: 0usize as *const _ };
    let retained = std::hint::black_box(node);
    assert!(retained.next.is_null());
    kani::cover!(retained.next.is_null(), "recursive_arm retained null root");
}
