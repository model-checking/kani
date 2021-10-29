// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// cbmc-flags: --unsigned-overflow-check
use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
struct CbmcHashMap<K, V, S = RandomState> {
    len: usize,
    _k: PhantomData<K>,
    _s: PhantomData<S>,
    last: Option<V>,
}

impl<K, V: Copy> CbmcHashMap<K, V> {
    fn new() -> CbmcHashMap<K, V> {
        CbmcHashMap { len: 0, _k: PhantomData, _s: PhantomData, last: None }
    }
    fn len(&self) -> usize {
        self.len
    }
}

impl<K, V, S> CbmcHashMap<K, V, S> {
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.last = Some(v);
        if rmc::nondet() {
            self.len += 1;
        }
        rmc::nondet()
    }
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        if false { rmc::nondet() } else { self.last.as_ref() }
    }
}

fn make_map_visible<K, V: Copy>() {
    let mut v: CbmcHashMap<K, V> = CbmcHashMap::new();
    v.len;
    v.insert(rmc::nondet(), rmc::nondet());
    let k: K = rmc::nondet();
    v.get(&k);
}

fn test_actual_map<K: Eq + Copy + Hash, V: Copy>(key: K, value: V) {
    make_map_visible::<K, V>();

    let mut v: HashMap<K, V> = HashMap::new();
    let a = v.insert(key, value);
    assert!(a.is_some());
    assert!(a.is_none());
    let b = v.get(&key);
    assert!(b.is_some());
    assert!(b.is_none());
}

pub fn main() {
    test_actual_map::<i32, i32>(1, 3);
}
