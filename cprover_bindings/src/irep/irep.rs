// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The actual `Irep` structure, and associated constructors, getters, and setters.

use super::super::MachineModel;
use super::super::goto_program::{Location, Type};
use super::{IrepId, ToIrep};
use crate::cbmc_string::InternedString;
use crate::linear_map;
use linear_map::LinearMap;
use num::BigInt;
use std::fmt::Debug;

/// The CBMC serialization format for goto-programs.
/// CBMC implementation code is at:
/// <https://github.com/diffblue/cbmc/blob/develop/src/util/irep.h>
#[derive(Clone, Debug, PartialEq)]
pub struct Irep {
    pub id: IrepId,
    pub sub: Vec<Irep>,
    pub named_sub: LinearMap<IrepId, Irep>,
}

/// Getters
impl Irep {
    pub fn lookup(&self, key: IrepId) -> Option<&Irep> {
        self.named_sub.get(&key)
    }

    pub fn lookup_as_string(&self, id: IrepId) -> Option<String> {
        self.lookup(id).and_then(|x| {
            let s = x.id.to_string();
            if s.is_empty() { None } else { Some(s) }
        })
    }
}

/// Fluent Builders
impl Irep {
    pub fn with_location(self, l: &Location, mm: &MachineModel) -> Self {
        if !l.is_none() {
            self.with_named_sub(IrepId::CSourceLocation, l.to_irep(mm))
        } else {
            self
        }
    }

    /// Adds a `comment` sub to the irep.
    /// Note that there might be comments both on the irep itself and
    /// inside the location sub of the irep.
    pub fn with_comment<T: Into<InternedString>>(self, c: T) -> Self {
        self.with_named_sub(IrepId::Comment, Irep::just_string_id(c))
    }

    pub fn with_named_sub(mut self, key: IrepId, value: Irep) -> Self {
        if !value.is_nil() {
            self.named_sub.insert(key, value);
        }
        self
    }

    pub fn with_named_sub_option(self, key: IrepId, value: Option<Irep>) -> Self {
        match value {
            Some(value) => self.with_named_sub(key, value),
            _ => self,
        }
    }

    pub fn with_type(self, t: &Type, mm: &MachineModel) -> Self {
        self.with_named_sub(IrepId::Type, t.to_irep(mm))
    }
}

/// Predicates
impl Irep {
    pub fn is_just_id(&self) -> bool {
        self.sub.is_empty() && self.named_sub.is_empty()
    }

    pub fn is_just_named_sub(&self) -> bool {
        self.id == IrepId::EmptyString && self.sub.is_empty()
    }

    pub fn is_just_sub(&self) -> bool {
        self.id == IrepId::EmptyString && self.named_sub.is_empty()
    }

    pub fn is_nil(&self) -> bool {
        self.id == IrepId::Nil
    }
}

/// Constructors
impl Irep {
    /// `__attribute__(constructor)`. Only valid as a function return type.
    /// <https://gcc.gnu.org/onlinedocs/gcc-4.7.0/gcc/Function-Attributes.html>
    pub fn constructor() -> Irep {
        Irep::just_id(IrepId::Constructor)
    }

    pub fn empty() -> Irep {
        Irep::just_id(IrepId::Empty)
    }

    pub fn just_bitpattern_id<T>(i: T, width: u64, signed: bool) -> Irep
    where
        T: Into<BigInt>,
    {
        Irep::just_id(IrepId::bitpattern_from_int(i, width, signed))
    }

    pub fn just_id(id: IrepId) -> Irep {
        Irep { id, sub: Vec::new(), named_sub: LinearMap::new() }
    }

    pub fn just_int_id<T>(i: T) -> Irep
    where
        T: Into<BigInt>,
    {
        Irep::just_id(IrepId::from_int(i))
    }
    pub fn just_named_sub(named_sub: LinearMap<IrepId, Irep>) -> Irep {
        Irep { id: IrepId::EmptyString, sub: vec![], named_sub }
    }

    pub fn just_string_id<T: Into<InternedString>>(s: T) -> Irep {
        Irep::just_id(IrepId::from_string(s))
    }

    pub fn just_sub(sub: Vec<Irep>) -> Irep {
        Irep { id: IrepId::EmptyString, sub, named_sub: LinearMap::new() }
    }

    pub fn nil() -> Irep {
        Irep::just_id(IrepId::Nil)
    }

    pub fn one() -> Irep {
        Irep::just_id(IrepId::Id1)
    }

    pub fn zero() -> Irep {
        Irep::just_id(IrepId::Id0)
    }

    pub fn tuple(sub: Vec<Irep>) -> Self {
        Irep {
            id: IrepId::Tuple,
            sub,
            named_sub: linear_map![(IrepId::Type, Irep::just_id(IrepId::Tuple))],
        }
    }
}
