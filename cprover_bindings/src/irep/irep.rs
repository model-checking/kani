// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The actual `Irep` structure, and associated constructors, getters, and setters.

use super::super::goto_program::{Location, Type};
use super::super::MachineModel;
use super::{IrepId, ToIrep};
use crate::cbmc_string::InternedString;
use crate::goto_program::PropertyClass;
use crate::InternString;
use num::BigInt;
use std::fmt::Debug;
use vector_map::VecMap;

/// The CBMC serialization format for goto-programs.
/// CBMC implementation code is at:
/// https://github.com/diffblue/cbmc/blob/develop/src/util/irep.h
#[derive(Clone, Debug, PartialEq)]
pub struct Irep {
    pub id: IrepId,
    pub sub: Vec<Irep>,
    pub named_sub: VecMap<IrepId, Irep>,
}

/// Getters
impl Irep {
    pub fn lookup(&self, key: IrepId) -> Option<&Irep> {
        self.named_sub.get(&key)
    }

    pub fn lookup_as_int(&self, id: IrepId) -> Option<&BigInt> {
        self.lookup(id).and_then(|x| match &x.id {
            IrepId::FreeformInteger(i) | IrepId::FreeformHexInteger(i) => Some(i),
            _ => None,
        })
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
            self.with_named_sub(IrepId::CSourceLocation, l.to_irep(&mm))
        } else {
            self
        }
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

    // /// Add Property Class and Message (Description) fields to Source location IRep
    // pub fn with_assert_properties(
    //     self,
    //     property_class: &PropertyClass,
    //     msg: InternedString,
    //     l: &Location,
    //     mm: &MachineModel,
    // ) -> Self {
    //     if !l.is_none() {
    //         let location_irep = self.with_location(l, mm);
    //         location_irep.with_named_sub(IrepId::Comment, Irep::just_string_id(msg)).with_named_sub(
    //             IrepId::PropertyClass,
    //             Irep::just_string_id(property_class.as_str()),
    //         )
    //     } else {
    //         self
    //     }
    // }
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
    /// https://gcc.gnu.org/onlinedocs/gcc-4.7.0/gcc/Function-Attributes.html
    pub fn constructor() -> Irep {
        Irep::just_id(IrepId::Constructor)
    }

    pub fn empty() -> Irep {
        Irep::just_id(IrepId::Empty)
    }

    pub fn just_hex_id<T>(i: T) -> Irep
    where
        T: Into<BigInt>,
    {
        Irep::just_id(IrepId::hex_from_int(i))
    }

    pub fn just_id(id: IrepId) -> Irep {
        Irep { id: id, sub: Vec::new(), named_sub: VecMap::new() }
    }

    pub fn just_int_id<T>(i: T) -> Irep
    where
        T: Into<BigInt>,
    {
        Irep::just_id(IrepId::from_int(i))
    }
    pub fn just_named_sub(named_sub: VecMap<IrepId, Irep>) -> Irep {
        Irep { id: IrepId::EmptyString, sub: vec![], named_sub: named_sub }
    }

    pub fn just_string_id<T: Into<InternedString>>(s: T) -> Irep {
        Irep::just_id(IrepId::from_string(s))
    }

    pub fn just_sub(sub: Vec<Irep>) -> Irep {
        Irep { id: IrepId::EmptyString, sub: sub, named_sub: VecMap::new() }
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
}
