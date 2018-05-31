//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module provides #[cfg(..)]ed type aliases over features.

macro_rules! multiplex_alloc {
    ($alloc: path, $std: path) => {
        #[cfg(all(feature = "alloc", not(feature = "std")))]
        pub(crate) use $alloc;
        #[cfg(feature = "std")]
        pub(crate) use $std;
    };
}

macro_rules! multiplex_core {
    ($core: path, $std: path) => {
        #[cfg(not(feature = "std"))]
        pub(crate) use $core;
        #[cfg(feature = "std")]
        pub(crate) use $std;
    };
}

multiplex_alloc!(alloc::borrow::Cow, ::std::borrow::Cow);
multiplex_alloc!(alloc::borrow::ToOwned, ::std::borrow::ToOwned);
multiplex_alloc!(alloc::boxed::Box, ::std::boxed::Box);
multiplex_alloc!(alloc::String, ::std::string::String);
multiplex_alloc!(alloc::arc::Arc, ::std::sync::Arc);
multiplex_alloc!(alloc::Vec, ::std::vec::Vec);
multiplex_alloc!(alloc::VecDeque, std::collections::VecDeque);
multiplex_alloc!(alloc::BinaryHeap, ::std::collections::BinaryHeap);
multiplex_alloc!(alloc::LinkedList, ::std::collections::LinkedList);
multiplex_alloc!(alloc::BTreeSet, ::std::collections::BTreeSet);
multiplex_alloc!(alloc::BTreeMap, ::std::collections::BTreeMap);
multiplex_alloc!(hashmap_core::HashMap, ::std::collections::HashMap);
multiplex_alloc!(hashmap_core::HashSet, ::std::collections::HashSet);

multiplex_core!(core::fmt, ::std::fmt);
multiplex_core!(core::cell::Cell, ::std::cell::Cell);
