// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use self::DatatypeComponent::*;
use self::Type::*;
use super::super::utils::{aggr_tag, max_int, min_int};
use super::super::MachineModel;
use super::{Expr, SymbolTable};
use crate::cbmc_string::InternedString;
use std::collections::BTreeMap;
use std::convert::TryInto;
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
    /// Machine dependent integers: `bool`, `char`, `int`, `size_t`, etc.
    CInteger(CIntType),
    /// `return_type x(parameters)`
    Code { parameters: Vec<Parameter>, return_type: Box<Type> },
    /// `__attribute__(constructor)`. Only valid as a function return type.
    /// https://gcc.gnu.org/onlinedocs/gcc-4.7.0/gcc/Function-Attributes.html
    Constructor,
    /// `double`
    Double,
    /// `void`
    Empty,
    /// `typ x[]`. Has a type, but no size. Only valid as the last element of a struct.
    FlexibleArray { typ: Box<Type> },
    /// `float`
    Float,
    /// `struct x {}`
    IncompleteStruct { tag: InternedString },
    /// `union x {}`
    IncompleteUnion { tag: InternedString },
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
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum CIntType {
    /// `bool`
    Bool,
    /// `char`
    Char,
    /// `int`
    Int,
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

    pub fn typ(&self) -> Type {
        match self {
            Field { typ, .. } => typ.clone(),
            Padding { bits, .. } => Type::unsigned_int(*bits),
        }
    }
}

//Constructors
impl DatatypeComponent {
    pub fn field<T: Into<InternedString>>(name: T, typ: Type) -> Self {
        let name = name.into();
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

impl CIntType {
    pub fn sizeof_in_bits(&self, st: &SymbolTable) -> u64 {
        match self {
            CIntType::Bool => st.machine_model().bool_width(),
            CIntType::Char => st.machine_model().char_width(),
            CIntType::Int => st.machine_model().int_width(),
            CIntType::SizeT => st.machine_model().pointer_width(),
            CIntType::SSizeT => st.machine_model().pointer_width(),
        }
    }
}

/// Getters
impl Type {
    /// Return the StructTag or UnionTag naming the struct or union type.
    pub fn aggr_tag(&self) -> Option<Type> {
        match self {
            IncompleteStruct { tag } | Struct { tag, .. } => Some(Type::struct_tag(*tag)),
            IncompleteUnion { tag } | Union { tag, .. } => Some(Type::union_tag(*tag)),
            StructTag(_) | UnionTag(_) => Some(self.clone()),
            _ => None,
        }
    }

    /// The base type of this type, if one exists.
    /// `typ*` | `typ x[width]` | `typ x : width`  -> `typ`,
    pub fn base_type(&self) -> Option<&Type> {
        match self {
            Array { typ, .. }
            | CBitField { typ, .. }
            | FlexibleArray { typ }
            | Pointer { typ }
            | Vector { typ, .. } => Some(typ),
            _ => None,
        }
    }

    pub fn components(&self) -> Option<&Vec<DatatypeComponent>> {
        match self {
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
        match self {
            CInteger(CIntType::SizeT) | CInteger(CIntType::SSizeT) | Pointer { .. } => {
                Some(mm.pointer_width())
            }
            CInteger(CIntType::Bool) => Some(mm.bool_width()),
            CInteger(CIntType::Char) => Some(mm.char_width()),
            CInteger(CIntType::Int) => Some(mm.int_width()),
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

    pub fn sizeof(&self, st: &SymbolTable) -> u64 {
        let bits = self.sizeof_in_bits(st);
        let char_width = st.machine_model().char_width();
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
        match self {
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
            Double => st.machine_model().double_width(),
            Empty => 0,
            FlexibleArray { .. } => 0,
            Float => st.machine_model().float_width(),
            IncompleteStruct { .. } => unreachable!("IncompleteStruct doesn't have a sizeof"),
            IncompleteUnion { .. } => unreachable!("IncompleteUnion doesn't have a sizeof"),
            InfiniteArray { .. } => unreachable!("InfiniteArray doesn't have a sizeof"),
            Pointer { .. } => st.machine_model().pointer_width(),
            Signedbv { width } => *width,
            Struct { components, .. } => {
                components.iter().map(|x| x.typ().sizeof_in_bits(st)).sum()
            }
            StructTag(tag) => st.lookup(*tag).unwrap().typ.sizeof_in_bits(st),
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
            | UnionTag(tag) => Some(*tag),
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
            | Union { tag, .. } => Some(aggr_tag(*tag)),
            StructTag(tag) | UnionTag(tag) => Some(*tag),
            _ => None,
        }
    }

    /// the width of an integer type
    pub fn width(&self) -> Option<u64> {
        match self {
            CBitField { width, .. } | Signedbv { width } | Unsignedbv { width } => Some(*width),
            _ => None,
        }
    }
}

/// Predicates
impl Type {
    pub fn is_array(&self) -> bool {
        match self {
            Array { .. } => true,
            _ => false,
        }
    }

    pub fn is_array_like(&self) -> bool {
        match self {
            Array { .. } | FlexibleArray { .. } | Vector { .. } => true,
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match self {
            Bool => true,
            _ => false,
        }
    }

    pub fn is_c_bool(&self) -> bool {
        match self {
            Type::CInteger(CIntType::Bool) => true,
            _ => false,
        }
    }

    pub fn is_c_size_t(&self) -> bool {
        match self {
            Type::CInteger(CIntType::SizeT) => true,
            _ => false,
        }
    }

    pub fn is_c_ssize_t(&self) -> bool {
        match self {
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
        match self {
            Double => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Empty => true,
            _ => false,
        }
    }

    /// Whether self and other have the same concrete type on the given machine
    /// (specifically whether they have the same bit-size and signed-ness)
    pub fn is_equal_on_machine(&self, other: &Self, mm: &MachineModel) -> bool {
        if self == other {
            true
        } else {
            self.native_width(mm) == other.native_width(mm)
                && self.is_signed(mm) == other.is_signed(mm)
        }
    }

    pub fn is_flexible_array(&self) -> bool {
        match self {
            FlexibleArray { .. } => true,
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Float => true,
            _ => false,
        }
    }

    pub fn is_floating_point(&self) -> bool {
        match self {
            Double | Float => true,
            _ => false,
        }
    }

    pub fn is_c_integer(&self) -> bool {
        match self {
            CInteger(_) => true,
            _ => false,
        }
    }

    /// Whether the current type is an integer with finite width
    pub fn is_integer(&self) -> bool {
        match self {
            CInteger(_) | Signedbv { .. } | Unsignedbv { .. } => true,
            _ => false,
        }
    }

    pub fn is_lvalue(&self) -> bool {
        match self {
            Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | Float
            | Pointer { .. }
            | Signedbv { .. }
            | Struct { .. }
            | StructTag(_)
            | Union { .. }
            | UnionTag(_)
            | Unsignedbv { .. }
            | Vector { .. } => true,

            Array { .. }
            | Code { .. }
            | Constructor
            | Empty
            | FlexibleArray { .. }
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | VariadicCode { .. } => false,
        }
    }

    /// Is the current type either an integer or a floating point?
    pub fn is_numeric(&self) -> bool {
        self.is_floating_point() || self.is_integer()
    }

    pub fn is_pointer(&self) -> bool {
        match self {
            Pointer { .. } => true,
            _ => false,
        }
    }

    /// Is this a size_t, ssize_t, or pointer?
    pub fn is_pointer_width(&self) -> bool {
        match self {
            Pointer { .. } | CInteger(CIntType::SizeT) | CInteger(CIntType::SSizeT) => true,
            _ => false,
        }
    }

    /// Is this a signed integer
    pub fn is_signed(&self, mm: &MachineModel) -> bool {
        match self {
            CInteger(CIntType::Int) | CInteger(CIntType::SSizeT) | Signedbv { .. } => true,
            CInteger(CIntType::Char) => !mm.char_is_unsigned(),
            _ => false,
        }
    }

    /// This is a struct (and not an incomplete struct or struct tag)
    pub fn is_struct(&self) -> bool {
        match self {
            Struct { .. } => true,
            _ => false,
        }
    }

    pub fn is_struct_like(&self) -> bool {
        match self {
            IncompleteStruct { .. } | Struct { .. } | StructTag(_) => true,
            _ => false,
        }
    }

    /// This is a struct tag
    pub fn is_struct_tag(&self) -> bool {
        match self {
            StructTag(_) => true,
            _ => false,
        }
    }

    /// This is a union (and not an incomplete union or union tag)
    pub fn is_union(&self) -> bool {
        match self {
            Union { .. } => true,
            _ => false,
        }
    }

    /// This is a union tag
    pub fn is_union_tag(&self) -> bool {
        match self {
            UnionTag(_) => true,
            _ => false,
        }
    }

    /// Is this an unsigned integer
    pub fn is_unsigned(&self, mm: &MachineModel) -> bool {
        match self {
            CInteger(CIntType::Bool) | CInteger(CIntType::SizeT) | Unsignedbv { .. } => true,
            CInteger(CIntType::Char) => mm.char_is_unsigned(),
            _ => false,
        }
    }

    pub fn is_variadic_code(&self) -> bool {
        match self {
            VariadicCode { .. } => true,
            _ => false,
        }
    }

    pub fn is_vector(&self) -> bool {
        match self {
            Vector { .. } => true,
            _ => false,
        }
    }

    pub fn is_transparent_type(&self, st: &SymbolTable) -> bool {
        match self {
            // Follow tags to get the underlying structure
            StructTag(tag) | UnionTag(tag) => st.lookup(*tag).unwrap().typ.is_transparent_type(st),

            // Recursively check: does this only have one field, which is either a wrapper or base type.
            Struct { .. } | Union { .. } => self.unwrap_transparent_type(st).is_some(),

            // Base types
            Array { .. }
            | Bool
            | CBitField { .. }
            | CInteger(_)
            | Code { .. }
            | Constructor
            | Double
            | Empty
            | FlexibleArray { .. }
            | Float
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | Pointer { .. }
            | Signedbv { .. }
            | Unsignedbv { .. }
            | VariadicCode { .. }
            | Vector { .. } => false,
        }
    }

    pub fn unwrap_transparent_type(&self, st: &SymbolTable) -> Option<Type> {
        match self {
            Array { .. }
            | Code { .. }
            | Constructor
            | FlexibleArray { .. }
            | IncompleteStruct { .. }
            | IncompleteUnion { .. }
            | InfiniteArray { .. }
            | VariadicCode { .. }
            | Vector { .. } => None,

            // Base types
            Bool
            | CBitField { .. }
            | CInteger(_)
            | Double
            | Empty
            | Float
            | Pointer { .. }
            | Signedbv { .. }
            | Unsignedbv { .. } => Some(self.clone()),

            // Follow tags to get the underlying structure
            StructTag(tag) | UnionTag(tag) => {
                st.lookup(*tag).unwrap().typ.unwrap_transparent_type(st)
            }

            // Recursively check: does this only have one field, which is either a wrapper or base type.
            Struct { components, .. } | Union { components, .. } => {
                if components.len() != 1 {
                    None
                } else {
                    match &components[0] {
                        Padding { .. } => None,
                        Field { typ, .. } => typ.unwrap_transparent_type(st),
                    }
                }
            }
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
    /// elem_t[size]
    pub fn array_of<T>(self, size: T) -> Self
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
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
        assert!(
            self.is_lvalue(),
            "Expected lvalue from {:?} {:?} {:?}",
            self,
            identifier,
            base_name
        );
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

    pub fn c_size_t() -> Self {
        CInteger(CIntType::SizeT)
    }

    pub fn c_ssize_t() -> Self {
        CInteger(CIntType::SSizeT)
    }

    /// corresponds to [code_typet] in CBMC, representing a function type
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

    /// A component of a datatype (e.g. a field of a struct or union)
    pub fn datatype_component<T: Into<InternedString>>(name: T, typ: Type) -> DatatypeComponent {
        let name = name.into();
        Field { name, typ }
    }

    // `__CPROVER_bitvector[bits] $pad<n>`
    pub fn datatype_padding<T: Into<InternedString>>(name: T, bits: u64) -> DatatypeComponent {
        let name = name.into();

        Padding { name, bits }
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
        InfiniteArray { typ: Box::new(self) }
    }

    /// self *
    pub fn to_pointer(self) -> Self {
        Pointer { typ: Box::new(self) }
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

    pub fn components_are_unique(components: &[DatatypeComponent]) -> bool {
        let mut names: Vec<_> = components.iter().map(|x| x.name()).collect();
        names.sort();
        names.dedup();
        names.len() == components.len()
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
            "Components contain duplicates: {:?}",
            components
        );
        let tag = tag.into();
        Struct { tag, components }
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
            "Components contain duplicates: {:?}",
            components
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

    /// corresponds to [code_typet] in CBMC, representing a function type
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
        if self.is_integer() {
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
        if self.is_integer() {
            Expr::int_constant(0, self.clone())
        } else if self.is_c_bool() {
            Expr::c_false()
        } else if self.is_float() {
            Expr::float_constant(0.0)
        } else if self.is_double() {
            Expr::double_constant(0.0)
        } else if self.is_pointer() {
            Expr::pointer_constant(0, self.clone())
        } else {
            unreachable!("Can't convert {:?} to a one value", self);
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
        let fields = symbol_table.lookup_fields_in_type(self).unwrap();
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
                format!("array_of_{}_{}", size, typ.to_identifier())
            }
            Type::Bool => format!("bool"),
            Type::CBitField { width, typ } => {
                format!("cbitfield_of_{}_{}", width, typ.to_identifier())
            }
            Type::CInteger(int_kind) => format!("c_int_{:?}", int_kind),
            // e.g. `int my_func(double x, float_y) {`
            // => "code_from_double_float_to_int"
            Type::Code { parameters, return_type } => {
                let parameter_string = parameters
                    .iter()
                    .map(|param| param.typ().to_identifier())
                    .collect::<Vec<_>>()
                    .join("_");
                let return_string = return_type.to_identifier();
                format!("code_from_{}_to_{}", parameter_string, return_string)
            }
            Type::Constructor => format!("constructor"),
            Type::Double => format!("double"),
            Type::Empty => format!("empty"),
            Type::FlexibleArray { typ } => format!("flexarray_of_{}", typ.to_identifier()),
            Type::Float => format!("float"),
            Type::IncompleteStruct { tag } => tag.to_string(),
            Type::IncompleteUnion { tag } => tag.to_string(),
            Type::InfiniteArray { typ } => {
                format!("infinite_array_of_{}", typ.to_identifier())
            }
            Type::Pointer { typ } => format!("pointer_to_{}", typ.to_identifier()),
            Type::Signedbv { width } => format!("signed_bv_{}", width),
            Type::Struct { tag, .. } => format!("struct_{}", tag),
            Type::StructTag(tag) => format!("struct_tag_{}", tag),
            Type::Union { tag, .. } => format!("union_{}", tag),
            Type::UnionTag(tag) => format!("union_tag_{}", tag),
            Type::Unsignedbv { width } => format!("unsigned_bv_{}", width),
            // e.g. `int my_func(double x, float_y, ..) {`
            // => "variadic_code_from_double_float_to_int"
            Type::VariadicCode { parameters, return_type } => {
                let parameter_string = parameters
                    .iter()
                    .map(|param| param.typ().to_identifier())
                    .collect::<Vec<_>>()
                    .join("_");
                let return_string = return_type.to_identifier();
                format!("variadic_code_from_{}_to_{}", parameter_string, return_string)
            }
            Type::Vector { size, typ } => {
                format!("vec_of_{}_{}", size, typ.to_identifier())
            }
        }
    }
}
