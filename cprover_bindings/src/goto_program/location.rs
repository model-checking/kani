// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::cbmc_string::{InternStringOption, InternedString};
use std::convert::TryInto;
use std::fmt::Debug;

/// A `Location` represents a source location.
#[derive(Copy, Clone, Debug)]
pub enum Location {
    /// Unknown source location
    None,
    /// Code is in a builtin function
    BuiltinFunction { function_name: InternedString, line: Option<u64> },
    /// Location in user code.
    /// `function` is `None` for global, `Some(function_name)` for function local.
    Loc { file: InternedString, function: Option<InternedString>, line: u64, col: Option<u64> },
    /// Location for Statements that use Property Class and Description - Assert, Assume, Cover etc.
    Property {
        file: InternedString,
        function: Option<InternedString>,
        line: u64,
        col: Option<u64>,
        comment: InternedString,
        property_class: InternedString,
    },
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

    pub fn filename(&self) -> Option<String> {
        match self {
            Location::Loc { file, .. } => Some(file.to_string()),
            _ => None,
        }
    }

    pub fn line(&self) -> Option<u64> {
        match self {
            Location::Loc { line, .. } => Some(*line),
            _ => None,
        }
    }

    pub fn function_name(&self) -> Option<InternedString> {
        match self {
            Location::Loc { function, .. } => *function,
            _ => None,
        }
    }

    pub fn column_number(&self) -> Option<u64> {
        match self {
            Location::Loc { col, .. } => *col,
            _ => None,
        }
    }

    pub fn comment(&self) -> Option<String> {
        match self {
            Location::Property { comment, .. } => Some(comment.to_string()),
            _ => None,
        }
    }

    pub fn property_class(&self) -> Option<String> {
        match self {
            Location::Property { property_class, .. } => Some(property_class.to_string()),
            _ => None,
        }
    }

    /// Convert a location to a short string suitable for (e.g.) logging.
    /// Goal is to return just "file:line" as clearly as possible.
    pub fn short_string(&self) -> String {
        match self {
            Location::None => "<none>".to_string(),
            Location::BuiltinFunction { function_name, line: Some(line)} => {
                format!("<{}>:{}", function_name, line)
            }
            Location::BuiltinFunction { function_name, line: None} => {
                format!("<{}>", function_name)
            }
            Location::Loc { file, line, .. } => format!("{}:{}", file, line),
            Location::Property { property_class, comment, .. } => {
                format!("{:?}:{:?}", property_class, comment)
            }
        }
    }
}

/// Constructors
impl Location {
    pub fn new<T, U: Into<InternedString>, V: Into<InternedString>>(
        file: U,
        function: Option<V>,
        line: T,
        col: Option<T>,
    ) -> Location
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let file = file.into();
        let line = line.try_into().unwrap();
        let col = col.map(|x| x.try_into().unwrap());
        let function = function.intern();
        Location::Loc { file, function, line, col }
    }

    pub fn property_location<T, U: Into<InternedString>, V: Into<InternedString>, X: Into<InternedString>>(
        file: U,
        function: Option<V>,
        line: T,
        col: Option<T>,
        comment: X,
        property_name: X,
    ) -> Location
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let file = file.into();
        let line = line.try_into().unwrap();
        let col = col.map(|x| x.try_into().unwrap());
        let function = function.intern();
        let property_class = property_name.into();
        let comment = comment.into();
        Location::Property { file, function, line, col, comment, property_class }
    }

    pub fn none() -> Location {
        Location::None
    }

    pub fn builtin_function<T: Into<InternedString>>(name: T, line: Option<u64>) -> Location {
        let function_name = name.into();
        Location::BuiltinFunction { function_name, line}
    }
}
