// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(allocator_api)]

fn __nondet<T>() -> T {
    unimplemented!()
}

use std::marker::PhantomData;
use std::alloc::Allocator;
use std::alloc::Global;
struct CbmcVec<T, A: Allocator = Global> {
    len: usize, _x : PhantomData<T>, last : Option<T>, _y : PhantomData<A>
}

impl <T: Copy> CbmcVec<T> {
    fn new() -> CbmcVec<T> {
        CbmcVec {len: 0, _x: PhantomData, last: None, _y: PhantomData}
    }
}

impl <T: Copy, A: Allocator> CbmcVec<T, A> {
    fn push(&mut self, val : T){
        self.len += 1;
        self.last = Some(val);
    }
    fn len(&self)  -> usize {
        self.len
    }
    fn pop(&mut self) -> Option<T> {
        if (self.len == 0) {
            None
        } else {
            self.len -= 1;
            self.last
            //Some(__nondet())
        }
    }
}
fn make_vec_visible<T: Copy>()
{
    let mut v : CbmcVec<T> = CbmcVec::new();
    v.push(__nondet());
    v.len();
    v.pop();
}

fn test_actual_vec<T: PartialEq + Copy>(to_push:T, not_pushed:T){
    make_vec_visible::<T>();

    let mut v : Vec<T> = Vec::new();
    v.push(to_push);
    assert!(v.len() == 1);
    assert!(v.len() == 11);
    let p = v.pop();
    assert!(p != None);
    assert!(p == None);
    assert!(p == Some(to_push));
    assert!(p == Some(not_pushed));
}

fn main() {
    test_actual_vec::<char>('a', 'b');
    test_actual_vec::<bool>(true,false);
    test_actual_vec::<i8>(1, 3);
    test_actual_vec::<f32>(1.1, 3.14);
    test_actual_vec::<f64>(1.1, 3.14);
}
