// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Assigning to a memory location via pointer dereferencing causes Drop::drop to be called for the location to which the pointer points.
// Here, kani::Arbitrary implementation for MyStruct deterministically sets MyStruct.0 to 1.
// We check whether AnySlice will properly initialize memory making the assertion in drop() to pass.

struct MyStruct(i32);

impl Drop for MyStruct {
    fn drop(&mut self) {
        assert!(self.0 == 1);
    }
}

impl kani::Arbitrary for MyStruct {
    fn any() -> Self {
        MyStruct(1)
    }
}

#[kani::proof]
fn my_proof() {
    let my_slice = kani::slice::any_slice::<MyStruct, 1>();
}
