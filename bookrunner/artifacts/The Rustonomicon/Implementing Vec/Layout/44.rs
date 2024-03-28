// compile-flags: --edition 2018
#![allow(unused)]
use std::ptr::NonNull;
use std::marker::PhantomData;

pub struct Vec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for Vec<T> {}
unsafe impl<T: Sync> Sync for Vec<T> {}
fn main() {}