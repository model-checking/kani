// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::convert::TryInto;
use std::fmt::Debug;

/// A `Location` represents a source location.

#[derive(Clone, Debug)]
pub enum Location {
    /// Unknown source location
    None,
    /// Code is in a builtin function
    BuiltinFunction { function_name: String, line: Option<u64> },
    /// Location in user code.
    /// `function` is `None` for global, `Some(function_name)` for function local.
    Loc { file: String, function: Option<String>, line: u64, col: Option<u64> },
}

/// Getters and predicates
impl Location {
    pub fn is_none(&self) -> bool {
        match self {
            Location::None => true,
            _ => false,
        }
    }

    pub fn is_builtin(&self) -> bool {
        match self {
            Location::BuiltinFunction { .. } => true,
            _ => false,
        }
    }
}

/// Constructors
impl Location {
    pub fn new<T>(file: String, function: Option<String>, line: T, col: Option<T>) -> Location
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let line = line.try_into().unwrap();
        let col = col.map(|x| x.try_into().unwrap());
        Location::Loc { file, function, line, col }
    }

    pub fn none() -> Location {
        Location::None
    }

    pub fn builtin_function(name: &str, line: Option<u64>) -> Location {
        Location::BuiltinFunction { line, function_name: name.to_string() }
    }
}
