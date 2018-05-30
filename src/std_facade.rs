//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module provides #[cfg(..)]ed type aliases over features.

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box as ABox;
#[cfg(feature = "std")]
use std::boxed::Box as ABox;

pub(crate) type Box<T> = ABox<T>;

#[cfg(all(feature = "alloc", not(feature="std")))]
use alloc::arc::Arc as AArc;
#[cfg(feature = "std")]
use std::sync::Arc as AArc;

pub(crate) type Arc<T> = AArc<T>;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec as AVec;
#[cfg(feature = "std")]
use std::vec::Vec as AVec;

pub(crate) type Vec<T> = AVec<T>;

#[cfg(all(feature = "alloc", not(feature="std")))]
use alloc::{BTreeMap as ABTreeMap, BTreeSet as ABTreeSet};
#[cfg(feature = "std")]
use std::collections::{BTreeMap as ABTreeMap, BTreeSet as ABTreeSet};

pub(crate) type BTreeMap<K, V> = ABTreeMap<K, V>;
pub(crate) type BTreeSet<T> = ABTreeSet<T>;
