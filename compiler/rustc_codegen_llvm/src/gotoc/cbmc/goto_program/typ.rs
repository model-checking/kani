// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use self::DatatypeComponent::*;
use self::Type::*;
use super::super::utils::aggr_name;
use super::super::MachineModel;
use super::{Expr, SymbolTable};
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
    IncompleteStruct { tag: String },
    /// `union x {}`
    IncompleteUnion { tag: String },
    /// CBMC specific. `typ x[__CPROVER_infinity()]`
    InfiniteArray { typ: Box<Type> },
    /// `typ*`
    Pointer { typ: Box<Type> },
    /// `int<width>_t`. e.g. `int32_t`
    Signedbv { width: u64 },
    /// `struct tag {component1.typ component1.name; component2.typ component2.name ... }`
    Struct { tag: String, components: Vec<DatatypeComponent> },
    /// CBMC specific. A reference into the symbol table, where the tag is the name of the symbol.
    StructTag(String),
    /// `union tag {component1.typ component1.name; component2.typ component2.name ... }`
    Union { tag: String, components: Vec<DatatypeComponent> },
    /// CBMC specific. A reference into the symbol table, where the tag is the name of the symbol.
    UnionTag(String),
    /// `int<width>_t`. e.g. `int32_t`
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
    Field { name: String, typ: Type },
    Padding { name: String, bits: u64 },
}

/// The formal parameters of a function.
#[derive(Debug, Clone)]
pub struct Parameter {
    typ: Type,
    /// The unique identifier that refers to this symbol (qualified by function name, module, etc)
    identifier: Option<String>,
    /// The local name the symbol has within the function
    base_name: Option<String>,
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

    pub fn name(&self) -> &str {
        match self {
            Field { name, .. } | Padding { name, .. } => &name,
        }
    }

    pub fn typ(&self) -> Type {
        match self {
            Field { typ, .. } => typ.clone(),
            Padding { bits, .. } => Type::unsigned_int(*bits),
        }
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
    pub fn base_name(&self) -> Option<&String> {
        self.base_name.as_ref()
    }

    pub fn identifier(&self) -> Option<&String> {
        self.identifier.as_ref()
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
            IncompleteStruct { tag } | Struct { tag, .. } => Some(Type::struct_tag(tag)),
            IncompleteUnion { tag } | Union { tag, .. } => Some(Type::union_tag(tag)),
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
            Bool => unreachable!("Doesn't have a sizeof"),
            CBitField { .. } => todo!("implement sizeof for bitfields"),
            CInteger(t) => t.sizeof_in_bits(st),
            Code { .. } => unreachable!("Doesn't have a sizeof"),
            Constructor => unreachable!("Doesn't have a sizeof"),
            Double => st.machine_model().double_width(),
            Empty => 0,
            FlexibleArray { .. } => 0,
            Float => st.machine_model().float_width(),
            IncompleteStruct { .. } => unreachable!("Doesn't have a sizeof"),
            IncompleteUnion { .. } => unreachable!("Doesn't have a sizeof"),
            InfiniteArray { .. } => unreachable!("Doesn't have a sizeof"),
            Pointer { .. } => st.machine_model().pointer_width(),
            Signedbv { width } => *width,
            Struct { components, .. } => {
                components.iter().map(|x| x.typ().sizeof_in_bits(st)).sum()
            }
            StructTag(tag) => st.lookup(tag).unwrap().typ.sizeof_in_bits(st),
            Union { components, .. } => {
                components.iter().map(|x| x.typ().sizeof_in_bits(st)).max().unwrap_or(0)
            }
            UnionTag(tag) => st.lookup(tag).unwrap().typ.sizeof_in_bits(st),
            Unsignedbv { width } => *width,
            VariadicCode { .. } => unreachable!("Doesn't have a sizeof"),
            Vector { typ, size } => typ.sizeof_in_bits(st) * size,
        }
    }

    /// Get the tag of a struct or union.
    pub fn tag(&self) -> Option<&str> {
        match self {
            IncompleteStruct { tag }
            | IncompleteUnion { tag }
            | Struct { tag, .. }
            | StructTag(tag)
            | Union { tag, .. }
            | UnionTag(tag) => Some(tag),
            _ => None,
        }
    }

    /// Given a `struct foo` or `union foo`, returns `Some("tag-foo")`.
    /// Otherwise, returns `None`.
    pub fn type_name(&self) -> Option<String> {
        match self {
            IncompleteStruct { tag }
            | Struct { tag, .. }
            | IncompleteUnion { tag }
            | Union { tag, .. } => Some(aggr_name(tag)),
            StructTag(tag) | UnionTag(tag) => Some(tag.to_string()),
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

    /// Whether the current type is an integer with finite width
    pub fn is_integer(&self) -> bool {
        match self {
            CInteger(_) | Signedbv { .. } | Unsignedbv { .. } => true,
            _ => false,
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

    /// This is a union (and not an incomplete union or union tag)
    pub fn is_union(&self) -> bool {
        match self {
            Union { .. } => true,
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

    /// corresponds to [code_typet] in CBMC, representing a function type
    ///    ret (params ..)
    pub fn code(parameters: Vec<Parameter>, return_type: Type) -> Self {
        Code { parameters, return_type: Box::new(return_type) }
    }

    /// CBMC, like c, allows function types to have unnamed formal paramaters
    /// `int foo(int, char, double)`
    pub fn code_with_unnamed_parameters(param_types: Vec<Type>, return_type: Type) -> Self {
        let parameters = param_types
            .into_iter()
            .map(|t| Parameter { identifier: None, base_name: None, typ: t })
            .collect();
        Type::code(parameters, return_type)
    }

    pub fn constructor() -> Self {
        Constructor
    }

    /// A component of a datatype (e.g. a field of a struct or union)
    pub fn datatype_component(name: &str, typ: Type) -> DatatypeComponent {
        Field { name: name.to_string(), typ }
    }

    // `__CPROVER_bitvector[bits] $pad<n>`
    pub fn datatype_padding(name: &str, bits: u64) -> DatatypeComponent {
        Padding { name: name.to_string(), bits }
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
    pub fn empty_struct(name: &str) -> Self {
        Struct { tag: name.to_string(), components: vec![] }
    }

    /// Empty union.
    /// union name {};
    pub fn empty_union(name: &str) -> Self {
        Union { tag: name.to_string(), components: vec![] }
    }

    pub fn flexible_array_of(self) -> Self {
        FlexibleArray { typ: Box::new(self) }
    }

    pub fn float() -> Self {
        Float
    }

    /// A formal function parameter.
    /// identifier: The unique identifier that refers to this parameter `foo12_bar17_x@1`
    /// base_name: the local name of the parameter within the function `x`
    /// typ: The type of the parameter
    pub fn parameter(
        identifier: Option<String>,
        base_name: Option<String>,
        typ: Type,
    ) -> Parameter {
        Parameter { identifier, base_name, typ }
    }

    /// A forward declared struct.
    /// struct foo;
    pub fn incomplete_struct(name: &str) -> Self {
        IncompleteStruct { tag: name.to_string() }
    }

    /// A forward declared union.
    /// union foo;
    pub fn incomplete_union(name: &str) -> Self {
        IncompleteUnion { tag: name.to_string() }
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
    pub fn struct_tag(name: &str) -> Self {
        StructTag(aggr_name(name))
    }

    pub fn components_are_unique(components: &Vec<DatatypeComponent>) -> bool {
        let mut names: Vec<_> = components.iter().map(|x| x.name()).collect();
        names.sort();
        names.dedup();
        names.len() == components.len()
    }

    /// struct name {
    ///     f1.typ f1.data; ...
    /// }
    pub fn struct_type(name: &str, components: Vec<DatatypeComponent>) -> Self {
        // TODO: turn this on after fixing issue #30
        // <https://github.com/model-checking/rmc/issues/30>
        //assert!(
        //    Type::components_are_unique(&components),
        //    "Components contain duplicates: {:?}",
        //    components
        //);
        Struct { tag: name.to_string(), components }
    }

    /// union name
    pub fn union_tag(name: &str) -> Self {
        UnionTag(aggr_name(name))
    }

    /// union name {
    ///     f1.typ f1.data; ...
    /// }
    pub fn union_type(name: &str, components: Vec<DatatypeComponent>) -> Self {
        assert!(
            Type::components_are_unique(&components),
            "Components contain duplicates: {:?}",
            components
        );
        Union { tag: name.to_string(), components }
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
    //TODO second mathematical check
    pub fn max_int(&self, mm: &MachineModel) -> i128 {
        assert!(self.is_integer());
        let w = self.native_width(mm).unwrap();
        assert!(w < 128);
        let shift = if self.is_signed(mm) { 129 - w } else { 128 - w };
        let max: u128 = u128::MAX >> shift;
        max.try_into().unwrap()
    }

    pub fn max_int_expr(&self, mm: &MachineModel) -> Expr {
        Expr::int_constant(self.max_int(mm), self.clone())
    }

    pub fn min_int(&self, mm: &MachineModel) -> i128 {
        assert!(self.is_integer());
        if self.is_unsigned(mm) {
            0
        } else {
            let max = self.max_int(mm);
            let min = -max - 1;
            min
        }
    }

    pub fn min_int_expr(&self, mm: &MachineModel) -> Expr {
        Expr::int_constant(self.min_int(mm), self.clone())
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
    pub fn struct_field_types(&self, symbol_table: &SymbolTable) -> BTreeMap<String, Type> {
        // TODO: Accept a Struct type, too, and not just a StructTag assumed below.
        assert!(self.is_struct_tag());

        let mut types: BTreeMap<String, Type> = BTreeMap::new();
        let fields = symbol_table.lookup_fields_in_type(self).unwrap();
        for field in fields {
            if field.is_padding() {
                continue;
            }
            types.insert(field.name().to_string(), field.typ());
        }
        types
    }
}
