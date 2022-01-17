// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[derive(Clone, Copy)]
struct Target {
    x: u32,
    y: u32,
}

struct Container<T> {
    ptr: std::ptr::NonNull<T>,
}

impl<T> Container<T>
where
    T: Copy,
{
    fn new(val: &mut T) -> Self {
        return Container { ptr: std::ptr::NonNull::new(val).unwrap() };
    }
    fn get(&self) -> T {
        return unsafe { *self.ptr.as_ptr() };
    }
}

fn main() {
    let mut x: u32 = 4;
    let container = Container::new(&mut x);
    let _y = container.get();
    assert_eq!(_y, 4);

    let mut target: Target = Target { x: 3, y: 4 };
    let cont = Container::new(&mut target);
    assert!((unsafe { *cont.ptr.as_ptr() }).x == 3);
    assert!((unsafe { *cont.ptr.as_ptr() }).y == 4);
}
