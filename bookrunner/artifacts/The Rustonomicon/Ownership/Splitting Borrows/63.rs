// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
use std::slice::from_raw_parts_mut;
struct FakeSlice<T>(T);
impl<T> FakeSlice<T> {
fn len(&self) -> usize { unimplemented!() }
fn as_mut_ptr(&mut self) -> *mut T { unimplemented!() }
pub fn split_at_mut(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
    let len = self.len();
    let ptr = self.as_mut_ptr();

    unsafe {
        assert!(mid <= len);

        (from_raw_parts_mut(ptr, mid),
         from_raw_parts_mut(ptr.add(mid), len - mid))
    }
}
}
}