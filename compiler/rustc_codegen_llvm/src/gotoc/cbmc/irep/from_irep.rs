// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Converts an IRep into the typesafe gotoc representation.
//! Work in progress - only implemented for some types of Ireps.

use super::super::goto_program::{
    BinaryOperand, DatatypeComponent, Expr, Location, Parameter, SelfOperand, Type, UnaryOperand,
};

use super::{Irep, IrepId};
use std::convert::TryInto;

pub trait FromIrep {
    fn from_irep(i: &Irep) -> Self;
}

/// Id converters
pub trait FromIrepId {
    fn from_irep_id(id: IrepId) -> Self;
}

impl FromIrepId for SelfOperand {
    fn from_irep_id(id: IrepId) -> Self {
        match id {
            IrepId::Postincrement => SelfOperand::Postincrement,
            IrepId::Postdecrement => SelfOperand::Postdecrement,
            _ => unreachable!("Invalid IrepId for self operation {:?}", id),
        }
    }
}

impl FromIrepId for UnaryOperand {
    fn from_irep_id(id: IrepId) -> Self {
        match id {
            IrepId::Bitnor => UnaryOperand::Bitnot,
            IrepId::Bswap => UnaryOperand::Bswap,
            IrepId::Not => UnaryOperand::Not,
            IrepId::Popcount => UnaryOperand::Popcount,
            IrepId::UnaryMinus => UnaryOperand::UnaryMinus,
            _ => unreachable!("Invalid IrepId for unary operation {:?}", id),
        }
    }
}

impl FromIrepId for BinaryOperand {
    fn from_irep_id(id: IrepId) -> Self {
        match id {
            IrepId::And => BinaryOperand::And,
            IrepId::Ashr => BinaryOperand::Ashr,
            IrepId::Bitand => BinaryOperand::Bitand,
            IrepId::Bitor => BinaryOperand::Bitor,
            IrepId::Bitxor => BinaryOperand::Bitxor,
            IrepId::Div => BinaryOperand::Div,
            IrepId::Equal => BinaryOperand::Equal,
            IrepId::Ge => BinaryOperand::Ge,
            IrepId::Gt => BinaryOperand::Gt,
            IrepId::IeeeFloatEqual => BinaryOperand::IeeeFloatEqual,
            IrepId::IeeeFloatNotequal => BinaryOperand::IeeeFloatNotequal,
            IrepId::Le => BinaryOperand::Le,
            IrepId::Lshr => BinaryOperand::Lshr,
            IrepId::Lt => BinaryOperand::Lt,
            IrepId::Minus => BinaryOperand::Minus,
            IrepId::Mod => BinaryOperand::Mod,
            IrepId::Mult => BinaryOperand::Mult,
            IrepId::Notequal => BinaryOperand::Notequal,
            IrepId::Or => BinaryOperand::Or,
            IrepId::OverflowMinus => BinaryOperand::OverflowMinus,
            IrepId::OverflowMult => BinaryOperand::OverflowMult,
            IrepId::OverflowPlus => BinaryOperand::OverflowPlus,
            IrepId::Plus => BinaryOperand::Plus,
            IrepId::Shl => BinaryOperand::Shl,
            IrepId::Xor => BinaryOperand::Xor,
            _ => unreachable!("Invalid IrepId for binary operation {:?}", id),
        }
    }
}

/// Main converters
impl<T> FromIrep for Box<T>
where
    T: FromIrep,
{
    fn from_irep(i: &Irep) -> Self {
        Box::new(FromIrep::from_irep(i))
    }
}

impl FromIrep for DatatypeComponent {
    fn from_irep(i: &Irep) -> Self {
        let name = i.lookup_as_string(IrepId::Name).unwrap();
        let typ = FromIrep::from_irep(i.lookup(IrepId::Type).unwrap());
        Type::datatype_component(&name, typ)
    }
}

impl FromIrep for Expr {
    fn from_irep(_i: &Irep) -> Self {
        todo!()
    }
}

impl FromIrep for Location {
    fn from_irep(i: &Irep) -> Self {
        if i.is_nil() {
            Location::none()
        } else {
            //TOOD pick which Scope to use based on the results of this
            let file = i.lookup_as_string(IrepId::File).unwrap();
            let function = i.lookup_as_string(IrepId::Function);
            let line = i.lookup_as_int(IrepId::Line).map(|x| x.try_into().unwrap());
            let column = i.lookup_as_int(IrepId::Column).map(|x| x.try_into().unwrap());
            if function.clone().map_or(false, |f| file == format!("<builtin-library-{}>", f)) {
                Location::builtin_function(&function.unwrap(), line)
            } else {
                Location::new(file, function, line.unwrap(), column)
            }
        }
    }
}

impl FromIrep for Parameter {
    fn from_irep(i: &Irep) -> Self {
        assert!(i.id == IrepId::Parameter);
        assert!(i.sub.is_empty());
        let identifier = i.lookup_as_string(IrepId::CIdentifier);
        let base_name = i.lookup_as_string(IrepId::CBaseName);
        let typ = FromIrep::from_irep(i.lookup(IrepId::Type).unwrap());
        Type::parameter(identifier, base_name, typ)
    }
}

impl FromIrep for Type {
    fn from_irep(i: &Irep) -> Self {
        match i.id {
            IrepId::Array => {
                assert!(i.sub.is_empty());
                let typ = FromIrep::from_irep(i.lookup(IrepId::Type).unwrap());
                let size_expr: Expr = FromIrep::from_irep(i.lookup(IrepId::Size).unwrap());
                assert!(size_expr.is_int_constant());
                //TODO assert that the type width was the machine width, and that its signed
                let size = size_expr.int_constant_value().unwrap().try_into().unwrap();
                Type::Array { typ, size }
            }
            IrepId::Bool => Type::Bool,
            IrepId::CBool => {
                assert!(i.sub.is_empty());
                assert!(i.lookup_as_int(IrepId::Width).unwrap() == 8);
                Type::c_bool()
            }
            IrepId::Code => {
                assert!(i.sub.is_empty());
                let parameters = i
                    .lookup(IrepId::Parameters)
                    .unwrap()
                    .sub
                    .iter()
                    .map(FromIrep::from_irep)
                    .collect();
                let return_type = FromIrep::from_irep(i.lookup(IrepId::ReturnType).unwrap());
                Type::Code { parameters, return_type }
            }
            IrepId::Constructor => {
                assert!(i.is_just_id());
                Type::Constructor
            }
            IrepId::Floatbv => {
                assert!(i.sub.is_empty());
                let f = i.lookup_as_int(IrepId::F).unwrap();
                let width = i.lookup_as_int(IrepId::Width).unwrap();
                match i.lookup(IrepId::CCType).unwrap().id {
                    IrepId::Double => {
                        assert!(f == 52 && width == 64);
                        Type::Double
                    }
                    IrepId::Float => {
                        assert!(f == 23 && width == 32);
                        Type::Float
                    }
                    _ => unreachable!("Can't unexpected float params in {:?}", i),
                }
            }
            IrepId::Empty => {
                assert!(i.is_just_id());
                Type::Empty
            }
            IrepId::Pointer => {
                assert!(i.sub.len() == 1);
                let typ = FromIrep::from_irep(&i.sub[0]);
                //let width = i.get_int_from_named_sub(IrepId::Width).try_into().unwrap();
                Type::Pointer { typ }
            }
            IrepId::Signedbv => {
                assert!(i.sub.is_empty());
                let width = i.lookup_as_int(IrepId::Width).unwrap().try_into().unwrap();
                Type::Signedbv { width }
            }
            IrepId::Struct => {
                assert!(i.sub.is_empty());
                let tag = i.lookup_as_string(IrepId::Tag).unwrap();
                //TODO check that this is the `one` irep.
                if i.lookup(IrepId::Incomplete).is_some() {
                    Type::IncompleteStruct { tag }
                } else {
                    let components = i
                        .lookup(IrepId::Components)
                        .unwrap()
                        .sub
                        .iter()
                        .map(FromIrep::from_irep)
                        .collect();
                    Type::Struct { tag, components }
                }
            }
            IrepId::StructTag => {
                assert!(i.sub.is_empty());
                let tag = i.lookup_as_string(IrepId::Identifier).unwrap();
                Type::StructTag(tag)
            }
            IrepId::UnionTag => {
                assert!(i.sub.is_empty());
                let tag = i.lookup_as_string(IrepId::Identifier).unwrap();
                Type::UnionTag(tag)
            }
            IrepId::Unsignedbv => {
                assert!(i.sub.is_empty());
                let width = i.lookup_as_int(IrepId::Width).unwrap().try_into().unwrap();
                Type::Unsignedbv { width }
            }
            _ => unreachable!("Can't convert {:?} into a Type", i),
        }
    }
}
