// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::cbmc_string::{InternStringOption, InternedString};
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
    Loc {
        file: InternedString,
        function: Option<InternedString>,
        start_line: u64,
        start_col: Option<u64>,
        end_line: u64,
        end_col: Option<u64>,
    },
    /// Location for Statements that use Property Class and Description - Assert, Assume, Cover etc.
    Property {
        file: InternedString,
        function: Option<InternedString>,
        line: u64,
        col: Option<u64>,
        comment: InternedString,
        property_class: InternedString,
    },
    /// Covers cases where Location Details are unknown or set as None but Property Class is needed.
    PropertyUnknownLocation { comment: InternedString, property_class: InternedString },
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

    pub fn start_line(&self) -> Option<u64> {
        match self {
            Location::Loc { start_line, .. } => Some(*start_line),
            _ => None,
        }
    }

    pub fn end_line(&self) -> Option<u64> {
        match self {
            Location::Loc { end_line, .. } => Some(*end_line),
            _ => None,
        }
    }

    /// Convert a location to a short string suitable for (e.g.) logging.
    /// Goal is to return just "file:line" as clearly as possible.
    pub fn short_string(&self) -> String {
        match self {
            Location::None => "<none>".to_string(),
            Location::BuiltinFunction { function_name, line: Some(line) } => {
                format!("<{function_name}>:{line}")
            }
            Location::BuiltinFunction { function_name, line: None } => {
                format!("<{function_name}>")
            }
            Location::Loc { file, start_line: line, .. } => format!("{file}:{line}"),
            Location::Property { file, line, .. } => {
                format!("<{file:?}>:{line}")
            }
            Location::PropertyUnknownLocation { .. } => "<none>".to_string(),
        }
    }
}

/// Constructors
impl Location {
    pub fn new<T, U: Into<InternedString>, V: Into<InternedString>>(
        file: U,
        function: Option<V>,
        start_line: T,
        start_col: Option<T>,
        end_line: T,
        end_col: Option<T>,
    ) -> Location
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let file_into = file.into();
        let start_line_into = start_line.try_into().unwrap();
        let start_col_into = start_col.map(|x| x.try_into().unwrap());
        let end_line_into = end_line.try_into().unwrap();
        let end_col_into = end_col.map(|x| x.try_into().unwrap());
        let function_into = function.intern();
        Location::Loc {
            file: file_into,
            function: function_into,
            start_line: start_line_into,
            start_col: start_col_into,
            end_line: end_line_into,
            end_col: end_col_into,
        }
    }

    /// Create a Property type Location
    pub fn property_location<T, U>(
        file: Option<U>,
        function: Option<U>,
        line: T,
        col: Option<T>,
        comment: U,
        property_name: U,
    ) -> Location
    where
        T: TryInto<u64>,
        T::Error: Debug,
        U: Into<InternedString>,
    {
        let file = file.unwrap().into();
        let line = line.try_into().unwrap();
        let col = col.map(|x| x.try_into().unwrap());
        let function = function.intern();
        let property_class = property_name.into();
        let comment = comment.into();
        Location::Property { file, function, line, col, comment, property_class }
    }

    /// Create a Property type Location from an already existing Location type
    pub fn create_location_with_property<T: Into<InternedString>>(
        comment: T,
        property_name: T,
        location: Self,
    ) -> Self {
        match location {
            Location::BuiltinFunction { function_name, line } => Location::property_location(
                None,
                Some(function_name),
                line.unwrap(),
                None,
                comment.into(),
                property_name.into(),
            ),
            Location::Loc { file, function, start_line, start_col, end_line: _, end_col: _ } => {
                Location::property_location(
                    file.into(),
                    function.intern(),
                    start_line,
                    start_col,
                    comment.into(),
                    property_name.into(),
                )
            }
            Location::Property { .. } => location,
            Location::PropertyUnknownLocation { .. } => location,
            // This converts None type Locations to PropertyUnknownLocation type which inserts Property Class and Description
            // into the Source Location Irep's without any location details.
            Location::None => Location::PropertyUnknownLocation {
                comment: comment.into(),
                property_class: property_name.into(),
            },
        }
    }

    pub fn none() -> Location {
        Location::None
    }

    pub fn builtin_function<T: Into<InternedString>>(name: T, line: Option<u64>) -> Location {
        let function_name = name.into();
        Location::BuiltinFunction { line, function_name }
    }
}
