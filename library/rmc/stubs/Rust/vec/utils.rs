// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This file should contain imports and methods which can be used across the
// different abstractions.

// We use methods from libc as they are directly translated into CBMC primitives.
// In which case, if CBMC does better by implementing any optimizations on these
// operations, RMC would do better too.
pub extern crate libc;

// Currently, the way we handle non-determinism is to implement a __nondet::<T>::()
// function which is stubbed to be `unimplemented!()`. However, at a later time
// it could be possible to implement a Nondet<T> trait per type. This would with
// enum types such as Option where we could decide whether we want to return
// a None or a Some(Nondet<T>). That method would likely end up in this file so
// that it can be used throughout.
