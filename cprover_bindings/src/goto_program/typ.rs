// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use self::DatatypeComponent::*;
use self::Type::*;
use super::super::utils::{aggr_tag, max_int, min_int};
use super::super::MachineModel;
use super::{Expr, SymbolTable};
use crate::cbmc_string::InternedString;
use std::collections::BTreeMap;
use std::fmt::Debug;

///////////////////////////////////////////////////////////////////////////////////////////////
/// Datatypes
///////////////////////////////////////////////////////////////////////////////////////////////

/// Represents the different types that can be used in a goto-program.
/// The names are directly taken from the CBMC IrepIds.
/// In the examples below, `x` is used as a placeholder showing how the a variable of that
/// type would be declared. In general, these types map directly to C types; when they do not,
/// the comment notes this.
#[derive(PartialEq, Debug, Clone)]
pub enum Type {
    /// `typ x[size]`. E.g. `unsigned int x[3]`
    Array { typ: Box<Type>, size: u64 },
    /// CBMC specific. `__CPROVER_bool x`. A single bit boolean
    Bool,
    /// `typ x : width`. e.g. `unsigned int x: 3`.
    CBitField { typ: Box<Type>, width: u64 },
    /// Machine dependent integers: `bool`, `char`, `int`, `long int`, `size_t`, etc.
    CInteger(CIntType),
    /// `return_type x(parameters)`
    Code { parameters: Vec<Parameter>, return_type: Box<Type> },
    /// `__attribute__(constructor)`. Only valid as a function return type.
    /// <https://gcc.gnu.org/onlinedocs/gcc-4.7.0/gcc/Function-Attributes.html>
    Constructor,
    /// `double`
    Double,
    /// `void`
    Empty,
    /// `typ x[]`. Has a type, but no size. Only valid as the last element of a struct.
    FlexibleArray { typ: Box<Type> },
    /// `float`
    Float,
    /// `Half float`
    Float16,
    /// `float 128`
    Float128,
    /// `struct x {}`
    IncompleteStruct { tag: InternedString },
    /// `union x {}`
    IncompleteUnion { tag: InternedString },
    /// `integer`: A machine independent integer
    Integer,
    /// CBMC specific. `typ x[__CPROVER_infinity()]`
    InfiniteArray { typ: Box<Type> },
    /// `typ*`
    Pointer { typ: Box<Type> },
    /// `int<width>_t`. e.g. `int32_t`
    Signedbv { width: u64 },
    /// `struct tag {component1.typ component1.name; component2.typ component2.name ... }`
    Struct { tag: InternedString, components: Vec<DatatypeComponent> },
    /// CBMC specific. A reference into the symbol table, where the tag is the name of the symbol.
    StructTag(InternedString),
    /// Typedef construct. It has a name and a type.
    TypeDef { name: InternedString, typ: Box<Type> },
    /// `union tag {component1.typ component1.name; component2.typ component2.name ... }`
    Union { tag: InternedString, components: Vec<DatatypeComponent> },
    /// CBMC specific. A reference into the symbol table, where the tag is the name of the symbol.
    UnionTag(InternedString),
    /// `uint<width>_t`. e.g. `uint32_t`
    Unsignedbv { width: u64 },
    /// `return_type x(parameters, ...)`
    VariadicCode { parameters: Vec<Parameter>, return_type: Box<Type> },
    /// Packed SIMD vectors
    /// In CBMC/gcc, variables of this type are declared as:
    /// `typ __attribute__((vector_size (size * sizeof(typ)))) var;`
    Vector { typ: Box<Type>, size: u64 },
}

/// Machine dependent integers: `bool`, `char`, `int`, `size_t`, etc.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum CIntType {
    /// `bool`
    Bool,
    /// `char`
    Char,
    /// `int`
    Int,
    /// `long int`
    LongInt,
    /// `size_t`
    SizeT,
    /// `ssize_t`
    SSizeT,
}

/// The fields types of a struct or union
#[derive(PartialEq, Debug, Clone)]
pub enum DatatypeComponent {
    Field { name: InternedString, typ: Type },
    Padding { name: InternedString, bits: u64 },
}

/// The formal parameters of a function.
#[derive(Debug, Clone)]
pub struct Parameter {
    typ: Type,
    /// The unique identifier that refers to this symbol (qualified by function name, module, etc)
    identifier: Option<InternedString>,
    /// The local name the symbol has within the function
    base_name: Option<InternedString>,
}

///////////////////////////////////////////////////////////////////////////////////////////////
/// Implementations
///////////////////////////////////////////////////////////////////////////////////////////////

/// Getters
impl DatatypeComponent {
    pub fn field_typ(&self) -> Option<&Type> {
        match self {
            Field { typ, .. } => Some(typ),
            Padding { .. } => None,
        }
    }

    pub fn is_field(&self) -> bool {
        match self {
            Field { .. } => true,
            Padding { .. } => false,
        }
    }

    pub fn is_padding(&self) -> bool {
        match self {
            Field { .. } => false,
            Padding { .. } => true,
        }
    }

    pub fn name(&self) -> InternedString {
        match self {
            Field { name, .. } | Padding { name, .. } => *name,
        }
    }

    pub fn sizeof_in_bits(&self, st: &SymbolTable) -> u64 {
        match self {
            Field { typ, .. } => typ.sizeof_in_bits(st),
            Padding { bits, .. } => *bits,
        }
    }

    pub fn typ(&self) -> Type {
        match self {
            Field { typ, .. } => typ.clone(),
            Padding { bits, .. } => Type::unsigned_int(*bits),
        }
    }
}

/// Constructors
impl DatatypeComponent {
    fn typecheck_datatype_field(typ: &Type) -> bool {
        match typ.unwrap_typedef() {
            Array { .. }
            | Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | FlexibleArray { .. }
            | Float
            | Float16
            | Float128
            | Integer
            | Pointer { .. }
            | Signedbv { .. }
            | Struct { .. }
            | StructTag(_)
            | Union { .. }
            | UnionTag(_)
            | Unsignedbv { .. }
            | Vector { .. } => true,

            Code { .. }
            | Constructor
            | Empty
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | VariadicCode { .. } => false,

            TypeDef { .. } => unreachable!("typedefs should have been unwrapped"),
        }
    }

    pub fn field<T: Into<InternedString>>(name: T, typ: Type) -> Self {
        let name = name.into();
        assert!(
            Self::typecheck_datatype_field(&typ),
            "Illegal field.\n\tName: {name}\n\tType: {typ:?}"
        );
        Field { name, typ }
    }

    pub fn padding<T: Into<InternedString>>(name: T, bits: u64) -> Self {
        let name = name.into();
        Padding { name, bits }
    }
}

/// Implement partial equal for Parameter.
/// Unlike the other cases, where we can just derive Eq, Parameters are equal regardless of the names given to them.
/// So we need to explicitly write the implementation to reflect that.
impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        self.typ == other.typ
    }
}

/// Getters
impl Parameter {
    pub fn base_name(&self) -> Option<InternedString> {
        self.base_name
    }

    pub fn identifier(&self) -> Option<InternedString> {
        self.identifier
    }

    pub fn typ(&self) -> &Type {
        &self.typ
    }
}

/// Constructor
impl Parameter {
    pub fn new<S: Into<InternedString>>(
        base_name: Option<S>,
        identifier: Option<S>,
        typ: Type,
    ) -> Self {
        Self { base_name: base_name.map(Into::into), identifier: identifier.map(Into::into), typ }
    }
}

impl CIntType {
    pub fn sizeof_in_bits(&self, st: &SymbolTable) -> u64 {
        match self {
            CIntType::Bool => st.machine_model().bool_width,
            CIntType::Char => st.machine_model().char_width,
            CIntType::Int => st.machine_model().int_width,
            CIntType::LongInt => st.machine_model().long_int_width,
            CIntType::SizeT => st.machine_model().pointer_width,
            CIntType::SSizeT => st.machine_model().pointer_width,
        }
    }
}

/// Getters
impl Type {
    /// Return the StructTag or UnionTag naming the struct or union type.
    pub fn aggr_tag(&self) -> Option<Type> {
        let concrete = self.unwrap_typedef();
        match concrete {
            IncompleteStruct { tag } | Struct { tag, .. } => Some(Type::struct_tag(*tag)),
            IncompleteUnion { tag } | Union { tag, .. } => Some(Type::union_tag(*tag)),
            StructTag(_) | UnionTag(_) => Some(self.clone()),
            _ => None,
        }
    }

    /// The base type of this type, if one exists.
    /// `typ*` | `typ x[width]` | `typ x : width`  -> `typ`,
    pub fn base_type(&self) -> Option<&Type> {
        let concrete = self.unwrap_typedef();
        match concrete {
            Array { typ, .. }
            | CBitField { typ, .. }
            | FlexibleArray { typ }
            | Pointer { typ }
            | Vector { typ, .. } => Some(typ),
            _ => None,
        }
    }

    pub fn components(&self) -> Option<&Vec<DatatypeComponent>> {
        let concrete = self.unwrap_typedef();
        match concrete {
            Struct { components, .. } | Union { components, .. } => Some(components),
            _ => None,
        }
    }

    /// The bitwidth of the integer type or pointer on the machine m.
    /// If the type doesn't have a width, return None.
    // TODO: This is only needed when calling CBMC intrinsics.
    //       1) Make CBMC have width independent intrinstics
    //    or 2) Move the use of the machine model to irep generation time
    pub fn native_width(&self, mm: &MachineModel) -> Option<u64> {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(CIntType::SizeT) | CInteger(CIntType::SSizeT) | Pointer { .. } => {
                Some(mm.pointer_width)
            }
            CInteger(CIntType::Bool) => Some(mm.bool_width),
            CInteger(CIntType::Char) => Some(mm.char_width),
            CInteger(CIntType::Int) => Some(mm.int_width),
            CInteger(CIntType::LongInt) => Some(mm.long_int_width),
            Signedbv { width } | Unsignedbv { width } => Some(*width),
            _ => None,
        }
    }

    pub fn parameters(&self) -> Option<&Vec<Parameter>> {
        match self {
            Code { parameters, .. } | VariadicCode { parameters, .. } => Some(parameters),
            _ => None,
        }
    }

    pub fn return_type(&self) -> Option<&Type> {
        match self {
            Code { return_type, .. } | VariadicCode { return_type, .. } => Some(return_type),
            _ => None,
        }
    }

    /// Returns the length (number of elements) in an array or vector type
    pub fn len(&self) -> Option<u64> {
        match self {
            Array { size, .. } => Some(*size),
            Vector { size, .. } => Some(*size),
            _ => None,
        }
    }

    pub fn sizeof(&self, st: &SymbolTable) -> u64 {
        let bits = self.sizeof_in_bits(st);
        let char_width = st.machine_model().char_width;
        assert_eq!(0, bits % char_width);
        bits / char_width
    }

    pub fn sizeof_expr(&self, st: &SymbolTable) -> Expr {
        Expr::int_constant(self.sizeof(st), Type::size_t())
    }

    pub fn sizeof_in_bits(&self, st: &SymbolTable) -> u64 {
        // TODO: sizeof involving bitfields is tricky, since bitfields in a struct can be merged.
        // I need to understand exactly when this can happen, and whether it depends on the
        // base type.
        let concrete = self.unwrap_typedef();
        match concrete {
            Array { typ, size } => typ.sizeof_in_bits(st) * size,
            Bool => unreachable!("Bool doesn't have a sizeof"),
            CBitField { .. } => todo!("implement sizeof for bitfields"),
            CInteger(t) => t.sizeof_in_bits(st),

            // We generate Code to put a reference to a Rust FnDef into a vtable; the definition
            // itself has no size (data is empty, and the vtable itself contains fn pointers for
            // Fn::call, etc).
            //
            // See Rust's implementation of layout_of, where FnDef is treated as a univariant
            // type with no fields (and thus a size of 0 in the layout):
            //     FnDef case in layout_raw_uncached, compiler/rustc_middle/src/ty/layout.rs
            Code { .. } => 0,
            Constructor => unreachable!("Constructor doesn't have a sizeof"),
            Double => st.machine_model().double_width,
            Empty => 0,
            FlexibleArray { .. } => 0,
            Float16 => 16,
            Float128 => 128,
            Float => st.machine_model().float_width,
            IncompleteStruct { .. } => unreachable!("IncompleteStruct doesn't have a sizeof"),
            IncompleteUnion { .. } => unreachable!("IncompleteUnion doesn't have a sizeof"),
            InfiniteArray { .. } => unreachable!("InfiniteArray doesn't have a sizeof"),
            Integer => unreachable!("Integer doesn't have a sizeof"),
            Pointer { .. } => st.machine_model().pointer_width,
            Signedbv { width } => *width,
            Struct { components, .. } => {
                components.iter().map(|x| x.typ().sizeof_in_bits(st)).sum()
            }
            StructTag(tag) => st.lookup(*tag).unwrap().typ.sizeof_in_bits(st),
            TypeDef { .. } => unreachable!("Expected concrete type."),
            Union { components, .. } => {
                components.iter().map(|x| x.typ().sizeof_in_bits(st)).max().unwrap_or(0)
            }
            UnionTag(tag) => st.lookup(*tag).unwrap().typ.sizeof_in_bits(st),
            Unsignedbv { width } => *width,
            // It's possible this should also have size 0, like Code, but we have not been
            // able to generate a unit test, so leaving it unreachable for now.
            VariadicCode { .. } => unreachable!("VariadicCode doesn't have a sizeof"),
            Vector { typ, size } => typ.sizeof_in_bits(st) * size,
        }
    }

    /// Get the tag of a struct or union.
    pub fn tag(&self) -> Option<InternedString> {
        match self {
            IncompleteStruct { tag }
            | IncompleteUnion { tag }
            | Struct { tag, .. }
            | StructTag(tag)
            | Union { tag, .. }
            | UnionTag(tag)
            | TypeDef { name: tag, .. } => Some(*tag),
            _ => None,
        }
    }

    /// Given a `struct foo` or `union foo`, returns `Some("tag-foo")`.
    /// Otherwise, returns `None`.
    pub fn type_name(&self) -> Option<InternedString> {
        match self {
            IncompleteStruct { tag }
            | Struct { tag, .. }
            | IncompleteUnion { tag }
            | Union { tag, .. }
            | TypeDef { name: tag, .. } => Some(aggr_tag(*tag)),
            StructTag(tag) | UnionTag(tag) => Some(*tag),
            _ => None,
        }
    }

    /// the width of an integer type
    pub fn width(&self) -> Option<u64> {
        let concrete = self.unwrap_typedef();
        match concrete {
            CBitField { width, .. } | Signedbv { width } | Unsignedbv { width } => Some(*width),
            _ => None,
        }
    }
}

/// Predicates
impl Type {
    pub fn is_array(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Array { .. } => true,
            _ => false,
        }
    }

    pub fn is_array_like(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Array { .. } | FlexibleArray { .. } | Vector { .. } => true,
            _ => false,
        }
    }

    pub fn is_bitfield(&self) -> bool {
        let concrete = self.unwrap_typedef();
        matches!(concrete, CBitField { .. })
    }

    pub fn is_bool(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Bool => true,
            _ => false,
        }
    }

    pub fn is_c_bool(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Type::CInteger(CIntType::Bool) => true,
            _ => false,
        }
    }

    pub fn is_long_int(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Type::CInteger(CIntType::LongInt) => true,
            _ => false,
        }
    }

    pub fn is_c_size_t(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Type::CInteger(CIntType::SizeT) => true,
            _ => false,
        }
    }

    pub fn is_c_ssize_t(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Type::CInteger(CIntType::SSizeT) => true,
            _ => false,
        }
    }

    pub fn is_code(&self) -> bool {
        match self {
            Code { .. } => true,
            _ => false,
        }
    }

    pub fn is_double(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Double => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Empty => true,
            _ => false,
        }
    }

    /// Whether self and other have the same concrete type on the given machine
    /// (specifically whether they have the same bit-size and signed-ness)
    pub fn is_equal_on_machine(&self, other: &Self, mm: &MachineModel) -> bool {
        let concrete_self = self.unwrap_typedef();
        let concrete_other = other.unwrap_typedef();
        if concrete_self == concrete_other {
            true
        } else {
            concrete_self.native_width(mm) == concrete_other.native_width(mm)
                && concrete_self.is_signed(mm) == concrete_other.is_signed(mm)
        }
    }

    pub fn is_flexible_array(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            FlexibleArray { .. } => true,
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Float => true,
            _ => false,
        }
    }

    pub fn is_floating_point(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Double | Float => true,
            _ => false,
        }
    }

    pub fn is_c_integer(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(_) => true,
            _ => false,
        }
    }

    /// Whether the current type is an integer with finite width
    pub fn is_integer(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(_) | Integer | Signedbv { .. } | Unsignedbv { .. } => true,
            _ => false,
        }
    }

    /// Whether the type can be an lvalue.
    ///
    /// Note that this is different than a modifiable lvalue type which does not include arrays.
    pub fn can_be_lvalue(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Array { .. }
            | Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | Float
            | Float16
            | Float128
            | Integer
            | Pointer { .. }
            | Signedbv { .. }
            | Struct { .. }
            | StructTag(_)
            | Union { .. }
            | UnionTag(_)
            | Unsignedbv { .. }
            | Vector { .. } => true,

            Code { .. }
            | Constructor
            | Empty
            | FlexibleArray { .. }
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | VariadicCode { .. } => false,

            TypeDef { .. } => unreachable!("Expected concrete type only."),
        }
    }

    /// Is the current type either an integer or a floating point?
    pub fn is_numeric(&self) -> bool {
        self.is_floating_point() || self.is_integer()
    }

    pub fn is_pointer(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Pointer { .. } => true,
            _ => false,
        }
    }

    /// Is this a size_t, ssize_t, or pointer?
    pub fn is_pointer_width(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Pointer { .. } | CInteger(CIntType::SizeT) | CInteger(CIntType::SSizeT) => true,
            _ => false,
        }
    }

    pub fn is_scalar(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            // Base types
            Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | Empty
            | Float
            | Float16
            | Float128
            | Integer
            | Pointer { .. }
            | Signedbv { .. }
            | Unsignedbv { .. } => true,

            Array { .. }
            | Code { .. }
            | Constructor
            | FlexibleArray { .. }
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | Struct { .. }
            | StructTag(_)
            | Union { .. }
            | UnionTag(_)
            | VariadicCode { .. }
            | Vector { .. } => false,

            TypeDef { .. } => unreachable!("Expected concrete type only."),
        }
    }

    /// Is this a signed integer
    pub fn is_signed(&self, mm: &MachineModel) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(CIntType::Int)
            | CInteger(CIntType::LongInt)
            | CInteger(CIntType::SSizeT)
            | Signedbv { .. } => true,
            CInteger(CIntType::Char) => !mm.char_is_unsigned,
            _ => false,
        }
    }

    /// This is a struct (and not an incomplete struct or struct tag)
    pub fn is_struct(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Struct { .. } => true,
            _ => false,
        }
    }

    pub fn is_struct_like(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            IncompleteStruct { .. } | Struct { .. } | StructTag(_) => true,
            _ => false,
        }
    }

    /// This is a struct tag
    pub fn is_struct_tag(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            StructTag(_) => true,
            _ => false,
        }
    }

    /// This is a union (and not an incomplete union or union tag)
    pub fn is_union(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Union { .. } => true,
            _ => false,
        }
    }

    pub fn is_union_like(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            IncompleteUnion { .. } | Union { .. } | UnionTag(_) => true,
            _ => false,
        }
    }

    /// This is a union tag
    pub fn is_union_tag(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            UnionTag(_) => true,
            _ => false,
        }
    }

    /// Is this an unsigned integer
    pub fn is_unsigned(&self, mm: &MachineModel) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(CIntType::Bool) | CInteger(CIntType::SizeT) | Unsignedbv { .. } => true,
            CInteger(CIntType::Char) => mm.char_is_unsigned,
            _ => false,
        }
    }

    pub fn is_variadic_code(&self) -> bool {
        matches!(self, VariadicCode { .. })
    }

    pub fn is_vector(&self) -> bool {
        let concrete = self.unwrap_typedef();
        match concrete {
            Vector { .. } => true,
            _ => false,
        }
    }

    pub fn is_typedef(&self) -> bool {
        matches!(self, TypeDef { .. })
    }

    /// This function will unwrap any type definitions into its concrete type.
    /// This will traverse chained typedefs until it finds the first concrete type.
    /// If the type is not a typedef, this will return the current type.
    ///
    /// post-condition: matches!(unwrap_typedef(typ), TypeDef{..}) is false.
    pub fn unwrap_typedef(&self) -> &Type {
        let mut final_typ = self;
        while let TypeDef { typ, .. } = &final_typ {
            final_typ = typ;
        }
        final_typ
    }

    /// A transparent type wraps a base type inside a struct, and has the same in-memory layout.
    /// For example,
    /// ```rust
    ///     struct TransparentWrapper { int x };
    /// ```
    /// We define it recursively as a type which has exactly one field, which it itself either
    /// a transparent type, or is a scalar type.
    pub fn is_transparent_type(&self, st: &SymbolTable) -> bool {
        self.unwrap_transparent_type(st).is_some()
    }

    /// If a type is transparent type (see comment on `Type::is_transparent_type()`),
    /// extract the type it wraps.
    pub fn unwrap_transparent_type(&self, st: &SymbolTable) -> Option<Type> {
        fn recurse(t: &Type, st: &SymbolTable) -> Option<Type> {
            // If the type has components, i.e. is either a union or a struct, recurse into them
            if t.is_struct_like() || t.is_union_like() {
                let components = t.get_non_empty_components(st).unwrap();
                if components.len() == 1 {
                    match &components[0] {
                        Padding { .. } => None,
                        Field { typ, .. } => recurse(typ, st),
                    }
                } else {
                    None
                }
            } else if t.is_scalar() {
                Some(t.clone())
            } else {
                None
            }
        }
        recurse(self.unwrap_typedef(), st)
    }

    /// Get the fields (including padding) in self.
    /// For StructTag or UnionTag, lookup the definition in the symbol table.
    pub fn lookup_components<'a>(&self, st: &'a SymbolTable) -> Option<&'a Vec<DatatypeComponent>> {
        self.type_name().and_then(|aggr_name| st.lookup(aggr_name)).and_then(|x| x.typ.components())
    }

    /// If typ.field_name exists in the symbol table, return Some(field),
    /// otherwise, return none.
    pub fn lookup_field<'a, T: Into<InternedString>>(
        &self,
        field_name: T,
        st: &'a SymbolTable,
    ) -> Option<&'a DatatypeComponent> {
        let field_name = field_name.into();
        self.lookup_components(st)
            .and_then(|fields| fields.iter().find(|&field| field.name() == field_name))
    }

    /// If typ.field_name exists in the symbol table, return Some(field.typ),
    /// otherwise, return none.
    pub fn lookup_field_type<T: Into<InternedString>>(
        &self,
        field_name: T,
        st: &SymbolTable,
    ) -> Option<Type> {
        self.lookup_field(field_name, st).map(|f| f.typ())
    }

    /// Get the non-zero-sized fields (including padding) in self
    pub fn get_non_empty_components<'a>(
        &self,
        st: &'a SymbolTable,
    ) -> Option<Vec<&'a DatatypeComponent>> {
        self.lookup_components(st)
            .map(|components| components.iter().filter(|x| x.sizeof_in_bits(st) != 0).collect())
    }

    /// Calculates an under-approximation of whether two types are structurally equivalent.
    ///
    /// Two types are structurally equivalent if one can be cast into the other without changing
    /// any bytes.  For e.g.,
    /// ```
    /// struct foo {
    ///     char x;
    ///     int y;
    /// }
    /// ```
    /// is structurally equivalent to
    /// ```
    /// struct i {
    ///     int z;
    /// }
    ///
    /// struct bar {
    ///     char a;
    ///     struct i b;
    /// }
    /// ```
    /// But, `struct foo` is not structurally equivalent to:
    /// ```
    /// __attribute__((packed))
    /// struct baz {
    ///     char x;
    ///     int y;
    /// }
    /// ```
    /// Since they have different padding.
    /// <https://github.com/diffblue/cbmc/blob/develop/src/solvers/lowering/byte_operators.cpp#L1093..L1136>
    pub fn is_structurally_equivalent_to(&self, other: &Type, st: &SymbolTable) -> bool {
        let concrete_other = other.unwrap_typedef();
        let concrete_self = self.unwrap_typedef();
        if concrete_self.sizeof_in_bits(st) != concrete_other.sizeof_in_bits(st) {
            false
        } else if concrete_self.is_scalar() && concrete_other.is_scalar() {
            concrete_self == concrete_other
        } else if concrete_self.is_struct_like() && concrete_other.is_scalar() {
            concrete_self
                .unwrap_transparent_type(st)
                .map_or(false, |wrapped| wrapped == *concrete_other)
        } else if concrete_self.is_scalar() && concrete_other.is_struct_like() {
            concrete_other
                .unwrap_transparent_type(st)
                .map_or(false, |wrapped| wrapped == *concrete_self)
        } else if concrete_self.is_struct_like() && concrete_other.is_struct_like() {
            let self_components = concrete_self.get_non_empty_components(st).unwrap();
            let other_components = concrete_other.get_non_empty_components(st).unwrap();
            if self_components.len() == other_components.len() {
                self_components.iter().zip(other_components.iter()).all(|(a, b)| {
                    (a.is_padding()
                        && b.is_padding()
                        && a.sizeof_in_bits(st) == b.sizeof_in_bits(st))
                        || (a.is_field()
                            && b.is_field()
                            && a.typ().is_structurally_equivalent_to(&b.typ(), st))
                })
            } else {
                false
            }
        } else {
            // TODO: Figure out under which cases unions can work here
            false
        }
    }

    /// This is a struct or union that completes an incomplete struct or union.
    pub fn completes(&self, old: &Type) -> bool {
        match (old, self) {
            (IncompleteStruct { tag: old_tag }, Struct { tag: new_tag, .. })
            | (IncompleteUnion { tag: old_tag }, Union { tag: new_tag, .. }) => old_tag == new_tag,
            _ => false,
        }
    }
}

/// Constructors
impl Type {
    fn typecheck_array_elem(&self) -> bool {
        match self.unwrap_typedef() {
            Array { .. }
            | Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | Float
            | Float16
            | Float128
            | Integer
            | Pointer { .. }
            | Signedbv { .. }
            | Struct { .. }
            | StructTag(_)
            | Union { .. }
            | UnionTag(_)
            | Unsignedbv { .. }
            | Vector { .. } => true,

            Code { .. }
            | Constructor
            | Empty
            | FlexibleArray { .. }
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | VariadicCode { .. } => false,

            TypeDef { .. } => unreachable!("typedefs should have been unwrapped"),
        }
    }

    /// elem_t\[size\]
    pub fn array_of<T>(self, size: T) -> Self
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        assert!(self.typecheck_array_elem(), "Can't make array of type {self:?}");
        let size: u64 = size.try_into().unwrap();
        Array { typ: Box::new(self), size }
    }

    pub fn as_bitfield(self, width: u64) -> Self {
        assert!(width > 0);
        assert!(self.is_integer());
        assert!(self.width().unwrap() >= width);
        CBitField { width, typ: Box::new(self) }
    }

    /// A formal function parameter.
    /// identifier: The unique identifier that refers to this parameter `foo12_bar17_x@1`
    /// base_name: the local name of the parameter within the function `x`
    /// typ: The type of the parameter
    pub fn as_parameter(
        self,
        identifier: Option<InternedString>,
        base_name: Option<InternedString>,
    ) -> Parameter {
        assert!(self.can_be_lvalue(), "Expected lvalue from {self:?} {identifier:?} {base_name:?}");
        Parameter { identifier, base_name, typ: self }
    }

    pub fn bool() -> Self {
        Bool
    }

    pub fn c_bool() -> Self {
        CInteger(CIntType::Bool)
    }

    pub fn c_char() -> Self {
        CInteger(CIntType::Char)
    }

    pub fn c_int() -> Self {
        CInteger(CIntType::Int)
    }

    pub fn c_long_int() -> Self {
        CInteger(CIntType::LongInt)
    }

    pub fn c_size_t() -> Self {
        CInteger(CIntType::SizeT)
    }

    pub fn c_ssize_t() -> Self {
        CInteger(CIntType::SSizeT)
    }

    /// corresponds to \[code_typet\] in CBMC, representing a function type
    ///    ret (params ..)
    pub fn code(parameters: Vec<Parameter>, return_type: Type) -> Self {
        Code { parameters, return_type: Box::new(return_type) }
    }

    /// CBMC, like c, allows function types to have unnamed formal paramaters
    /// `int foo(int, char, double)`
    pub fn code_with_unnamed_parameters(param_types: Vec<Type>, return_type: Type) -> Self {
        let parameters = param_types.into_iter().map(|t| t.as_parameter(None, None)).collect();
        Type::code(parameters, return_type)
    }

    pub fn constructor() -> Self {
        Constructor
    }

    pub fn double() -> Self {
        Double
    }

    /// The void type
    pub fn empty() -> Self {
        Empty
    }

    /// Empty struct.
    /// struct name {};
    pub fn empty_struct<T: Into<InternedString>>(tag: T) -> Self {
        Struct { tag: tag.into(), components: vec![] }
    }

    /// Empty union.
    /// union name {};
    pub fn empty_union<T: Into<InternedString>>(tag: T) -> Self {
        Union { tag: tag.into(), components: vec![] }
    }

    pub fn flexible_array_of(self) -> Self {
        FlexibleArray { typ: Box::new(self) }
    }

    pub fn float16() -> Self {
        Float16
    }

    pub fn float128() -> Self {
        Float128
    }

    pub fn float() -> Self {
        Float
    }

    /// A forward declared struct.
    /// struct foo;
    pub fn incomplete_struct<T: Into<InternedString>>(tag: T) -> Self {
        let tag = tag.into();
        IncompleteStruct { tag }
    }

    /// A forward declared union.
    /// union foo;
    pub fn incomplete_union<T: Into<InternedString>>(tag: T) -> Self {
        let tag = tag.into();
        IncompleteUnion { tag }
    }

    pub fn infinite_array_of(self) -> Self {
        assert!(self.typecheck_array_elem(), "Can't make infinite array of type {self:?}");
        InfiniteArray { typ: Box::new(self) }
    }

    pub fn integer() -> Self {
        Integer
    }

    /// self *
    pub fn to_pointer(self) -> Self {
        Pointer { typ: Box::new(self) }
    }

    /// Convert type to its signed counterpart if possible.
    /// For types that are already signed, this will return self.
    /// Note: This will expand any typedef.
    pub fn to_signed(&self) -> Option<Self> {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(CIntType::SizeT) => Some(CInteger(CIntType::SSizeT)),
            Unsignedbv { ref width } => Some(Signedbv { width: *width }),
            CInteger(CIntType::SSizeT) | Signedbv { .. } => Some(self.clone()),
            _ => None,
        }
    }

    /// Convert type to its unsigned counterpart if possible.
    /// For types that are already unsigned, this will return self.
    /// Note: This will expand any typedef.
    pub fn to_unsigned(&self) -> Option<Self> {
        let concrete = self.unwrap_typedef();
        match concrete {
            CInteger(CIntType::SSizeT) => Some(CInteger(CIntType::SizeT)),
            Signedbv { ref width } => Some(Unsignedbv { width: *width }),
            CInteger(CIntType::SizeT) | Unsignedbv { .. } => Some(self.clone()),
            _ => None,
        }
    }

    pub fn signed_int<T>(w: T) -> Self
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let width: u64 = w.try_into().unwrap();
        assert!(width > 0);
        Signedbv { width }
    }

    pub fn size_t() -> Self {
        CInteger(CIntType::SizeT)
    }

    pub fn ssize_t() -> Self {
        CInteger(CIntType::SSizeT)
    }
    /// struct name
    pub fn struct_tag<T: Into<InternedString>>(name: T) -> Self {
        StructTag(aggr_tag(name.into()))
    }

    /// struct name, but don't add a tag- prefix
    pub fn struct_tag_raw(name: InternedString) -> Self {
        StructTag(name)
    }

    fn components_are_unique(components: &[DatatypeComponent]) -> bool {
        let mut names: Vec<_> = components.iter().map(|x| x.name()).collect();
        names.sort();
        names.dedup();
        names.len() == components.len()
    }

    fn components_are_not_flexible_array(components: &[DatatypeComponent]) -> bool {
        components.iter().all(|x| !x.typ().is_flexible_array())
    }

    /// A struct can only contain a flexible array as its last element
    /// Check this
    fn components_in_valid_order_for_struct(components: &[DatatypeComponent]) -> bool {
        if let Some((_, cs)) = components.split_last() {
            Type::components_are_not_flexible_array(cs)
        } else {
            true
        }
    }

    /// struct name {
    ///     f1.typ f1.data; ...
    /// }
    pub fn struct_type<T: Into<InternedString>>(
        tag: T,
        components: Vec<DatatypeComponent>,
    ) -> Self {
        assert!(
            Type::components_are_unique(&components),
            "Components contain duplicates: {components:?}"
        );
        assert!(
            Type::components_in_valid_order_for_struct(&components),
            "Components are not in valid order for struct: {components:?}"
        );

        let tag = tag.into();
        Struct { tag, components }
    }

    /// self *
    pub fn to_typedef<T: Into<InternedString>>(self, name: T) -> Self {
        TypeDef { typ: Box::new(self), name: name.into() }
    }

    /// union name
    pub fn union_tag<T: Into<InternedString>>(name: T) -> Self {
        UnionTag(aggr_tag(name.into()))
    }

    /// union name, but don't add a tag- prefix
    pub fn union_tag_raw(name: InternedString) -> Self {
        UnionTag(name)
    }

    /// union name {
    ///     f1.typ f1.data; ...
    /// }
    pub fn union_type<T: Into<InternedString>>(tag: T, components: Vec<DatatypeComponent>) -> Self {
        let tag = tag.into();
        assert!(
            Type::components_are_unique(&components),
            "Components contain duplicates: {components:?}"
        );
        assert!(
            Type::components_are_not_flexible_array(&components),
            "Unions cannot contain flexible arrays: {components:?}"
        );
        Union { tag, components }
    }

    pub fn unsigned_int<T>(w: T) -> Self
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let width: u64 = w.try_into().unwrap();
        assert!(width > 0);
        Unsignedbv { width }
    }

    /// corresponds to \[code_typet\] in CBMC, representing a function type
    ///    ret (params, ... )
    pub fn variadic_code(parameters: Vec<Parameter>, return_type: Type) -> Self {
        VariadicCode { parameters, return_type: Box::new(return_type) }
    }

    /// CBMC, like c, allows function types to have unnamed formal paramaters
    /// `int foo(int, char, double, ...)`
    pub fn variadic_code_with_unnamed_parameters(
        param_types: Vec<Type>,
        return_type: Type,
    ) -> Self {
        let parameters = param_types
            .into_iter()
            .map(|t| Parameter { identifier: None, base_name: None, typ: t })
            .collect();
        Type::variadic_code(parameters, return_type)
    }

    // `size` is the number of elements (e.g., a SIMD vector of 4 integers)
    pub fn vector(typ: Type, size: u64) -> Self {
        assert!(typ.is_numeric());
        Type::Vector { typ: Box::new(typ), size }
    }

    /// `void *`
    pub fn void_pointer() -> Self {
        Type::empty().to_pointer()
    }
}

/// Constants from Types, for use in Expr contexts
impl Type {
    pub fn max_int_expr(&self, mm: &MachineModel) -> Expr {
        assert!(self.is_integer());
        let width = self.native_width(mm).unwrap();
        let signed = self.is_signed(mm);
        Expr::int_constant(max_int(width, signed), self.clone())
    }

    pub fn min_int_expr(&self, mm: &MachineModel) -> Expr {
        assert!(self.is_integer());
        let width = self.native_width(mm).unwrap();
        let signed = self.is_signed(mm);
        Expr::int_constant(min_int(width, signed), self.clone())
    }

    /// an expression of nondeterministic value of type self
    pub fn nondet(&self) -> Expr {
        Expr::nondet(self.clone())
    }

    /// null pointer of self type
    /// (t)NULL
    pub fn null(&self) -> Expr {
        assert!(self.is_pointer());
        self.zero()
    }

    pub fn one(&self) -> Expr {
        if self.is_integer() || self.is_bitfield() {
            Expr::int_constant(1, self.clone())
        } else if self.is_c_bool() {
            Expr::c_true()
        } else if self.is_float() {
            Expr::float_constant(1.0)
        } else if self.is_double() {
            Expr::double_constant(1.0)
        } else {
            unreachable!("Can't convert {:?} to a one value", self)
        }
    }

    pub fn zero(&self) -> Expr {
        if self.is_integer() || self.is_bitfield() {
            Expr::int_constant(0, self.clone())
        } else if self.is_bool() {
            Expr::bool_false()
        } else if self.is_c_bool() {
            Expr::c_false()
        } else if self.is_float() {
            Expr::float_constant(0.0)
        } else if self.is_double() {
            Expr::double_constant(0.0)
        } else if self.is_pointer() {
            Expr::pointer_constant(0, self.clone())
        } else {
            unreachable!("Can't convert {:?} to a zero value", self);
        }
    }

    pub fn zero_initializer(&self, st: &SymbolTable) -> Expr {
        let concrete = self.unwrap_typedef();
        match concrete {
            // Base case
            Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | Float
            | Float16
            | Float128
            | Integer
            | Pointer { .. }
            | Signedbv { .. }
            | Unsignedbv { .. } => self.zero(),

            // Recursive cases
            Array { typ, size } => typ.zero_initializer(st).array_constant(*size),
            InfiniteArray { typ } => typ.zero_initializer(st).infinite_array_constant(),
            Struct { components, .. } => {
                let values: Vec<Expr> =
                    components.iter().map(|c| c.typ().zero_initializer(st)).collect();
                Expr::struct_expr_from_padded_values(self.clone(), values, st)
            }
            StructTag(tag) => st.lookup(*tag).unwrap().typ.zero_initializer(st),
            TypeDef { .. } => unreachable!("Should have been normalized away"),
            Union { components, .. } => {
                if components.is_empty() {
                    Expr::empty_union(self.clone(), st)
                } else {
                    let largest = components.iter().max_by_key(|c| c.sizeof_in_bits(st)).unwrap();
                    Expr::union_expr(
                        self.clone(),
                        largest.name(),
                        largest.typ().zero_initializer(st),
                        st,
                    )
                }
            }
            UnionTag(tag) => st.lookup(*tag).unwrap().typ.zero_initializer(st),
            Vector { typ, size } => {
                let zero = typ.zero_initializer(st);
                let size = (*size).try_into().unwrap();
                let elems = vec![zero; size];
                Expr::vector_expr(self.clone(), elems)
            }

            // Cases that can't be zero init
            // Note that other than flexible array, none of these can be fields in a struct or union
            Code { .. }
            | Constructor
            | Empty
            | FlexibleArray { .. }
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | VariadicCode { .. } => panic!("Can't zero init {self:?}"),
        }
    }
}

impl Type {
    /// Given a struct type, construct a mapping from struct field names
    /// (Strings) to struct field types (Types).
    ///
    /// The Struct variant of the Type enum models the fields of a struct as a
    /// list of pairs (data type components) consisting of a field name and a
    /// field type.  A pair may represent an actual field in the struct or just
    /// padding in the layout of the struct.  This function returns a mapping of
    /// field names (ignoring the padding fields) to field types.  This makes it
    /// easier to look up field types (and modestly easier to interate over
    /// field types).
    pub fn struct_field_types(&self, symbol_table: &SymbolTable) -> BTreeMap<InternedString, Type> {
        // TODO: Accept a Struct type, too, and not just a StructTag assumed below.
        assert!(self.is_struct_tag());

        let mut types: BTreeMap<InternedString, Type> = BTreeMap::new();
        let fields = self.lookup_components(symbol_table).unwrap();
        for field in fields {
            if field.is_padding() {
                continue;
            }
            types.insert(field.name(), field.typ());
        }
        types
    }

    /// Generate a string which uniquely identifies the given type
    /// while also being a valid variable/funcion name
    pub fn to_identifier(&self) -> String {
        // Use String instead of InternedString, since we don't want to intern temporaries.
        match self {
            Type::Array { typ, size } => {
                format!("array_of_{size}_{}", typ.to_identifier())
            }
            Type::Bool => "bool".to_string(),
            Type::CBitField { width, typ } => {
                format!("cbitfield_of_{width}_{}", typ.to_identifier())
            }
            Type::CInteger(int_kind) => format!("c_int_{int_kind:?}"),
            // e.g. `int my_func(double x, float_y) {`
            // => "code_from_double_float_to_int"
            Type::Code { parameters, return_type } => {
                let parameter_string = parameters
                    .iter()
                    .map(|param| param.typ().to_identifier())
                    .collect::<Vec<_>>()
                    .join("_");
                let return_string = return_type.to_identifier();
                format!("code_from_{parameter_string}_to_{return_string}")
            }
            Type::Constructor => "constructor".to_string(),
            Type::Double => "double".to_string(),
            Type::Empty => "empty".to_string(),
            Type::FlexibleArray { typ } => format!("flexarray_of_{}", typ.to_identifier()),
            Type::Float => "float".to_string(),
            Type::Float16 => "float16".to_string(),
            Type::Float128 => "float128".to_string(),
            Type::IncompleteStruct { tag } => tag.to_string(),
            Type::IncompleteUnion { tag } => tag.to_string(),
            Type::InfiniteArray { typ } => {
                format!("infinite_array_of_{}", typ.to_identifier())
            }
            Type::Integer => "integer".to_string(),
            Type::Pointer { typ } => format!("pointer_to_{}", typ.to_identifier()),
            Type::Signedbv { width } => format!("signed_bv_{width}"),
            Type::Struct { tag, .. } => format!("struct_{tag}"),
            Type::StructTag(tag) => format!("struct_tag_{tag}"),
            Type::TypeDef { name: tag, .. } => format!("type_def_{tag}"),
            Type::Union { tag, .. } => format!("union_{tag}"),
            Type::UnionTag(tag) => format!("union_tag_{tag}"),
            Type::Unsignedbv { width } => format!("unsigned_bv_{width}"),
            // e.g. `int my_func(double x, float_y, ..) {`
            // => "variadic_code_from_double_float_to_int"
            Type::VariadicCode { parameters, return_type } => {
                let parameter_string = parameters
                    .iter()
                    .map(|param| param.typ().to_identifier())
                    .collect::<Vec<_>>()
                    .join("_");
                let return_string = return_type.to_identifier();
                format!("variadic_code_from_{parameter_string}_to_{return_string}")
            }
            Type::Vector { typ, size } => {
                let typ = typ.to_identifier();
                format!("vec_of_{size}_{typ}")
            }
        }
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;
    use crate::goto_program::typ::CIntType::Char;
    use crate::goto_program::{Location, Symbol};
    use crate::machine_model::test_util::machine_model_test_stub;

    // Just a dummy name used for the tests.
    const NAME: &str = "Dummy";

    #[test]
    fn check_typedef_tag() {
        let type_def = Bool.to_typedef(NAME);
        assert_eq!(type_def.tag().unwrap().to_string().as_str(), NAME);
        assert_eq!(type_def.type_name().unwrap().to_string(), format!("tag-{NAME}"));
    }

    #[test]
    fn check_typedef_identifier() {
        let type_def = Bool.to_typedef(NAME);
        let id = type_def.to_identifier();
        assert!(id.ends_with(NAME));
        assert!(id.starts_with("type_def"));
    }

    #[test]
    fn check_typedef_create() {
        assert!(matches!(Bool.to_typedef(NAME), TypeDef { .. }));
        assert!(matches!(StructTag(NAME.into()).to_typedef(NAME), TypeDef { .. }));
        assert!(matches!(Double.to_typedef(NAME), TypeDef { .. }));
    }

    #[test]
    fn check_unwrap_typedef_works() {
        assert!(matches!(Bool.to_typedef(NAME).unwrap_typedef(), Bool));
        assert!(matches!(Double.to_typedef(NAME).unwrap_typedef(), Double));
        assert!(matches!(StructTag(NAME.into()).to_typedef(NAME).unwrap_typedef(), StructTag(..)));
    }

    #[test]
    fn check_unwrap_nested_typedef_works() {
        assert!(matches!(Bool.to_typedef(NAME).to_typedef(NAME).unwrap_typedef(), Bool));
        assert!(matches!(
            StructTag(NAME.into()).to_typedef(NAME).to_typedef(NAME).unwrap_typedef(),
            StructTag(..)
        ));
    }

    fn check_properties(src_type: Type) {
        let type_def = src_type.clone().to_typedef(NAME);
        let mm = machine_model_test_stub();
        assert_eq!(type_def.is_empty(), src_type.is_empty());
        assert_eq!(type_def.is_double(), src_type.is_double());
        assert_eq!(type_def.is_bool(), src_type.is_bool());
        assert_eq!(type_def.is_long_int(), src_type.is_long_int());
        assert_eq!(type_def.is_array(), src_type.is_array());
        assert_eq!(type_def.is_array_like(), src_type.is_array_like());
        assert_eq!(type_def.is_union(), src_type.is_union());
        assert_eq!(type_def.is_union_like(), src_type.is_union_like());
        assert_eq!(type_def.is_union_tag(), src_type.is_union_tag());
        assert_eq!(type_def.is_struct_like(), src_type.is_struct_like());
        assert_eq!(type_def.is_struct(), src_type.is_struct());
        assert_eq!(type_def.is_signed(&mm), src_type.is_signed(&mm));
        assert_eq!(type_def.is_unsigned(&mm), src_type.is_unsigned(&mm));
        assert_eq!(type_def.is_scalar(), src_type.is_scalar());
        assert_eq!(type_def.is_float(), src_type.is_float());
        assert_eq!(type_def.is_floating_point(), src_type.is_floating_point());
        assert_eq!(type_def.width(), src_type.width());
        assert_eq!(type_def.can_be_lvalue(), src_type.can_be_lvalue());
    }

    /// Check that a typedef is equivalent to its base type.
    /// Note that not all types can be checked for equivalence.
    fn check_equivalent(src_type: Type, sym_table: SymbolTable) {
        let type_def = src_type.clone().to_typedef(NAME);
        assert!(type_def.is_structurally_equivalent_to(&src_type, &sym_table));
        assert!(src_type.is_structurally_equivalent_to(&type_def, &sym_table));
    }

    #[test]
    fn check_typedef_bool_properties() {
        check_properties(Bool);
    }

    #[test]
    fn check_typedef_empty_properties() {
        check_properties(Empty);
        check_equivalent(Empty, SymbolTable::new(machine_model_test_stub()));
    }

    #[test]
    fn check_typedef_float_properties() {
        check_properties(Float);
        check_equivalent(Float, SymbolTable::new(machine_model_test_stub()));
    }

    #[test]
    fn check_typedef_struct_properties() {
        // Create a struct with a random field.
        let struct_name: InternedString = "MyStruct".into();
        let struct_type = Type::struct_type(
            struct_name,
            vec![DatatypeComponent::Field { name: "field".into(), typ: Double }],
        );
        // Insert a field to the sym table to represent the struct field.
        let mut sym_table = SymbolTable::new(machine_model_test_stub());
        sym_table.ensure(struct_type.type_name().unwrap(), |_, name| {
            Symbol::variable(name, name, struct_type.clone(), Location::none())
        });

        check_properties(struct_type.clone());
        check_equivalent(struct_type, sym_table);
    }

    #[test]
    fn check_typedef_array_properties() {
        check_properties(Array { typ: Box::new(CInteger(Char)), size: 10 });
    }

    #[test]
    fn check_typedef_pointer_properties() {
        let ptr_type = Pointer { typ: Box::new(Empty) };
        check_properties(ptr_type.clone());
        check_equivalent(ptr_type, SymbolTable::new(machine_model_test_stub()));
    }
}
