// TODO how does licensing work if I took this code from a crate?

extern crate kani;

use kani::ensures as post;
use std::{
    iter::FromIterator,
    ops::{Index, IndexMut},
};

pub fn implies(premise: bool, conclusion: bool) -> bool {
    !premise || conclusion
}

/// A std::vec::Vec based Map, motivated by the fact that, for some key types,
/// iterating over a vector can be faster than other methods for small maps.
///
/// Most of the operations on this map implementation work in O(n), including
/// some of the ones that are O(1) in HashMap. However, optimizers can work magic with
/// contiguous arrays like Vec, and so for small sets (up to 256 elements for integer keys,
/// for example), iterating through a vector actually yields better performance than the
/// less branch- and cache-predictable hash maps.
///
/// To keep item removal fast, this container doesn't form guaranties on item ordering,
/// nor on the stability of the ordering.
///
/// The good news about that is that you're free to mutate keys if your use-case requires that,
/// though I wouldn't recommend it: the underlying vector can be accessed through the unsafe part
/// of the API, in hopes to discourage you from using it.
///
/// Checking equality between maps is defined as "both maps are the same set", and performs worst
/// for maps that are permutations of each other.
#[derive(Clone, Default)]
pub struct VecMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
}

// #[invariant(self.keys.len() == self.values.len())]
impl<K, V> VecMap<K, V> {
    #[post(result.len() == 0)]
    pub fn new() -> Self
    where
        K: PartialEq,
    {
        Self::with_capacity(0)
    }

    #[post(result.len() == 0)]
    pub fn with_capacity(capacity: usize) -> Self
    where
        K: PartialEq,
    {
        VecMap { keys: Vec::with_capacity(capacity), values: Vec::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.keys.capacity().min(self.values.capacity())
    }

    // #[post(self.len() == 0)]
    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }

    #[inline]
    fn position<Q: PartialEq<K> + ?Sized>(&self, key: &Q) -> Option<usize> {
        self.keys.iter().position(|k| key == k)
    }

    pub fn contains_key<Q: PartialEq<K> + ?Sized>(&self, key: &Q) -> bool {
        self.position(key).is_some()
    }

    #[post(implies(!self.contains_key(key), result.is_none()))]
    #[post(implies(self.contains_key(key), result.is_some()))]
    pub fn get<'l, Q: PartialEq<K> + ?Sized>(&'l self, key: &Q) -> Option<&'l V> {
        self.position(key).map(|p| &self.values[p])
    }

    //#[post(implies(!old(self.contains_key(key)), result.is_none()))]
    //#[post(implies(old(self.contains_key(key)), result.is_some()))]
    pub fn get_mut<'l, Q: PartialEq<K> + ?Sized>(&'l mut self, key: &Q) -> Option<&'l mut V> {
        self.position(key).map(move |p| &mut self.values[p])
    }

    //#[post(implies(!old(self.contains_key(&key)), result.is_none()))]
    //#[post(implies(old(self.contains_key(&key)), result.is_some()))]
    pub fn insert(&mut self, key: K, mut value: V) -> Option<V>
    where
        K: PartialEq,
    {
        if let Some(position) = self.position(&key) {
            std::mem::swap(&mut value, &mut self.values[position]);
            Some(value)
        } else {
            self.keys.push(key);
            self.values.push(value);
            None
        }
    }

    pub fn drain(&mut self) -> Drain<K, V> {
        Drain { iter: self.keys.drain(..).zip(self.values.drain(..)) }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.keys.reserve(additional);
        self.values.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.keys.shrink_to_fit();
        self.values.shrink_to_fit();
    }

    #[post(implies(!self.contains_key(key), result.is_none()))]
    #[post(implies(self.contains_key(key), result.is_some()))]
    pub fn get_key_value<'l, Q: PartialEq<K> + ?Sized>(
        &'l self,
        key: &Q,
    ) -> Option<(&'l K, &'l V)> {
        self.position(key).map(|p| (&self.keys[p], &self.values[p]))
    }

    //#[post(implies(!old(self.contains_key(key)), result.is_none()))]
    //#[post(implies(old(self.contains_key(key)), result.is_some()))]
    #[post(self.contains_key(key) == false)]
    pub fn remove<Q: PartialEq<K> + ?Sized>(&mut self, key: &Q) -> Option<V> {
        if let Some(index) = self.position(key) {
            self.keys.swap_remove(index);
            Some(self.values.swap_remove(index))
        } else {
            None
        }
    }

    pub fn entry(&mut self, key: K) -> Entry<K, V>
    where
        K: PartialEq,
    {
        match self.keys().enumerate().find(|(_, k)| &&key == k).map(|(n, _)| n) {
            Some(index) => Entry::Occupied(OccupiedEntry { map: self, index }),
            None => Entry::Vacant(VacantEntry { map: self, key }),
        }
    }

    //#[post(implies(!old(self.contains_key(key)), result.is_none()))]
    //#[post(implies(old(self.contains_key(key)), result.is_some()))]
    #[post(self.contains_key(key) == false)]
    pub fn remove_entry<Q: PartialEq<K> + ?Sized>(&mut self, key: &Q) -> Option<(K, V)> {
        if let Some(index) = self.position(key) {
            Some((self.keys.swap_remove(index), self.values.swap_remove(index)))
        } else {
            None
        }
    }

    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        for i in (0..self.len()).rev() {
            if !f(&self.keys[i], &mut self.values[i]) {
                self.keys.swap_remove(i);
                self.values.swap_remove(i);
            }
        }
    }

    pub fn iter(&self) -> Iter<K, V> {
        Iter { iter: self.keys.iter().zip(self.values.iter()) }
    }

    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut { iter: self.keys.iter().zip(self.values.iter_mut()) }
    }

    pub fn sort(&mut self)
    where
        K: Ord,
    {
        let mut indices: Vec<usize> = (0..self.len()).collect();
        indices.sort_unstable_by_key(|i| &self.keys[*i]);
        reorder_vec(&mut self.keys, indices.iter().copied());
        reorder_vec(&mut self.values, indices.iter().copied());
    }

    /// Much faster than `self == other`, but will return false if the order of the data isn't identical.
    /// # Safety
    /// Note that for the order of data with two `VecMap`s to be identical, they must either have been both sorted,
    /// or they must have undergone the insertion and removal of keys in the same order.
    pub unsafe fn identical(&self, other: &Self) -> bool
    where
        K: PartialEq,
        V: PartialEq,
    {
        self.keys == other.keys && self.values == other.values
    }

    pub fn keys(&self) -> Keys<K, V> {
        Keys { iter: self.keys.iter(), _phantom: Default::default() }
    }

    pub fn values(&self) -> Values<K, V> {
        Values { iter: self.values.iter(), _phantom: Default::default() }
    }
}

impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for VecMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

fn reorder_vec<T>(vec: &mut Vec<T>, order: impl Iterator<Item = usize>) {
    use std::mem::MaybeUninit;
    let mut buffer: Vec<MaybeUninit<T>> = vec.iter().map(|_| MaybeUninit::uninit()).collect();
    for (from, to) in order.enumerate() {
        std::mem::swap(&mut vec[to], unsafe { &mut *(buffer[from].as_mut_ptr()) });
    }
    for i in 0..vec.len() {
        std::mem::swap(&mut vec[i], unsafe { &mut *(buffer[i].as_mut_ptr()) });
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for VecMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (key, value) in self.iter() {
            match other.get(key) {
                Some(v) if value == v => {}
                _ => return false,
            }
        }
        true
    }
}

impl<'a, K: PartialEq + Copy + 'a, V: Copy + 'a> Extend<(&'a K, &'a V)> for VecMap<K, V> {
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        for (key, value) in iter.into_iter() {
            self.insert(*key, *value);
        }
    }
}

impl<'a, K: PartialEq, V> Extend<(K, V)> for VecMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter.into_iter() {
            self.insert(key, value);
        }
    }
}

impl<K: PartialEq, V> FromIterator<(K, V)> for VecMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iterator = iter.into_iter();
        let lower = iterator.size_hint().0;
        let mut this = Self::with_capacity(lower);
        this.extend(iterator);
        this
    }
}

impl<'a, Q: PartialEq<K> + ?Sized, K, V> Index<&'a Q> for VecMap<K, V> {
    type Output = V;
    fn index(&self, key: &'a Q) -> &Self::Output {
        self.get(key).unwrap()
    }
}

impl<'a, Q: PartialEq<K> + ?Sized, K, V> IndexMut<&'a Q> for VecMap<K, V> {
    fn index_mut(&mut self, key: &'a Q) -> &mut Self::Output {
        self.get_mut(key).unwrap()
    }
}

impl<'a, K, V> IntoIterator for &'a VecMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut VecMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { iter: self.keys.into_iter().zip(self.values.into_iter()) }
    }
}

#[derive(Clone)]
pub struct IntoIter<K, V> {
    iter: std::iter::Zip<std::vec::IntoIter<K>, std::vec::IntoIter<V>>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    fn next_back(&mut self) -> Option<(K, V)> {
        self.iter.next_back()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// A view into a single occupied location in a `VecMap`.
///
/// See [`VecMap::entry`](struct.VecMap.html#method.entry) for details.
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    map: &'a mut VecMap<K, V>,
    index: usize,
}

/// A view into a single vacant location in a `VecMap`.
///
/// See [`VecMap::entry`](struct.VecMap.html#method.entry) for details.
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    map: &'a mut VecMap<K, V>,
    key: K,
}

/// A view into a single entry in a `VecMap`.
///
/// See [`VecMap::entry`](struct.VecMap.html#method.entry) for details.
pub enum Entry<'a, K: 'a, V: 'a> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, K, V>),

    /// A vacant entry.
    Vacant(VacantEntry<'a, K, V>),
}
use Entry::*;
impl<'a, K, V> Entry<'a, K, V> {
    /// Ensures that the entry is occupied by inserting the given value if it is vacant.
    ///
    /// Returns a mutable reference to the entry's value.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures that the entry is occupied by inserting the the result of the given function if it
    /// is vacant.
    ///
    /// Returns a mutable reference to the entry's value.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default()),
        }
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// Returns a reference to the entry's value.
    pub fn get(&self) -> &V {
        &self.map.values[self.index]
    }

    /// Returns a mutable reference to the entry's value.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.values[self.index]
    }

    /// Returns a mutable reference to the entry's value with the same lifetime as the map.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map.values[self.index]
    }

    /// Replaces the entry's value with the given one and returns the previous value.
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    /// Removes the entry from the map and returns its value.
    pub fn remove(self) -> V {
        self.map.keys.swap_remove(self.index);
        self.map.values.swap_remove(self.index)
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    /// Inserts the entry into the map with the given value.
    ///
    /// Returns a mutable reference to the entry's value with the same lifetime as the map.
    pub fn insert(self, value: V) -> &'a mut V {
        self.map.keys.push(self.key);
        self.map.values.push(value);
        self.map.values.last_mut().unwrap()
    }
}

/// A draining iterator over a `VecMap`.
///
/// See [`VecMap::drain`](struct.VecMap.html#method.drain) for details.
pub struct Drain<'a, K: 'a, V: 'a> {
    iter: std::iter::Zip<std::vec::Drain<'a, K>, std::vec::Drain<'a, V>>,
}

/// An iterator yielding references to a `VecMap`'s keys and their corresponding values.
///
/// See [`VecMap::iter`](struct.VecMap.html#method.iter) for details.
#[derive(Clone)]
pub struct Iter<'a, K: 'a, V: 'a> {
    iter: std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>,
}

/// An iterator yielding references to a `VecMap`'s keys and mutable references to their
/// corresponding values.
///
/// See [`VecMap::iter_mut`](struct.VecMap.html#method.iter_mut) for details.
pub struct IterMut<'a, K: 'a, V: 'a> {
    iter: std::iter::Zip<std::slice::Iter<'a, K>, std::slice::IterMut<'a, V>>,
}

/// An iterator yielding references to a `VecMap`'s keys in arbitrary order.
///
/// See [`VecMap::keys`](struct.VecMap.html#method.keys) for details.
pub struct Keys<'a, K: 'a, V> {
    iter: std::slice::Iter<'a, K>,
    _phantom: std::marker::PhantomData<V>,
}

impl<'a, K, V> Clone for Keys<'a, K, V> {
    fn clone(&self) -> Self {
        Keys { iter: self.iter.clone(), _phantom: Default::default() }
    }
}

/// An iterator yielding references to a `VecMap`'s values in arbitrary order.
///
/// See [`VecMap::values`](struct.VecMap.html#method.values) for details.
pub struct Values<'a, K, V: 'a> {
    iter: std::slice::Iter<'a, V>,
    _phantom: std::marker::PhantomData<K>,
}

impl<'a, K, V> Clone for Values<'a, K, V> {
    fn clone(&self) -> Self {
        Values { iter: self.iter.clone(), _phantom: Default::default() }
    }
}

macro_rules! impl_iter {
    ($typ:ty, $item:ty) => {
        impl<'a, K, V> Iterator for $typ {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<'a, K, V> DoubleEndedIterator for $typ {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
        }

        impl<'a, K, V> ExactSizeIterator for $typ {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
    };
}
impl_iter! {Drain<'a,K,V>,  (K,V)}
impl_iter! {Iter<'a,K,V>,  (&'a K, &'a V)}
impl_iter! {IterMut<'a,K,V>,  (&'a K, &'a mut V)}
impl_iter! {Keys<'a,K,V>,  &'a K}
impl_iter! {Values<'a,K,V>,  &'a V}

#[kani::proof]
#[kani::stub(core::fmt::Arguments::new_const, ArgumentsProxy::new_consts)]
fn reorder() {
    let n = 128;
    let m = 128;
    let expected: Vec<usize> = (0..n).collect();
    let mut test = expected.clone();
    for _ in 0..m {
        let rands: Vec<usize> = test.iter().map(|_| kani::any()).collect();
        test.sort_by_key(|x| rands[*x]);
        let mut indices: Vec<usize> = (0..test.len()).collect();
        indices.sort_unstable_by_key(|i| test[*i]);
        reorder_vec(&mut test, indices.into_iter());
        assert_eq!(test, expected);
    }
    for _ in 0..m {
        let mut map: VecMap<usize, f32> = VecMap::with_capacity(n);
        for _ in 0..n {
            map.insert(kani::any(), kani::any());
        }
        let clone = map.clone();
        map.sort();
        let mut map_iter = map.iter();
        let first = *map_iter.by_ref().take(1).next().unwrap().0;
        assert!(
            map_iter
                .fold(Some(first), |acc, (k, _v)| {
                    let k = *k;
                    match acc {
                        Some(v) if v < k => Some(k),
                        _ => None,
                    }
                })
                .is_some()
        );
        assert_eq!(map, clone);
    }
}

#[kani::proof]
#[kani::stub(core::fmt::Arguments::new_const, ArgumentsProxy::new_consts)]
fn unsized_key_queries() {
    let mut map = VecMap::<String, u8>::new();
    map.insert("foo".to_owned(), 1);
    map.insert("bar".to_owned(), 2);

    assert_eq!(&map["bar"], &2);
}

struct ArgumentProxy {
    position: usize,
    // Eliding format spec, we just need this to take up *some* space
}

struct ArgumentV1Proxy<'a> {
    value: &'a (usize, usize), // hopefully this is ensures the reference is the correct size
                               // Eliding other fields
}
struct ArgumentsProxy<'a> {
    pieces: &'a [&'static str],
    fmt: Option<&'a [ArgumentProxy]>,
    args: &'a [ArgumentV1Proxy<'a>],
}

impl<'a> ArgumentsProxy<'a> {
    fn new_consts(pieces: &'a [&'static str]) -> std::fmt::Arguments<'a> {
        unsafe { std::mem::transmute(ArgumentsProxy { pieces, fmt: None, args: &[] }) }
    }
}
