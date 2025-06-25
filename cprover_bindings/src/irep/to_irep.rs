// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Converts a typed goto-program into the `Irep` serilization format of CBMC
use std::hash::Hash;
use std::mem::ManuallyDrop;

// TODO: consider making a macro to replace `linear_map![])` for initilizing btrees.
use super::super::MachineModel;
use super::super::goto_program;
use super::{Irep, IrepId};
use crate::InternedString;
use crate::linear_map;
use bumpalo::Bump;
use goto_program::{
    BinaryOperator, CIntType, DatatypeComponent, Expr, ExprValue, Lambda, Location, Parameter,
    SelfOperator, Stmt, StmtBody, SwitchCase, SymbolValues, Type, UnaryOperator,
};
use hashbrown::DefaultHashBuilder;
use hashbrown::HashMap;

pub trait ToIrep {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b>;
}

pub(crate) fn collect_into<T: IntoIterator>(
    t: T,
    arena: &Bump,
) -> std::mem::ManuallyDrop<Vec<T::Item, &Bump>> {
    let mut v = Vec::new_in(arena);
    let i = t.into_iter();

    for t in i {
        v.push(t);
    }
    std::mem::ManuallyDrop::new(v)
}

pub fn hash_collect_into<K: Eq + Hash, V, T: IntoIterator<Item = (K, V)>>(
    t: T,
    arena: &Bump,
) -> std::mem::ManuallyDrop<HashMap<K, V, DefaultHashBuilder, &Bump>> {
    let mut h = HashMap::new_in(arena);
    let i = t.into_iter();
    for (k, v) in i {
        h.insert(k, v);
    }
    std::mem::ManuallyDrop::new(h)
}

#[macro_export]
macro_rules! vec_in {
    ($arena:expr $(,)?) => {
        std::mem::ManuallyDrop::new(Vec::new_in($arena))
    };
    ($arena:expr, $($x:expr),+ $(,)?) => {
        collect_into([$($x),+], $arena)
    }
}

/// Utility functions
fn arguments_irep<'b, 'a>(
    arena: &'b Bump,
    arguments: impl Iterator<Item = &'a Expr>,
    mm: &MachineModel,
) -> Irep<'b> {
    Irep {
        id: IrepId::Arguments,
        sub: collect_into(arguments.map(|x| x.to_irep(arena, mm)), arena),
        named_sub: ManuallyDrop::new(HashMap::new_in(arena)),
    }
}
fn code_irep<'b>(
    arena: &'b Bump,
    kind: IrepId,
    ops: std::mem::ManuallyDrop<Vec<Irep<'b>, &'b Bump>>,
) -> Irep<'b> {
    Irep {
        id: IrepId::Code,
        sub: ops,
        named_sub: hash_collect_into([(IrepId::Statement, Irep::just_id(arena, kind))], arena),
    }
}
fn side_effect_irep<'b>(
    arena: &'b Bump,
    kind: IrepId,
    ops: std::mem::ManuallyDrop<Vec<Irep<'b>, &'b Bump>>,
) -> Irep<'b> {
    Irep {
        id: IrepId::SideEffect,
        named_sub: linear_map![arena, (IrepId::Statement, Irep::just_id(arena, kind))],
        sub: ops,
    }
}
fn switch_default_irep<'b>(arena: &'b Bump, body: &Stmt, mm: &MachineModel) -> Irep<'b> {
    code_irep(arena, IrepId::SwitchCase, vec_in![arena, Irep::nil(arena), body.to_irep(arena, mm)])
        .with_named_sub(IrepId::Default, Irep::one(arena))
        .with_owned_location(*body.location(), mm)
}

/// ID Converters
pub trait ToIrepId {
    fn to_irep_id(&self) -> IrepId;
}

impl ToIrepId for BinaryOperator {
    fn to_irep_id(&self) -> IrepId {
        match self {
            BinaryOperator::And => IrepId::And,
            BinaryOperator::Ashr => IrepId::Ashr,
            BinaryOperator::Bitand => IrepId::Bitand,
            BinaryOperator::Bitnand => IrepId::Bitnand,
            BinaryOperator::Bitor => IrepId::Bitor,
            BinaryOperator::Bitxor => IrepId::Bitxor,
            BinaryOperator::Div => IrepId::Div,
            BinaryOperator::Equal => IrepId::Equal,
            BinaryOperator::FloatbvRoundToIntegral => IrepId::FloatbvRoundToIntegral,
            BinaryOperator::Ge => IrepId::Ge,
            BinaryOperator::Gt => IrepId::Gt,
            BinaryOperator::IeeeFloatEqual => IrepId::IeeeFloatEqual,
            BinaryOperator::IeeeFloatNotequal => IrepId::IeeeFloatNotequal,
            BinaryOperator::Implies => IrepId::Implies,
            BinaryOperator::Le => IrepId::Le,
            BinaryOperator::Lshr => IrepId::Lshr,
            BinaryOperator::Lt => IrepId::Lt,
            BinaryOperator::Minus => IrepId::Minus,
            BinaryOperator::Mod => IrepId::Mod,
            BinaryOperator::Mult => IrepId::Mult,
            BinaryOperator::Notequal => IrepId::Notequal,
            BinaryOperator::Or => IrepId::Or,
            BinaryOperator::OverflowMinus => IrepId::OverflowMinus,
            BinaryOperator::OverflowMult => IrepId::OverflowMult,
            BinaryOperator::OverflowPlus => IrepId::OverflowPlus,
            BinaryOperator::OverflowResultMinus => IrepId::OverflowResultMinus,
            BinaryOperator::OverflowResultMult => IrepId::OverflowResultMult,
            BinaryOperator::OverflowResultPlus => IrepId::OverflowResultPlus,
            BinaryOperator::Plus => IrepId::Plus,
            BinaryOperator::ROk => IrepId::ROk,
            BinaryOperator::Rol => IrepId::Rol,
            BinaryOperator::Ror => IrepId::Ror,
            BinaryOperator::Shl => IrepId::Shl,
            BinaryOperator::Xor => IrepId::Xor,
            BinaryOperator::VectorEqual => IrepId::VectorEqual,
            BinaryOperator::VectorNotequal => IrepId::VectorNotequal,
            BinaryOperator::VectorGe => IrepId::VectorGe,
            BinaryOperator::VectorLe => IrepId::VectorLe,
            BinaryOperator::VectorGt => IrepId::VectorGt,
            BinaryOperator::VectorLt => IrepId::VectorLt,
        }
    }
}

impl ToIrepId for SelfOperator {
    fn to_irep_id(&self) -> IrepId {
        match self {
            SelfOperator::Postdecrement => IrepId::Postdecrement,
            SelfOperator::Postincrement => IrepId::Postincrement,
            SelfOperator::Predecrement => IrepId::Predecrement,
            SelfOperator::Preincrement => IrepId::Preincrement,
        }
    }
}

impl ToIrepId for UnaryOperator {
    fn to_irep_id(&self) -> IrepId {
        match self {
            UnaryOperator::Bitnot => IrepId::Bitnot,
            UnaryOperator::BitReverse => IrepId::BitReverse,
            UnaryOperator::Bswap => IrepId::Bswap,
            UnaryOperator::CountLeadingZeros { .. } => IrepId::CountLeadingZeros,
            UnaryOperator::CountTrailingZeros { .. } => IrepId::CountTrailingZeros,
            UnaryOperator::IsDynamicObject => IrepId::IsDynamicObject,
            UnaryOperator::IsFinite => IrepId::IsFinite,
            UnaryOperator::Not => IrepId::Not,
            UnaryOperator::ObjectSize => IrepId::ObjectSize,
            UnaryOperator::PointerObject => IrepId::PointerObject,
            UnaryOperator::PointerOffset => IrepId::PointerOffset,
            UnaryOperator::Popcount => IrepId::Popcount,
            UnaryOperator::UnaryMinus => IrepId::UnaryMinus,
        }
    }
}

/// The main converters
impl ToIrep for DatatypeComponent {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        match self {
            DatatypeComponent::Field { name, typ } => Irep::just_named_sub(
                arena,
                linear_map![
                    arena,
                    (IrepId::Name, Irep::just_string_id(arena, name.to_string())),
                    (IrepId::CPrettyName, Irep::just_string_id(arena, name.to_string())),
                    (IrepId::Type, typ.to_irep(arena, mm)),
                ],
            ),
            DatatypeComponent::UnionField { name, typ: _, padded_typ } => Irep::just_named_sub(
                arena,
                linear_map![
                    arena,
                    (IrepId::Name, Irep::just_string_id(arena, name.to_string())),
                    (IrepId::CPrettyName, Irep::just_string_id(arena, name.to_string())),
                    (IrepId::Type, padded_typ.to_irep(arena, mm)),
                ],
            ),
            DatatypeComponent::Padding { name, bits } => Irep::just_named_sub(
                arena,
                linear_map![
                    arena,
                    (IrepId::CIsPadding, Irep::one(arena)),
                    (IrepId::Name, Irep::just_string_id(arena, name.to_string())),
                    (IrepId::Type, Type::unsigned_int(*bits).to_irep(arena, mm)),
                ],
            ),
        }
    }
}

impl ToIrep for Expr {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        if let ExprValue::IntConstant(i) = self.value() {
            let typ_width = self.typ().native_width(mm);
            let irep_value = if let Some(width) = typ_width {
                Irep::just_bitpattern_id(arena, i.clone(), width, self.typ().is_signed(mm))
            } else {
                Irep::just_int_id(arena, i.clone())
            };
            Irep {
                id: IrepId::Constant,
                sub: vec_in![arena],
                named_sub: linear_map![arena, (IrepId::Value, irep_value,)],
            }
            .with_owned_location(*self.location(), mm)
            .with_owned_type(self.typ().clone(), mm)
        } else {
            self.value()
                .to_irep(arena, mm)
                .with_owned_location(*self.location(), mm)
                .with_owned_type(self.typ().clone(), mm)
        }
        .with_named_sub_option(
            IrepId::CCSizeofType,
            self.size_of_annotation().map(|ty| ty.to_irep(arena, mm)),
        )
    }
}

impl<'b> Irep<'b> {
    pub fn symbol(arena: &'b Bump, identifier: InternedString) -> Self {
        Irep {
            id: IrepId::Symbol,
            sub: vec_in![arena],
            named_sub: linear_map![
                arena,
                (IrepId::Identifier, Irep::just_string_id(arena, identifier))
            ],
        }
    }
}

impl ToIrep for ExprValue {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        match self {
            ExprValue::AddressOf(e) => Irep {
                id: IrepId::AddressOf,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::Array { elems } => Irep {
                id: IrepId::Array,
                sub: collect_into(elems.iter().map(|x| x.to_irep(arena, mm)), arena),
                named_sub: linear_map![arena],
            },
            ExprValue::ArrayOf { elem } => Irep {
                id: IrepId::ArrayOf,
                sub: vec_in![arena, elem.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::Assign { left, right } => side_effect_irep(
                arena,
                IrepId::Assign,
                vec_in![arena, left.to_irep(arena, mm), right.to_irep(arena, mm)],
            ),
            ExprValue::BinOp { op, lhs, rhs } => Irep {
                id: op.to_irep_id(),
                sub: vec_in![arena, lhs.to_irep(arena, mm), rhs.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::BoolConstant(c) => Irep {
                id: IrepId::Constant,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (
                        IrepId::Value,
                        if *c {
                            Irep::just_id(arena, IrepId::True)
                        } else {
                            Irep::just_id(arena, IrepId::False)
                        },
                    )
                ],
            },
            ExprValue::ByteExtract { e, offset } => Irep {
                id: if mm.is_big_endian {
                    IrepId::ByteExtractBigEndian
                } else {
                    IrepId::ByteExtractLittleEndian
                },
                sub: vec_in![
                    arena,
                    e.to_irep(arena, mm),
                    Expr::int_constant(*offset, Type::ssize_t()).to_irep(arena, mm)
                ],
                named_sub: linear_map![arena],
            },
            ExprValue::CBoolConstant(i) => Irep {
                id: IrepId::Constant,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (
                        IrepId::Value,
                        Irep::just_bitpattern_id(
                            arena,
                            if *i { 1u8 } else { 0 },
                            mm.bool_width,
                            false
                        )
                    )
                ],
            },
            ExprValue::Dereference(e) => Irep {
                id: IrepId::Dereference,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            //TODO, determine if there is an endineness problem here
            ExprValue::DoubleConstant(i) => {
                let c: u64 = i.to_bits();
                Irep {
                    id: IrepId::Constant,
                    sub: vec_in![arena],
                    named_sub: linear_map![
                        arena,
                        (IrepId::Value, Irep::just_bitpattern_id(arena, c, mm.double_width, false))
                    ],
                }
            }
            ExprValue::EmptyUnion => Irep::just_id(arena, IrepId::EmptyUnion),
            ExprValue::FloatConstant(i) => {
                let c: u32 = i.to_bits();
                Irep {
                    id: IrepId::Constant,
                    sub: vec_in![arena],
                    named_sub: linear_map![
                        arena,
                        (IrepId::Value, Irep::just_bitpattern_id(arena, c, mm.float_width, false))
                    ],
                }
            }
            ExprValue::Float16Constant(i) => {
                let c: u16 = i.to_bits();
                Irep {
                    id: IrepId::Constant,
                    sub: vec_in![arena],
                    named_sub: linear_map![
                        arena,
                        (IrepId::Value, Irep::just_bitpattern_id(arena, c, 16, false))
                    ],
                }
            }
            ExprValue::Float128Constant(i) => {
                let c: u128 = i.to_bits();
                Irep {
                    id: IrepId::Constant,
                    sub: vec_in![arena],
                    named_sub: linear_map![
                        arena,
                        (IrepId::Value, Irep::just_bitpattern_id(arena, c, 128, false))
                    ],
                }
            }
            ExprValue::FunctionCall { function, arguments } => side_effect_irep(
                arena,
                IrepId::FunctionCall,
                vec_in![
                    arena,
                    function.to_irep(arena, mm),
                    arguments_irep(arena, arguments.iter(), mm)
                ],
            ),
            ExprValue::If { c, t, e } => Irep {
                id: IrepId::If,
                sub: vec_in![
                    arena,
                    c.to_irep(arena, mm),
                    t.to_irep(arena, mm),
                    e.to_irep(arena, mm)
                ],
                named_sub: linear_map![arena],
            },
            ExprValue::Index { array, index } => Irep {
                id: IrepId::Index,
                sub: vec_in![arena, array.to_irep(arena, mm), index.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::IntConstant(_) => {
                unreachable!("Should have been processed in previous step")
            }
            ExprValue::Member { lhs, field } => Irep {
                id: IrepId::Member,
                sub: vec_in![arena, lhs.to_irep(arena, mm)],
                named_sub: linear_map![
                    arena,
                    (IrepId::CLvalue, Irep::one(arena)),
                    (IrepId::ComponentName, Irep::just_string_id(arena, field.to_string())),
                ],
            },
            ExprValue::Nondet => side_effect_irep(arena, IrepId::Nondet, vec_in![arena]),
            ExprValue::PointerConstant(0) => Irep {
                id: IrepId::Constant,
                sub: vec_in![arena],
                named_sub: linear_map![arena, (IrepId::Value, Irep::just_id(arena, IrepId::NULL))],
            },
            ExprValue::PointerConstant(i) => Irep {
                id: IrepId::Constant,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Value, Irep::just_bitpattern_id(arena, *i, mm.pointer_width, false))
                ],
            },
            ExprValue::ReadOk { ptr, size } => Irep {
                id: IrepId::ROk,
                sub: vec_in![arena, ptr.to_irep(arena, mm), size.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::SelfOp { op, e } => {
                side_effect_irep(arena, op.to_irep_id(), vec_in![arena, e.to_irep(arena, mm)])
            }
            ExprValue::StatementExpression { statements: ops, location: loc } => side_effect_irep(
                arena,
                IrepId::StatementExpression,
                vec_in![arena, Stmt::block(ops.to_vec(), *loc).to_irep(arena, mm)],
            ),
            ExprValue::StringConstant { s } => Irep {
                id: IrepId::StringConstant,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Value, Irep::just_string_id(arena, s.to_string()),)
                ],
            },
            ExprValue::Struct { values } => Irep {
                id: IrepId::Struct,
                sub: collect_into(values.iter().map(|x| x.to_irep(arena, mm)), arena),
                named_sub: linear_map![arena],
            },
            ExprValue::Symbol { identifier } => Irep::symbol(arena, *identifier),
            ExprValue::Typecast(e) => Irep {
                id: IrepId::Typecast,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::Union { value, field } => Irep {
                id: IrepId::Union,
                sub: vec_in![arena, value.to_irep(arena, mm)],
                named_sub: linear_map![
                    arena,
                    (IrepId::ComponentName, Irep::just_string_id(arena, field.to_string()),)
                ],
            },
            ExprValue::UnOp { op: UnaryOperator::Bswap, e } => Irep {
                id: IrepId::Bswap,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![arena, (IrepId::BitsPerByte, Irep::just_int_id(arena, 8u8))],
            },
            ExprValue::UnOp { op: UnaryOperator::BitReverse, e } => Irep {
                id: IrepId::BitReverse,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::UnOp { op: UnaryOperator::CountLeadingZeros { allow_zero }, e } => Irep {
                id: IrepId::CountLeadingZeros,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![
                    arena,
                    (
                        IrepId::CBoundsCheck,
                        if *allow_zero { Irep::zero(arena) } else { Irep::one(arena) }
                    )
                ],
            },
            ExprValue::UnOp { op: UnaryOperator::CountTrailingZeros { allow_zero }, e } => Irep {
                id: IrepId::CountTrailingZeros,
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![
                    arena,
                    (
                        IrepId::CBoundsCheck,
                        if *allow_zero { Irep::zero(arena) } else { Irep::one(arena) }
                    )
                ],
            },
            ExprValue::UnOp { op, e } => Irep {
                id: op.to_irep_id(),
                sub: vec_in![arena, e.to_irep(arena, mm)],
                named_sub: linear_map![arena],
            },
            ExprValue::Vector { elems } => Irep {
                id: IrepId::Vector,
                sub: collect_into(elems.iter().map(|x| x.to_irep(arena, mm)), arena),
                named_sub: linear_map![arena],
            },
            ExprValue::Forall { variable, domain } => Irep {
                id: IrepId::Forall,
                sub: vec_in![
                    arena,
                    Irep {
                        id: IrepId::Tuple,
                        sub: vec_in![arena, variable.to_irep(arena, mm)],
                        named_sub: linear_map![arena],
                    },
                    domain.to_irep(arena, mm),
                ],
                named_sub: linear_map![arena],
            },
            ExprValue::Exists { variable, domain } => Irep {
                id: IrepId::Exists,
                sub: vec_in![
                    arena,
                    Irep {
                        id: IrepId::Tuple,
                        sub: vec_in![arena, variable.to_irep(arena, mm)],
                        named_sub: linear_map![arena],
                    },
                    domain.to_irep(arena, mm),
                ],
                named_sub: linear_map![arena],
            },
        }
    }
}

impl ToIrep for Location {
    fn to_irep<'b>(&self, arena: &'b Bump, _mm: &MachineModel) -> Irep<'b> {
        match self {
            Location::None => Irep::nil(arena),
            Location::BuiltinFunction { line, function_name } => Irep::just_named_sub(
                arena,
                linear_map![
                    arena,
                    (
                        IrepId::File,
                        Irep::just_string_id(arena, format!("<builtin-library-{function_name}>")),
                    ),
                    (IrepId::Function, Irep::just_string_id(arena, function_name.to_string())),
                ],
            )
            .with_named_sub_option(IrepId::Line, line.map(|a| Irep::just_int_id(arena, a))),
            Location::Loc {
                file,
                function,
                start_line,
                start_col,
                end_line: _,
                end_col: _,
                pragmas,
            } => Irep::just_named_sub(
                arena,
                linear_map![
                    arena,
                    (IrepId::File, Irep::just_string_id(arena, file.to_string())),
                    (IrepId::Line, Irep::just_int_id(arena, *start_line)),
                ],
            )
            .with_named_sub_option(IrepId::Column, start_col.map(|a| Irep::just_int_id(arena, a)))
            .with_named_sub_option(
                IrepId::Function,
                function.map(|s| Irep::just_string_id(arena, s)),
            )
            .with_named_sub_option(
                IrepId::Pragma,
                Some(Irep::just_named_sub(
                    arena,
                    hash_collect_into(
                        pragmas.iter().map(|pragma| {
                            (
                                IrepId::from_string(*pragma),
                                Irep::just_id(arena, IrepId::EmptyString),
                            )
                        }),
                        arena,
                    ),
                )),
            ),
            Location::Property { file, function, line, col, property_class, comment, pragmas } => {
                Irep::just_named_sub(
                    arena,
                    hash_collect_into(
                        [
                            (IrepId::File, Irep::just_string_id(arena, file.to_string())),
                            (IrepId::Line, Irep::just_int_id(arena, *line)),
                        ],
                        arena,
                    ),
                )
                .with_named_sub_option(IrepId::Column, col.map(|a| Irep::just_int_id(arena, a)))
                .with_named_sub_option(
                    IrepId::Function,
                    function.map(|s| Irep::just_string_id(arena, s)),
                )
                .with_named_sub(IrepId::Comment, Irep::just_string_id(arena, comment.to_string()))
                .with_named_sub(
                    IrepId::PropertyClass,
                    Irep::just_string_id(arena, property_class.to_string()),
                )
                .with_named_sub_option(
                    IrepId::Pragma,
                    Some(Irep::just_named_sub(
                        arena,
                        hash_collect_into(
                            pragmas.iter().map(|pragma| {
                                (
                                    IrepId::from_string(*pragma),
                                    Irep::just_id(arena, IrepId::EmptyString),
                                )
                            }),
                            arena,
                        ),
                    )),
                )
            }
            Location::PropertyUnknownLocation { property_class, comment } => Irep::just_named_sub(
                arena,
                linear_map![
                    arena,
                    (IrepId::Comment, Irep::just_string_id(arena, comment.to_string())),
                    (
                        IrepId::PropertyClass,
                        Irep::just_string_id(arena, property_class.to_string())
                    )
                ],
            ),
        }
    }
}

impl ToIrep for Parameter {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        Irep {
            id: IrepId::Parameter,
            sub: vec_in![arena],
            named_sub: linear_map![arena, (IrepId::Type, self.typ().to_irep(arena, mm))],
        }
        .with_named_sub_option(
            IrepId::CIdentifier,
            self.identifier().map(|s| Irep::just_string_id(arena, s)),
        )
        .with_named_sub_option(
            IrepId::CBaseName,
            self.base_name().map(|s| Irep::just_string_id(arena, s)),
        )
    }
}

impl ToIrep for Stmt {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        self.body().to_irep(arena, mm).with_owned_location(*self.location(), mm)
    }
}

impl ToIrep for StmtBody {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        match self {
            StmtBody::Assign { lhs, rhs } => code_irep(
                arena,
                IrepId::Assign,
                vec_in![arena, lhs.to_irep(arena, mm), rhs.to_irep(arena, mm)],
            ),
            StmtBody::Assert { cond, .. } => {
                code_irep(arena, IrepId::Assert, vec_in![arena, cond.to_irep(arena, mm)])
            }
            StmtBody::Assume { cond } => {
                code_irep(arena, IrepId::Assume, vec_in![arena, cond.to_irep(arena, mm)])
            }
            StmtBody::AtomicBlock(stmts) => {
                let mut irep_stmts =
                    vec_in![arena, code_irep(arena, IrepId::AtomicBegin, vec_in![arena])];
                irep_stmts.extend(&mut stmts.iter().map(|x| x.to_irep(arena, mm)));
                irep_stmts.push(code_irep(arena, IrepId::AtomicEnd, vec_in![arena]));
                code_irep(arena, IrepId::Block, irep_stmts)
            }
            StmtBody::Block(stmts) => code_irep(
                arena,
                IrepId::Block,
                collect_into(stmts.iter().map(|x| x.to_irep(arena, mm)), arena),
            ),
            StmtBody::Break => code_irep(arena, IrepId::Break, vec_in![arena]),
            StmtBody::Continue => code_irep(arena, IrepId::Continue, vec_in![arena]),
            StmtBody::Dead(symbol) => {
                code_irep(arena, IrepId::Dead, vec_in![arena, symbol.to_irep(arena, mm)])
            }
            StmtBody::Decl { lhs, value } => {
                if value.is_some() {
                    code_irep(
                        arena,
                        IrepId::Decl,
                        vec_in![
                            arena,
                            lhs.to_irep(arena, mm),
                            value.as_ref().unwrap().to_irep(arena, mm)
                        ],
                    )
                } else {
                    code_irep(arena, IrepId::Decl, vec_in![arena, lhs.to_irep(arena, mm)])
                }
            }
            StmtBody::Deinit(place) => {
                // CBMC doesn't yet have a notion of poison (https://github.com/diffblue/cbmc/issues/7014)
                // So we translate identically to `nondet` here, but add a comment noting we wish it were poison
                // potentially for other backends to pick up and treat specially.
                code_irep(
                    arena,
                    IrepId::Assign,
                    vec_in![
                        arena,
                        place.to_irep(arena, mm),
                        place.typ().nondet().to_irep(arena, mm)
                    ],
                )
                .with_comment(arena, "deinit")
            }
            StmtBody::Expression(e) => {
                code_irep(arena, IrepId::Expression, vec_in![arena, e.to_irep(arena, mm)])
            }
            StmtBody::For { init, cond, update, body } => code_irep(
                arena,
                IrepId::For,
                vec_in![
                    arena,
                    init.to_irep(arena, mm),
                    cond.to_irep(arena, mm),
                    update.to_irep(arena, mm),
                    body.to_irep(arena, mm)
                ],
            ),
            StmtBody::FunctionCall { lhs, function, arguments } => code_irep(
                arena,
                IrepId::FunctionCall,
                vec_in![
                    arena,
                    lhs.as_ref().map_or(Irep::nil(arena), |x| x.to_irep(arena, mm)),
                    function.to_irep(arena, mm),
                    arguments_irep(arena, arguments.iter(), mm),
                ],
            ),
            StmtBody::Goto { dest, loop_invariants } => {
                let stmt_goto = code_irep(arena, IrepId::Goto, vec_in![arena]).with_named_sub(
                    IrepId::Destination,
                    Irep::just_string_id(arena, dest.to_string()),
                );
                if let Some(inv) = loop_invariants {
                    stmt_goto.with_named_sub(
                        IrepId::CSpecLoopInvariant,
                        inv.clone().and(Expr::bool_true()).to_irep(arena, mm),
                    )
                } else {
                    stmt_goto
                }
            }
            StmtBody::Ifthenelse { i, t, e } => code_irep(
                arena,
                IrepId::Ifthenelse,
                vec_in![
                    arena,
                    i.to_irep(arena, mm),
                    t.to_irep(arena, mm),
                    e.as_ref().map_or(Irep::nil(arena), |x| x.to_irep(arena, mm)),
                ],
            ),
            StmtBody::Label { label, body } => {
                code_irep(arena, IrepId::Label, vec_in![arena, body.to_irep(arena, mm)])
                    .with_named_sub(IrepId::Label, Irep::just_string_id(arena, label.to_string()))
            }
            StmtBody::Return(e) => code_irep(
                arena,
                IrepId::Return,
                vec_in![arena, e.as_ref().map_or(Irep::nil(arena), |x| x.to_irep(arena, mm))],
            ),
            StmtBody::Skip => code_irep(arena, IrepId::Skip, vec_in![arena]),
            StmtBody::Switch { control, cases, default } => {
                let mut switch_arms =
                    collect_into(cases.iter().map(|x| x.to_irep(arena, mm)), arena);
                if default.is_some() {
                    switch_arms.push(switch_default_irep(arena, default.as_ref().unwrap(), mm));
                }
                code_irep(
                    arena,
                    IrepId::Switch,
                    vec_in![
                        arena,
                        control.to_irep(arena, mm),
                        code_irep(arena, IrepId::Block, switch_arms)
                    ],
                )
            }
            StmtBody::While { cond, body } => code_irep(
                arena,
                IrepId::While,
                vec_in![arena, cond.to_irep(arena, mm), body.to_irep(arena, mm)],
            ),
        }
    }
}

impl ToIrep for SwitchCase {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        code_irep(
            arena,
            IrepId::SwitchCase,
            vec_in![arena, self.case().to_irep(arena, mm), self.body().to_irep(arena, mm)],
        )
        .with_owned_location(*self.body().location(), mm)
    }
}

impl ToIrep for Lambda {
    /// At the moment this function assumes that this lambda is used for a
    /// `modifies` contract. It should work for any other lambda body, but
    /// the parameter names use "modifies" in their generated names.
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        let (mut ops_ireps, mut types) = (vec_in![arena], vec_in![arena]);

        for (op, ty) in self.arguments.iter().enumerate().map(|(index, param)| {
            let ty_rep = param.typ().to_irep(arena, mm);
            (
                Irep::symbol(
                    arena,
                    param.identifier().unwrap_or_else(|| format!("_modifies_{index}").into()),
                )
                .with_named_sub(IrepId::Type, ty_rep.clone()),
                ty_rep,
            )
        }) {
            ops_ireps.push(op);
            types.push(ty);
        }

        let typ = Irep {
            id: IrepId::MathematicalFunction,
            sub: vec_in![arena, Irep::just_sub(types), self.body.typ().to_irep(arena, mm)],
            named_sub: ManuallyDrop::new(HashMap::new_in(arena)),
        };
        Irep {
            id: IrepId::Lambda,
            sub: vec_in![arena, Irep::tuple(ops_ireps), self.body.to_irep(arena, mm)],
            named_sub: hash_collect_into([(IrepId::Type, typ)], arena),
        }
    }
}

impl goto_program::Symbol {
    pub fn to_irep<'b>(&'b self, arena: &'b Bump, mm: &MachineModel) -> super::Symbol<'b> {
        let mut typ = self.typ.to_irep(arena, mm);
        if let Some(contract) = &self.contract {
            typ = typ.with_named_sub(
                IrepId::CSpecAssigns,
                Irep::just_sub(collect_into(
                    contract.assigns.iter().map(|req| req.to_irep(arena, mm)),
                    arena,
                )),
            );
        }
        if self.is_static_const {
            // Add a `const` to the type.
            typ = typ.with_named_sub(IrepId::CConstant, Irep::just_id(arena, IrepId::from_int(1)))
        }
        super::Symbol {
            typ,
            value: match &self.value {
                SymbolValues::Expr(e) => e.to_irep(arena, mm),
                SymbolValues::Stmt(s) => s.to_irep(arena, mm),
                SymbolValues::None => Irep::nil(arena),
            },
            location: self.location.to_irep(arena, mm),
            // Unique identifier, same as key in symbol table `foo::x`
            name: self.name,
            // Only used by verilog
            module: self.module.unwrap_or("".into()),
            // Local identifier `x`
            base_name: self.base_name.unwrap_or("".into()),
            // Almost always the same as `base_name`, but with name mangling can be relevant
            pretty_name: self.pretty_name.unwrap_or("".into()),
            // Currently set to C. Consider creating a "rust" mode and using it in cbmc
            // https://github.com/model-checking/kani/issues/1
            mode: self.mode.to_string().into(),

            // global properties
            is_type: self.is_type,
            is_macro: self.is_macro,
            is_exported: self.is_exported,
            is_input: self.is_input,
            is_output: self.is_output,
            is_state_var: self.is_state_var,
            is_property: self.is_property,

            // ansi-C properties
            is_static_lifetime: self.is_static_lifetime,
            is_thread_local: self.is_thread_local,
            is_lvalue: self.is_lvalue,
            is_file_local: self.is_file_local,
            is_extern: self.is_extern,
            is_volatile: self.is_volatile,
            is_parameter: self.is_parameter,
            is_auxiliary: self.is_auxiliary,
            is_weak: self.is_weak,
        }
    }
}

impl goto_program::SymbolTable {
    pub fn to_irep_in<'b>(&'b self, arena: &'b bumpalo::Bump) -> super::SymbolTable<'b> {
        let mm = self.machine_model();
        let mut st = super::SymbolTable::new_in(arena);
        for (_key, value) in self.iter() {
            st.insert(value.to_irep(arena, mm))
        }
        st
    }
}

impl ToIrep for Type {
    fn to_irep<'b>(&self, arena: &'b Bump, mm: &MachineModel) -> Irep<'b> {
        match self {
            Type::Array { typ, size } => {
                //CBMC expects the size to be a signed int constant.
                let size = Expr::int_constant(*size, Type::ssize_t());
                Irep {
                    id: IrepId::Array,
                    sub: vec_in![arena, typ.to_irep(arena, mm)],
                    named_sub: linear_map![arena, (IrepId::Size, size.to_irep(arena, mm))],
                }
            }
            //TODO make from_irep that matches this.
            Type::CBitField { typ, width } => Irep {
                id: IrepId::CBitField,
                sub: vec_in![arena, typ.to_irep(arena, mm)],
                named_sub: linear_map![arena, (IrepId::Width, Irep::just_int_id(arena, *width))],
            },
            Type::Bool => Irep::just_id(arena, IrepId::Bool),
            Type::CInteger(CIntType::Bool) => Irep {
                id: IrepId::CBool,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.bool_width))
                ],
            },
            Type::CInteger(CIntType::Char) => Irep {
                id: if mm.char_is_unsigned { IrepId::Unsignedbv } else { IrepId::Signedbv },
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.char_width),)
                ],
            },
            Type::CInteger(CIntType::Int) => Irep {
                id: IrepId::Signedbv,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.int_width),)
                ],
            },
            Type::CInteger(CIntType::LongInt) => Irep {
                id: IrepId::Signedbv,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.long_int_width),)
                ],
            },
            Type::CInteger(CIntType::SizeT) => Irep {
                id: IrepId::Unsignedbv,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.pointer_width),)
                ],
            },
            Type::CInteger(CIntType::SSizeT) => Irep {
                id: IrepId::Signedbv,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.pointer_width),)
                ],
            },
            Type::Code { parameters, return_type } => Irep {
                id: IrepId::Code,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (
                        IrepId::Parameters,
                        Irep::just_sub(collect_into(
                            parameters.iter().map(|x| x.to_irep(arena, mm)),
                            arena
                        )),
                    ),
                    (IrepId::ReturnType, return_type.to_irep(arena, mm)),
                ],
            },
            Type::Constructor => Irep::just_id(arena, IrepId::Constructor),
            Type::Double => Irep {
                id: IrepId::Floatbv,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::F, Irep::just_int_id(arena, 52)),
                    (IrepId::Width, Irep::just_int_id(arena, 64)),
                    (IrepId::CCType, Irep::just_id(arena, IrepId::Double)),
                ],
            },
            Type::Empty => Irep::just_id(arena, IrepId::Empty),
            // CMBC currently represents these as 0 length arrays.
            Type::FlexibleArray { typ } => {
                //CBMC expects the size to be a signed int constant.
                let size = Type::ssize_t().zero();
                Irep {
                    id: IrepId::Array,
                    sub: vec_in![arena, typ.to_irep(arena, mm)],
                    named_sub: linear_map![arena, (IrepId::Size, size.to_irep(arena, mm))],
                }
            }
            Type::Float => Irep {
                id: IrepId::Floatbv,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::F, Irep::just_int_id(arena, 23)),
                    (IrepId::Width, Irep::just_int_id(arena, 32)),
                    (IrepId::CCType, Irep::just_id(arena, IrepId::Float)),
                ],
            },
            Type::Float16 => Irep {
                id: IrepId::Floatbv,
                sub: vec_in![arena],
                // Fraction bits: 10
                // Exponent width bits: 5
                // Sign bit: 1
                named_sub: linear_map![
                    arena,
                    (IrepId::F, Irep::just_int_id(arena, 10)),
                    (IrepId::Width, Irep::just_int_id(arena, 16)),
                    (IrepId::CCType, Irep::just_id(arena, IrepId::Float16)),
                ],
            },
            Type::Float128 => Irep {
                id: IrepId::Floatbv,
                sub: vec_in![arena],
                // Fraction bits: 112
                // Exponent width bits: 15
                // Sign bit: 1
                named_sub: linear_map![
                    arena,
                    (IrepId::F, Irep::just_int_id(arena, 112)),
                    (IrepId::Width, Irep::just_int_id(arena, 128)),
                    (IrepId::CCType, Irep::just_id(arena, IrepId::Float128)),
                ],
            },
            Type::IncompleteStruct { tag } => Irep {
                id: IrepId::Struct,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Tag, Irep::just_string_id(arena, tag.to_string())),
                    (IrepId::Incomplete, Irep::one(arena)),
                ],
            },
            Type::IncompleteUnion { tag } => Irep {
                id: IrepId::Union,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Tag, Irep::just_string_id(arena, tag.to_string())),
                    (IrepId::Incomplete, Irep::one(arena)),
                ],
            },
            Type::InfiniteArray { typ } => {
                let infinity =
                    Irep::just_id(arena, IrepId::Infinity).with_owned_type(Type::ssize_t(), mm);
                Irep {
                    id: IrepId::Array,
                    sub: vec_in![arena, typ.to_irep(arena, mm)],
                    named_sub: linear_map![arena, (IrepId::Size, infinity)],
                }
            }
            Type::Integer => Irep::just_id(arena, IrepId::Integer),
            Type::Pointer { typ } => Irep {
                id: IrepId::Pointer,
                sub: vec_in![arena, typ.to_irep(arena, mm)],
                named_sub: linear_map![
                    arena,
                    (IrepId::Width, Irep::just_int_id(arena, mm.pointer_width),)
                ],
            },
            Type::Signedbv { width } => Irep {
                id: IrepId::Signedbv,
                sub: vec_in![arena],
                named_sub: linear_map![arena, (IrepId::Width, Irep::just_int_id(arena, *width))],
            },
            Type::Struct { tag, components } => Irep {
                id: IrepId::Struct,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Tag, Irep::just_string_id(arena, tag.to_string())),
                    (
                        IrepId::Components,
                        Irep::just_sub(collect_into(
                            components.iter().map(|x| x.to_irep(arena, mm)),
                            arena
                        )),
                    ),
                ],
            },
            Type::StructTag(name) => Irep {
                id: IrepId::StructTag,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Identifier, Irep::just_string_id(arena, name.to_string()),)
                ],
            },
            Type::TypeDef { name, typ } => typ
                .to_irep(arena, mm)
                .with_named_sub(IrepId::CTypedef, Irep::just_string_id(arena, name.to_string())),

            Type::Union { tag, components } => Irep {
                id: IrepId::Union,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Tag, Irep::just_string_id(arena, tag.to_string())),
                    (
                        IrepId::Components,
                        Irep::just_sub(collect_into(
                            components.iter().map(|x| x.to_irep(arena, mm)),
                            arena
                        )),
                    ),
                ],
            },
            Type::UnionTag(name) => Irep {
                id: IrepId::UnionTag,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (IrepId::Identifier, Irep::just_string_id(arena, name.to_string()),)
                ],
            },
            Type::Unsignedbv { width } => Irep {
                id: IrepId::Unsignedbv,
                sub: vec_in![arena],
                named_sub: linear_map![arena, (IrepId::Width, Irep::just_int_id(arena, *width))],
            },
            Type::VariadicCode { parameters, return_type } => Irep {
                id: IrepId::Code,
                sub: vec_in![arena],
                named_sub: linear_map![
                    arena,
                    (
                        IrepId::Parameters,
                        Irep::just_sub(collect_into(
                            parameters.iter().map(|x| x.to_irep(arena, mm)),
                            arena
                        ))
                        .with_named_sub(IrepId::Ellipsis, Irep::one(arena)),
                    ),
                    (IrepId::ReturnType, return_type.to_irep(arena, mm)),
                ],
            },
            Type::Vector { typ, size } => {
                let size = Expr::int_constant(*size, Type::ssize_t());
                Irep {
                    id: IrepId::Vector,
                    sub: vec_in![arena, typ.to_irep(arena, mm)],
                    named_sub: linear_map![arena, (IrepId::Size, size.to_irep(arena, mm))],
                }
            }
        }
    }
}
