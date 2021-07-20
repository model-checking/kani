// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use self::BinaryOperand::*;
use self::ExprValue::*;
use self::UnaryOperand::*;
use super::super::MachineModel;
use super::{DatatypeComponent, Location, Parameter, Stmt, SwitchCase, SymbolTable, Type};
use num::bigint::BigInt;
use std::collections::BTreeMap;
use std::fmt::Debug;

///////////////////////////////////////////////////////////////////////////////////////////////
/// Datatypes
///////////////////////////////////////////////////////////////////////////////////////////////

/// An `Expr` represents an expression type: i.e. a computation that returns a value.
/// Every expression has a type, a value, and a location (which may be `None`).
///
/// The fields of `Expr` are kept private, and there are no getters that return mutable references.
/// This means that the only way to create and update `Expr`s is using the constructors and setters.
/// In a few cases, there are properties, such as the existence of a field on a struct type,
/// which can only be checked given a symbol table.
/// Other than these properties, the constructors ensure that all expressions are well-formed.
///
/// In general, expressions are constructed in a "chained" style:
///     ` *(&x + i);` would translate to `x.address_of().plus(i).dereference()`
/// By default, these expressions have no location: to add a location, use the `.with_location()`
/// fluent builder to add locations when desired.
/// e.g. `x.address_of().with_location(l1).plus(i).with_location(l2).dereference().with_location(l3)`
///
/// TODO:
/// The CBMC irep resentation uses sharing to reduce the in-memory size of expressions.
/// This is not currently implemented for these expressions, but would be possible given a factory.
#[derive(Debug, Clone)]
pub struct Expr {
    value: Box<ExprValue>,
    typ: Type,
    location: Location,
}

/// The different kinds of values an expression can have.
/// The names are chosen to map directly onto the IrepID used by CBMC.
/// Each expression is described by reference to the corresponding C code that would generate it.
/// When an expression makes most sense in a broader statement context,
/// the characters >>> e <<< are used to mark the part described by the enum.
#[derive(Debug, Clone)]
pub enum ExprValue {
    /// `&self`
    AddressOf(Expr),
    /// `typ x[] = >>> {elems0, elems1 ...} <<<`
    Array {
        elems: Vec<Expr>,
    },
    /// `typ x[width] = >>> {elem} <<<`
    ArrayOf {
        elem: Expr,
    },
    /// `left = right`
    Assign {
        left: Expr,
        right: Expr,
    },
    /// `lhs op rhs`.  E.g. `lhs + rhs` if `op == BinaryOperand::Plus`
    BinOp {
        op: BinaryOperand,
        lhs: Expr,
        rhs: Expr,
    },
    /// `(__CPROVER_bool) >>> true/false <<<`. True/False as a single bit boolean.
    BoolConstant(bool),
    /// Reinterpret bytes of e as type self.typ
    ByteExtract {
        e: Expr,
        offset: u64,
    },
    /// `(bool) 1`. True false as an 8 bit c_boolean.
    CBoolConstant(bool),
    /// `*self`
    Dereference(Expr),
    /// `1.0`
    DoubleConstant(f64),
    /// `1.0f`
    FloatConstant(f32),
    /// `function(arguments)`
    FunctionCall {
        function: Expr,
        arguments: Vec<Expr>,
    },
    /// `c ? t : e`
    If {
        c: Expr,
        t: Expr,
        e: Expr,
    },
    /// `array[index]`
    Index {
        array: Expr,
        index: Expr,
    },
    /// `123`
    IntConstant(BigInt),
    /// `lhs.field`
    Member {
        lhs: Expr,
        field: String,
    },
    /// `__nondet()`
    Nondet,
    /// `NULL`
    PointerConstant(u64),
    // `op++` etc
    SelfOp {
        op: SelfOperand,
        e: Expr,
    },
    /// https://gcc.gnu.org/onlinedocs/gcc/Statement-Exprs.html
    /// e.g. `({ int y = foo (); int z; if (y > 0) z = y; else z = - y; z; })`
    /// `({ op1; op2; ...})`
    StatementExpression {
        statements: Vec<Stmt>,
    },
    /// A raw string constant. Note that you normally actually want a pointer to the first element.
    /// `"s"`
    StringConstant {
        s: String,
    },
    /// Struct initializer  
    /// `struct foo the_foo = >>> {field1, field2, ... } <<<`
    Struct {
        values: Vec<Expr>,
    },
    /// `self`
    Symbol {
        identifier: String,
    },
    /// `(typ) self`. Target type is in the outer `Expr` struct.
    Typecast(Expr),
    /// Union initializer  
    /// `union foo the_foo = >>> {.field = value } <<<`
    Union {
        value: Expr,
        field: String,
    },
    // `op self` eg `! self` if `op == UnaryOperand::Not`
    UnOp {
        op: UnaryOperand,
        e: Expr,
    },
}

/// Binary operators. The names are the same as in the Irep representation.
#[derive(Debug, Clone, Copy)]
pub enum BinaryOperand {
    And,
    Ashr,
    Bitand,
    Bitor,
    Bitxor,
    Div,
    Equal,
    Ge,
    Gt,
    IeeeFloatEqual,
    IeeeFloatNotequal,
    Implies,
    Le,
    Lshr,
    Lt,
    Minus,
    Mod,
    Mult,
    Notequal,
    Or,
    OverflowMinus,
    OverflowMult,
    OverflowPlus,
    Plus,
    Rol,
    Ror,
    Shl,
    Xor,
}

// Unary operators with side-effects
#[derive(Debug, Clone, Copy)]
pub enum SelfOperand {
    /// `self--`
    Postdecrement,
    /// `self++`
    Postincrement,
    /// `--self`
    Predecrement,
    /// `++self`
    Preincrement,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperand {
    /// `~self`
    Bitnot,
    /// `__builtin_bswap<n>(self)`
    Bswap,
    /// `__CPROVER_DYNAMIC_OBJECT(self)`
    IsDynamicObject,
    /// `!self`
    Not,
    /// `__CPROVER_OBJECT_SIZE(self)`
    ObjectSize,
    /// `__CPROVER_POINTER_OBJECT(self)`
    PointerObject,
    /// `__CPROVER_POINTER_OFFSET(self)`
    PointerOffset,
    /// `__builtin_popcount(self)`
    Popcount,
    /// `__builtin_cttz(self)`
    CountTrailingZeros { allow_zero: bool },
    /// `__builtin_ctlz(self)`
    CountLeadingZeros { allow_zero: bool },
    /// `-self`
    UnaryMinus,
}

/// The return type for `__builtin_op_overflow` operations
pub struct ArithmeticOverflowResult {
    /// If overflow did not occur, the result of the operation. Otherwise undefined.
    pub result: Expr,
    /// Boolean: true if overflow occured, false otherwise.
    pub overflowed: Expr,
}

///////////////////////////////////////////////////////////////////////////////////////////////
/// Implementations
///////////////////////////////////////////////////////////////////////////////////////////////

/// Getters
impl Expr {
    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn typ(&self) -> &Type {
        &self.typ
    }

    pub fn value(&self) -> &ExprValue {
        &self.value
    }

    /// If the expression is an Int constant type, return its value
    pub fn int_constant_value(&self) -> Option<BigInt> {
        match &*self.value {
            ExprValue::IntConstant(i) => Some(i.clone()),
            _ => None,
        }
    }

    pub fn struct_expr_values(&self) -> Option<&Vec<Expr>> {
        match &*self.value {
            Struct { values } => Some(values),
            _ => None,
        }
    }
}

/// Predicates
impl Expr {
    pub fn is_int_constant(&self) -> bool {
        match *self.value {
            IntConstant(_) => true,
            _ => false,
        }
    }
    pub fn is_side_effect(&self) -> bool {
        match *self.value {
            Assign { .. }
            | FunctionCall { .. }
            | Nondet
            | SelfOp { .. }
            | StatementExpression { .. } => true,
            _ => false,
        }
    }

    pub fn is_symbol(&self) -> bool {
        match *self.value {
            Symbol { .. } => true,
            _ => false,
        }
    }

    /// What typecasts are legal. Based off the C standard, plus some additional types
    /// that don't appear in the standard, like `bool`
    /// https://docs.microsoft.com/en-us/cpp/c-language/type-cast-conversions?view=msvc-160
    pub fn can_cast_from(source: &Type, target: &Type) -> bool {
        if source == target {
            true
        } else if target.is_bool() {
            source.is_c_bool() || source.is_integer() || source.is_pointer()
        } else if target.is_c_bool() {
            source.is_integer() || source.is_pointer() || source.is_bool()
        } else if target.is_integer() {
            source.is_c_bool()
                || source.is_integer()
                || source.is_floating_point()
                || source.is_pointer()
        } else if target.is_floating_point() {
            source.is_numeric()
        } else if target.is_pointer() {
            source.is_integer() || source.is_pointer()
        } else if target.is_empty() {
            true
        } else {
            false
        }
    }

    pub fn can_cast_to(&self, target: &Type) -> bool {
        Expr::can_cast_from(&self.typ, target)
    }

    pub fn can_take_address_of(&self) -> bool {
        match *self.value {
            Dereference(_) | Index { .. } | Member { .. } | Symbol { .. } => true,
            _ => false,
        }
    }
}

/// Setters
impl Expr {
    pub fn with_location(mut self, loc: Location) -> Self {
        self.location = loc;
        self
    }
}

/// Private constructor. Making this a macro allows multiple reference to self in the same call.
macro_rules! expr {
    ( $value:expr,  $typ:expr) => {{
        let typ = $typ;
        let value = Box::new($value);
        Expr { value, typ, location: Location::none() }
    }};
}

/// Constructors for the main types
impl Expr {
    /// `&self`
    pub fn address_of(self) -> Self {
        assert!(self.can_take_address_of());
        expr!(AddressOf(self), self.typ.clone().to_pointer())
    }

    /// `typ x[width] = >>> {elem} <<<`
    pub fn array_constant(self, width: u64) -> Self {
        assert!(self.is_int_constant());
        expr!(ArrayOf { elem: self }, self.typ.clone().array_of(width))
    }

    /// `typ x[] = >>> {elems0, elems1 ...} <<<`
    pub fn array_expr(typ: Type, elems: Vec<Expr>) -> Self {
        if let Type::Array { size, typ: value_typ } = typ.clone() {
            assert_eq!(size as usize, elems.len());
            assert!(
                elems.iter().all(|x| x.typ == *value_typ),
                "Array type and value types don't match: \n{:?}\n{:?}",
                typ,
                elems
            );
        } else {
            unreachable!("Can't make an array_val with non-array target type {:?}", typ);
        }
        expr!(Array { elems }, typ)
    }

    /// `(__CPROVER_bool) >>> true/false <<<`. True/False as a single bit boolean.
    pub fn bool_constant(c: bool) -> Self {
        expr!(BoolConstant(c), Type::bool())
    }

    /// `(__CPROVER_bool) false`. False as a single bit boolean.
    pub fn bool_false() -> Self {
        Expr::bool_constant(false)
    }

    /// `(__CPROVER_bool) true`. True as a single bit boolean.
    pub fn bool_true() -> Self {
        Expr::bool_constant(true)
    }

    /// `(bool) 1`. True false as an 8 bit c_boolean.
    pub fn c_bool_constant(c: bool) -> Self {
        expr!(CBoolConstant(c), Type::c_bool())
    }

    /// `(bool) 1`. True false as an 8 bit c_boolean.
    pub fn c_true() -> Self {
        Self::c_bool_constant(true)
    }

    /// `(bool) 0`. True false as an 8 bit c_boolean.
    pub fn c_false() -> Self {
        Self::c_bool_constant(false)
    }

    /// `(typ) self`.
    pub fn cast_to(self, typ: Type) -> Self {
        assert!(self.can_cast_to(&typ), "Can't cast\n\n{:?}\n\n{:?}", self, typ);
        if self.typ == typ {
            self
        } else if typ.is_bool() {
            let zero = self.typ.zero();
            self.neq(zero)
        } else {
            expr!(Typecast(self), typ)
        }
    }

    /// Casts value to new_typ, only when the current type of value
    /// is equivalent to new_typ on the given machine (e.g. i32 -> c_int)
    pub fn cast_to_machine_equivalent_type(self, new_typ: &Type, mm: &MachineModel) -> Expr {
        if self.typ() == new_typ {
            self
        } else {
            assert!(self.typ().is_equal_on_machine(new_typ, mm));
            self.cast_to(new_typ.clone())
        }
    }

    /// Casts arguments to type of function parameters when the corresponding types
    /// are equivalent on the given machine (e.g. i32 -> c_int)
    pub fn cast_arguments_to_machine_equivalent_function_parameter_types(
        function: &Expr,
        mut arguments: Vec<Expr>,
        mm: &MachineModel,
    ) -> Vec<Expr> {
        let parameters = function.typ().parameters().unwrap();
        assert!(arguments.len() >= parameters.len());
        let mut rval: Vec<_> = parameters
            .iter()
            .map(|parameter| {
                arguments.remove(0).cast_to_machine_equivalent_type(&parameter.typ(), mm)
            })
            .collect();

        rval.append(&mut arguments);

        rval
    }

    /// *self: t
    pub fn dereference(self) -> Self {
        assert!(self.typ.is_pointer());
        expr!(Dereference(self), self.typ.base_type().unwrap().clone())
    }

    /// `1.0`
    pub fn double_constant(c: f64) -> Self {
        expr!(DoubleConstant(c), Type::double())
    }

    /// `union {double d; uint64_t bp} u = {.bp = 0x1234}; >>> u.d <<<`
    pub fn double_constant_from_bitpattern(bp: u64) -> Self {
        let c = unsafe { std::mem::transmute(bp) };
        Self::double_constant(c)
    }

    /// `1.0f`
    pub fn float_constant(c: f32) -> Self {
        expr!(FloatConstant(c), Type::float())
    }

    /// `union {float f; uint32_t bp} u = {.bp = 0x1234}; >>> u.f <<<`
    pub fn float_constant_from_bitpattern(bp: u32) -> Self {
        let c = unsafe { std::mem::transmute(bp) };
        Self::float_constant(c)
    }

    /// `self[index]`
    pub fn index_array(self, index: Expr) -> Self {
        assert!(index.typ.is_integer());
        assert!(self.typ.is_array_like());
        let typ = self.typ().base_type().unwrap().clone();
        expr!(Index { array: self, index }, typ)
    }

    /// `123`
    pub fn int_constant<T>(i: T, typ: Type) -> Self
    where
        T: Into<BigInt>,
    {
        assert!(typ.is_integer());
        let i = i.into();
        //TODO: This check fails on some regressions
        // if i != 0 && i != 1 {
        //     assert!(
        //         typ.min_int() <= i && i <= typ.max_int(),
        //         "{} {} {} {:?}",
        //         i,
        //         typ.min_int(),
        //         typ.max_int(),
        //         typ
        //     );
        // }
        expr!(IntConstant(i), typ)
    }

    pub fn typecheck_call(function: &Expr, arguments: &[Expr]) -> bool {
        // For variadic functions, all named arguments must match the type of their formal param.
        // Extra arguments (e.g the ... args) can have any type.
        fn typecheck_named_args(parameters: &[Parameter], arguments: &[Expr]) -> bool {
            parameters.iter().zip(arguments.iter()).all(|(p, a)| a.typ() == p.typ())
        }

        if function.typ().is_code() {
            let parameters = function.typ().parameters().unwrap();
            arguments.len() == parameters.len() && typecheck_named_args(parameters, arguments)
        } else if function.typ().is_variadic_code() {
            let parameters = function.typ().parameters().unwrap();
            arguments.len() >= parameters.len() && typecheck_named_args(parameters, arguments)
        } else {
            false
        }
    }

    /// `function(arguments)`
    ///
    /// This gives an _expression_.
    /// If you are using this in statement context (e.g. ignoring or assigning the value), use
    /// the `Stmt::function_call` constructor.
    pub fn call(self, arguments: Vec<Expr>) -> Self {
        assert!(
            Expr::typecheck_call(&self, &arguments),
            "Function call does not type check:\nfunc: {:?}\nargs: {:?}",
            self,
            arguments
        );
        let typ = self.typ().return_type().unwrap().clone();
        expr!(FunctionCall { function: self, arguments }, typ)
    }

    /// `self.field`
    pub fn member(self, field: &str, symbol_table: &SymbolTable) -> Self {
        assert!(
            self.typ.is_struct_tag() || self.typ.is_union_tag(),
            "Can't apply .member operation to\n\t{:?}\n\t{:?}",
            self,
            field,
        );

        let typ = symbol_table.lookup_field_type_in_type(self.typ(), field).unwrap().clone();
        expr!(Member { lhs: self, field: field.to_string() }, typ)
    }

    /// `__nondet_typ()`
    pub fn nondet(typ: Type) -> Self {
        expr!(Nondet, typ)
    }

    /// `e.g. NULL`
    pub fn pointer_constant(c: u64, typ: Type) -> Self {
        assert!(typ.is_pointer());
        expr!(PointerConstant(c), typ)
    }

    /// https://gcc.gnu.org/onlinedocs/gcc/Statement-Exprs.html
    /// e.g. `({ int y = foo (); int z; if (y > 0) z = y; else z = - y; z; })`
    /// `({ op1; op2; ...})`
    pub fn statement_expression(ops: Vec<Stmt>, typ: Type) -> Self {
        assert!(ops.len() > 0);
        assert_eq!(ops.last().unwrap().get_expression().unwrap().typ, typ);
        expr!(StatementExpression { statements: ops }, typ)
    }

    /// Internal helper function for Struct initalizer  
    /// `struct foo the_foo = >>> {.field1 = val1, .field2 = val2, ... } <<<`
    /// ALL fields must be given, including padding
    fn struct_expr_with_explicit_padding(
        typ: Type,
        fields: &[DatatypeComponent],
        values: Vec<Expr>,
    ) -> Self {
        assert_eq!(fields.len(), values.len());
        // Check that each formal field has an value
        assert!(
            fields.iter().zip(values.iter()).all(|(f, v)| f.typ() == *v.typ()),
            "Error in struct_expr; value type does not match field type.\n\t{:?}\n\t{:?}",
            typ,
            values
        );
        expr!(Struct { values }, typ)
    }

    /// Struct initializer  
    /// `struct foo the_foo = >>> {.field1 = val1, .field2 = val2, ... } <<<`
    /// Note that only the NON padding fields should be explicitly given.
    /// Padding fields are automatically inserted using the type from the `SymbolTable`
    pub fn struct_expr(
        typ: Type,
        mut components: BTreeMap<String, Expr>,
        symbol_table: &SymbolTable,
    ) -> Self {
        assert!(
            typ.is_struct_tag(),
            "Error in struct_expr; must be given a struct_tag.\n\t{:?}\n\t{:?}",
            typ,
            components
        );
        let fields = symbol_table.lookup_fields_in_type(&typ).unwrap();
        let non_padding_fields: Vec<_> = fields.iter().filter(|x| !x.is_padding()).collect();
        assert_eq!(
            non_padding_fields.len(),
            components.len(),
            "Error in struct_expr; mismatch in number of fields and components.\n\t{:?}\n\t{:?}",
            typ,
            components
        );

        // Check that each formal field has an value
        for field in non_padding_fields {
            let field_typ = field.field_typ().unwrap();
            let value = components.get(field.name()).unwrap();
            assert_eq!(value.typ(), field_typ);
        }

        let values = fields
            .iter()
            .map(|field| {
                if field.is_padding() {
                    field.typ().nondet()
                } else {
                    components.remove(field.name()).unwrap()
                }
            })
            .collect();

        Expr::struct_expr_with_explicit_padding(typ, fields, values)
    }

    /// Struct initializer with default nondet fields except for given `components`
    /// `struct foo the_foo = >>> {.field1 = val1, .field2 = val2, ... } <<<`
    pub fn struct_expr_with_nondet_fields(
        typ: Type,
        mut components: BTreeMap<String, Expr>,
        symbol_table: &SymbolTable,
    ) -> Self {
        assert!(typ.is_struct_tag());
        let fields = symbol_table.lookup_fields_in_type(&typ).unwrap();
        let non_padding_fields: Vec<_> = fields.iter().filter(|x| !x.is_padding()).collect();
        let values = non_padding_fields
            .iter()
            .map(|field| {
                let field_name = field.name();
                if components.contains_key(field_name) {
                    components.remove(field_name).unwrap()
                } else {
                    field.typ().nondet()
                }
            })
            .collect();
        Expr::struct_expr_from_values(typ, values, symbol_table)
    }

    /// Struct initializer  
    /// `struct foo the_foo = >>> {field1, field2, ... } <<<`
    /// Note that only the NON padding fields should be explicitly given.
    /// Padding fields are automatically inserted using the type from the `SymbolTable`
    pub fn struct_expr_from_values(
        typ: Type,
        mut non_padding_values: Vec<Expr>,
        symbol_table: &SymbolTable,
    ) -> Self {
        assert!(
            typ.is_struct_tag(),
            "Error in struct_expr; must be given struct_tag.\n\t{:?}\n\t{:?}",
            typ,
            non_padding_values
        );
        let fields = symbol_table.lookup_fields_in_type(&typ).unwrap();
        let non_padding_fields: Vec<_> = fields.iter().filter(|x| !x.is_padding()).collect();
        assert_eq!(
            non_padding_fields.len(),
            non_padding_values.len(),
            "Error in struct_expr; mismatch in number of fields and values.\n\t{:?}\n\t{:?}",
            typ,
            non_padding_values
        );
        assert!(
            non_padding_fields
                .iter()
                .zip(non_padding_values.iter())
                .all(|(f, v)| f.field_typ().unwrap() == v.typ()),
            "Error in struct_expr; value type does not match field type.\n\t{:?}\n\t{:?}",
            typ,
            non_padding_values
        );

        let values = fields
            .iter()
            .map(|f| if f.is_padding() { f.typ().nondet() } else { non_padding_values.remove(0) })
            .collect();

        Expr::struct_expr_with_explicit_padding(typ, fields, values)
    }

    /// Struct initializer  
    /// `struct foo the_foo = >>> {field1, padding2, field3, ... } <<<`
    /// Note that padding fields should be explicitly given.
    /// This would be used when the values and padding have already been combined,
    /// e.g. when extracting the values out of an existing struct expr (see transformer.rs)
    pub fn struct_expr_from_padded_values(
        typ: Type,
        values: Vec<Expr>,
        symbol_table: &SymbolTable,
    ) -> Self {
        assert!(
            typ.is_struct_tag(),
            "Error in struct_expr; must be given struct_tag.\n\t{:?}\n\t{:?}",
            typ,
            values
        );
        let fields = symbol_table.lookup_fields_in_type(&typ).unwrap();
        assert_eq!(
            fields.len(),
            values.len(),
            "Error in struct_expr; mismatch in number of padded fields and padded values.\n\t{:?}\n\t{:?}",
            typ,
            values
        );
        assert!(
            fields.iter().zip(values.iter()).all(|(f, v)| &f.typ() == v.typ()),
            "Error in struct_expr; value type does not match field type.\n\t{:?}\n\t{:?}",
            typ,
            values
        );

        Expr::struct_expr_with_explicit_padding(typ, fields, values)
    }

    /// `identifier`
    pub fn symbol_expression(identifier: String, typ: Type) -> Self {
        expr!(Symbol { identifier }, typ)
    }

    /// `self ? t : e`
    pub fn ternary(self, t: Expr, e: Expr) -> Expr {
        assert_eq!(t.typ, e.typ);
        expr!(If { c: self.cast_to(Type::bool()), t, e }, t.typ.clone())
    }

    /// Reinterpret the bits of `self` as being of type `t`.
    /// Note that this differs from standard casts, which may convert values.
    /// To abuse syntax: `(uint32_t)(1.0) == 1`, while `(1.0).transmute_to(uin32_t) == 0x3f800000`
    /// If `sizeof(self.typ()) != sizeof(t)`, then CBMC will truncate/extend with nondet as needed.
    /// In cases where this is not desired (e.g. casting the first element of a struct to the struct
    /// type itself, try using `reinterpret_cast`.
    pub fn transmute_to(self, t: Type, st: &SymbolTable) -> Expr {
        assert_eq!(self.typ().sizeof_in_bits(st), t.sizeof_in_bits(st));
        expr!(ByteExtract { e: self, offset: 0 }, t)
    }

    /// Union initializer  
    /// `union foo the_foo = >>> {.field = value } <<<`
    pub fn union_expr(typ: Type, field: &str, value: Expr, symbol_table: &SymbolTable) -> Self {
        assert!(typ.is_union_tag());
        assert_eq!(symbol_table.lookup_field_type_in_type(&typ, field), Some(value.typ()));
        expr!(Union { value, field: field.to_string() }, typ)
    }
}

/// Constructors for Binary Operations
impl Expr {
    fn typecheck_binop_args(op: BinaryOperand, lhs: &Expr, rhs: &Expr) -> bool {
        match op {
            // Arithmetic which can include pointers
            Minus => {
                (lhs.typ == rhs.typ)
                    && (lhs.typ.is_pointer() || lhs.typ.is_numeric() || lhs.typ.is_vector())
                    || (lhs.typ.is_pointer() && rhs.typ.is_integer())
            }
            Plus => {
                (lhs.typ == rhs.typ && (lhs.typ.is_numeric() || lhs.typ.is_vector()))
                    || (lhs.typ.is_pointer() && rhs.typ.is_integer())
            }
            // Arithmetic
            Div | Mod | Mult => lhs.typ == rhs.typ && (lhs.typ.is_numeric() || lhs.typ.is_vector()),
            // Bitshifts
            Ashr | Lshr | Shl => {
                lhs.typ.is_integer() && rhs.typ.is_integer()
                    || (lhs.typ == rhs.typ && lhs.typ.is_vector())
            }
            Rol | Ror => lhs.typ.is_integer() && rhs.typ.is_integer(),
            // Boolean ops
            And | Implies | Or | Xor => lhs.typ.is_bool() && rhs.typ.is_bool(),
            // Bitwise ops
            Bitand | Bitor | Bitxor => {
                lhs.typ == rhs.typ && (lhs.typ.is_integer() || lhs.typ.is_vector())
            }
            // Comparisons
            Ge | Gt | Le | Lt => {
                lhs.typ == rhs.typ
                    && (lhs.typ.is_numeric() || lhs.typ.is_pointer() || lhs.typ.is_vector())
            }
            // Equalities
            Equal | Notequal => {
                lhs.typ == rhs.typ
                    && (lhs.typ.is_c_bool()
                        || lhs.typ.is_integer()
                        || lhs.typ.is_pointer()
                        || lhs.typ.is_vector())
            }
            // Floating Point Equalities
            IeeeFloatEqual | IeeeFloatNotequal => lhs.typ == rhs.typ && lhs.typ.is_floating_point(),
            // Overflow flags
            OverflowMinus | OverflowMult | OverflowPlus => {
                lhs.typ == rhs.typ && lhs.typ.is_integer()
            }
        }
    }

    fn binop_return_type(op: BinaryOperand, lhs: &Expr, rhs: &Expr) -> Type {
        match op {
            // Arithmetic which can include pointers
            Minus => {
                if lhs.typ.is_pointer() && rhs.typ.is_pointer() {
                    Type::ssize_t()
                } else {
                    lhs.typ.clone()
                }
            }
            // Arithmetic
            Div | Mod | Mult | Plus => lhs.typ.clone(),
            // Bitshifts
            Ashr | Lshr | Rol | Ror | Shl => lhs.typ.clone(),
            // Boolean ops
            And | Implies | Or | Xor => Type::bool(),
            // Bitwise ops
            Bitand | Bitor | Bitxor => lhs.typ.clone(),
            // Comparisons
            Ge | Gt | Le | Lt => {
                if lhs.typ.is_vector() {
                    lhs.typ.clone()
                } else {
                    Type::bool()
                }
            }
            // Equalities
            Equal | Notequal => {
                if lhs.typ.is_vector() {
                    lhs.typ.clone()
                } else {
                    Type::bool()
                }
            }
            // Floating Point Equalities
            IeeeFloatEqual | IeeeFloatNotequal => Type::bool(),
            // Overflow flags
            OverflowMinus | OverflowMult | OverflowPlus => Type::bool(),
        }
    }
    /// self op right;
    fn binop(self, op: BinaryOperand, rhs: Expr) -> Expr {
        assert!(
            Expr::typecheck_binop_args(op, &self, &rhs),
            "BinaryOperation Expression does not typecheck {:?} {:?} {:?}",
            op,
            self,
            rhs
        );
        expr!(BinOp { op, lhs: self, rhs }, Expr::binop_return_type(op, &self, &rhs))
    }

    /// `__builtin_add_overflow_p(self,e)
    pub fn add_overflow_p(self, e: Expr) -> Expr {
        self.binop(OverflowPlus, e)
    }

    /// `__builtin_sub_overflow_p(self,e)
    pub fn sub_overflow_p(self, e: Expr) -> Expr {
        self.binop(OverflowMinus, e)
    }

    /// `__builtin_mul_overflow_p(self,e)
    pub fn mul_overflow_p(self, e: Expr) -> Expr {
        self.binop(OverflowMult, e)
    }

    /// `self / e`
    pub fn div(self, e: Expr) -> Expr {
        self.binop(Div, e)
    }

    /// `self % e`
    pub fn rem(self, e: Expr) -> Expr {
        self.binop(Mod, e)
    }

    /// `self && e`
    pub fn and(self, e: Expr) -> Expr {
        self.cast_to(Type::bool()).binop(And, e.cast_to(Type::bool()))
    }

    /// `self ==> e`;
    pub fn implies(self, e: Expr) -> Expr {
        self.cast_to(Type::bool()).binop(Implies, e.cast_to(Type::bool()))
    }

    /// `self || e`
    pub fn or(self, e: Expr) -> Expr {
        self.cast_to(Type::bool()).binop(Or, e.cast_to(Type::bool()))
    }

    /// logical xor
    pub fn xor(self, e: Expr) -> Expr {
        self.binop(Xor, e)
    }

    /// `self & e`
    pub fn bitand(self, e: Expr) -> Expr {
        self.binop(Bitand, e)
    }

    /// `self | e`
    pub fn bitor(self, e: Expr) -> Expr {
        self.binop(Bitor, e)
    }

    /// `self ^ e`
    pub fn bitxor(self, e: Expr) -> Expr {
        self.binop(Bitxor, e)
    }

    /// `self << e`
    pub fn shl(self, e: Expr) -> Expr {
        self.binop(Shl, e)
    }

    /// `self >> e` (Signed arithmetic shift)
    pub fn ashr(self, e: Expr) -> Expr {
        self.binop(Ashr, e)
    }

    /// `self >> e` (Unsigned logical shift)
    pub fn lshr(self, e: Expr) -> Expr {
        self.binop(Lshr, e)
    }

    /// `self + e`
    pub fn plus(self, e: Expr) -> Expr {
        self.binop(Plus, e)
    }

    /// `self - e`
    pub fn sub(self, e: Expr) -> Expr {
        self.binop(Minus, e)
    }

    /// self * e
    pub fn mul(self, e: Expr) -> Expr {
        self.binop(Mult, e)
    }

    /// self <= e
    pub fn le(self, e: Expr) -> Expr {
        self.binop(Le, e)
    }

    /// self < e
    pub fn lt(self, e: Expr) -> Expr {
        self.binop(Lt, e)
    }

    /// self >= e
    pub fn ge(self, e: Expr) -> Expr {
        self.binop(Ge, e)
    }

    /// self > e
    pub fn gt(self, e: Expr) -> Expr {
        self.binop(Gt, e)
    }

    /// self : integer == e
    pub fn eq(self, e: Expr) -> Expr {
        self.binop(Equal, e)
    }

    /// self : integer != e
    pub fn neq(self, e: Expr) -> Expr {
        self.binop(Notequal, e)
    }

    /// self : floating point == e
    pub fn feq(self, e: Expr) -> Expr {
        self.binop(IeeeFloatEqual, e)
    }

    /// self : floating point != e
    pub fn fneq(self, e: Expr) -> Expr {
        self.binop(IeeeFloatNotequal, e)
    }

    /// `__builtin_rotateleft(self, e)`
    pub fn rol(self, e: Expr) -> Expr {
        self.binop(Rol, e)
    }

    /// `__builtin_rotateright(self, e)`
    pub fn ror(self, e: Expr) -> Expr {
        self.binop(Ror, e)
    }
}

/// Constructors for self operations
impl Expr {
    /// Private constructor for self operations
    fn self_op(self, op: SelfOperand) -> Expr {
        assert!(self.typ.is_integer() || self.typ.is_pointer());
        expr!(SelfOp { op, e: self }, self.typ.clone())
    }

    /// `self++`
    pub fn postincr(self) -> Expr {
        self.self_op(SelfOperand::Postincrement)
    }

    /// `self--`
    pub fn postdecr(self) -> Expr {
        self.self_op(SelfOperand::Postdecrement)
    }

    /// `++self`
    pub fn preincr(self) -> Expr {
        self.self_op(SelfOperand::Postincrement)
    }

    /// `--self`
    pub fn predecr(self) -> Expr {
        self.self_op(SelfOperand::Postdecrement)
    }
}

/// Constructors for unary operators
impl Expr {
    fn typecheck_unop_arg(op: UnaryOperand, arg: &Expr) -> bool {
        match op {
            Bitnot | Bswap | Popcount => arg.typ.is_integer(),
            CountLeadingZeros { .. } | CountTrailingZeros { .. } => arg.typ.is_integer(),
            IsDynamicObject | ObjectSize | PointerObject => arg.typ().is_pointer(),
            PointerOffset => arg.typ == Type::void_pointer(),
            Not => arg.typ.is_bool(),
            UnaryMinus => arg.typ().is_numeric(),
        }
    }

    fn unop_return_type(op: UnaryOperand, arg: &Expr) -> Type {
        match op {
            Bitnot | Bswap | UnaryMinus => arg.typ.clone(),
            CountLeadingZeros { .. } | CountTrailingZeros { .. } => arg.typ.clone(),
            ObjectSize | PointerObject => Type::size_t(),
            PointerOffset => Type::ssize_t(),
            IsDynamicObject | Not => Type::bool(),
            Popcount => arg.typ.clone(),
        }
    }
    /// Private helper function to make unary operators
    fn unop(self, op: UnaryOperand) -> Expr {
        assert!(Expr::typecheck_unop_arg(op, &self));
        let typ = Expr::unop_return_type(op, &self);
        expr!(ExprValue::UnOp { op, e: self }, typ)
    }

    /// `~self`
    pub fn bitnot(self) -> Expr {
        self.unop(Bitnot)
    }

    ///  `__builtin_bswap<n>(self)`
    pub fn bswap(self) -> Expr {
        self.unop(Bswap)
    }

    /// `__CPROVER_DYNAMIC_OBJECT(self)`
    pub fn dynamic_object(self) -> Self {
        self.unop(IsDynamicObject)
    }

    /// `-self`
    pub fn neg(self) -> Expr {
        self.unop(UnaryMinus)
    }

    /// `!self`
    pub fn not(self) -> Expr {
        self.cast_to(Type::bool()).unop(Not)
    }

    /// `__CPROVER_OBJECT_SIZE(self)`
    pub fn object_size(self) -> Self {
        self.unop(ObjectSize)
    }

    /// `__CPROVER_POINTER_OBJECT(self)`
    pub fn pointer_object(self) -> Self {
        self.unop(PointerObject)
    }

    /// `__CPROVER_POINTER_OFFSET(self)`
    pub fn pointer_offset(self) -> Self {
        self.cast_to(Type::void_pointer()).unop(PointerOffset)
    }

    /// `__builtin_popcount(self)`
    pub fn popcount(self) -> Expr {
        self.unop(Popcount)
    }

    /// `__builtin_cttz(self)`
    /// If `allow_zero == false`, calling this builtin with 0 causes UB
    /// Otherwise it is defined for all values
    pub fn cttz(self, allow_zero: bool) -> Expr {
        self.unop(CountTrailingZeros { allow_zero })
    }

    /// `__builtin_ctlz(self)`
    /// If `allow_zero == false`, calling this builtin with 0 causes UB
    /// Otherwise it is defined for all values
    pub fn ctlz(self, allow_zero: bool) -> Expr {
        self.unop(CountLeadingZeros { allow_zero })
    }
}

/// Compound Expressions
impl Expr {
    /// `self < 0`
    pub fn is_negative(self) -> Self {
        assert!(self.typ.is_numeric());
        let typ = self.typ.clone();
        self.lt(typ.zero())
    }

    /// `self == 0`
    pub fn is_zero(self) -> Self {
        assert!(self.typ.is_numeric());
        let typ = self.typ.clone();
        self.eq(typ.zero())
    }

    /// `self != NULL`
    pub fn is_nonnull(self) -> Self {
        assert!(self.typ.is_pointer());
        let nullptr = self.typ().null();
        self.neq(nullptr)
    }

    /// `ArithmeticOverflowResult r; >>>r.overflowed = builtin_add_overflow(self, e, &r.result)<<<`
    pub fn add_overflow(self, e: Expr) -> ArithmeticOverflowResult {
        let result = self.clone().plus(e.clone());
        let overflowed = self.add_overflow_p(e);
        ArithmeticOverflowResult { result, overflowed }
    }

    /// `&self[0]`. Converts arrays into pointers
    pub fn array_to_ptr(self) -> Self {
        assert!(self.typ().is_array_like());
        self.index_array(Type::ssize_t().zero()).address_of()
    }

    /// `self[i]` where self is a pointer or an array
    pub fn index(self, idx: Expr) -> Self {
        assert!(idx.typ().is_integer());
        if self.typ().is_pointer() {
            self.index_ptr(idx)
        } else if self.typ().is_array_like() {
            self.index_array(idx)
        } else {
            panic!("Can't index: {:?}", self)
        }
    }

    /// `self[i]` where self is a pointer
    pub fn index_ptr(self, idx: Expr) -> Self {
        assert!(idx.typ().is_integer());
        assert!(self.typ().is_pointer());
        self.plus(idx).dereference()
    }

    /// `ArithmeticOverflowResult r; >>>r.overflowed = builtin_sub_overflow(self, e, &r.result)<<<`
    pub fn mul_overflow(self, e: Expr) -> ArithmeticOverflowResult {
        let result = self.clone().mul(e.clone());
        let overflowed = self.mul_overflow_p(e);
        ArithmeticOverflowResult { result, overflowed }
    }

    /// Reinterpret the bits of `self` as being of type `t`.
    /// Note that this differs from standard casts, which may convert values.
    /// in C++ syntax: `(uint32_t)(1.0) == 1`, while `reinterpret_cast<uin32_t>(1.0) == 0x3f800000`
    /// Currently implemented as `*(T *)&self`, which creates a requirement that we can take the
    /// address of `self`.
    /// Unlike `transmute_to`, this "works" on arguments of different sizes.
    pub fn reinterpret_cast(self, t: Type) -> Expr {
        assert!(
            self.can_take_address_of(),
            "Can't take address of {:?} when coercing to {:?}",
            self,
            t
        );
        self.address_of().cast_to(t.to_pointer()).dereference()
    }

    /// `ArithmeticOverflowResult r; >>>r.overflowed = builtin_mul_overflow(self, e, &r.result)<<<`
    pub fn sub_overflow(self, e: Expr) -> ArithmeticOverflowResult {
        let result = self.clone().sub(e.clone());
        let overflowed = self.sub_overflow_p(e);
        ArithmeticOverflowResult { result, overflowed }
    }

    /// `__CPROVER_same_object(self, e)`
    pub fn same_object(self, e: Expr) -> Self {
        self.pointer_object().eq(e.pointer_object())
    }

    /// addition that saturates at bounds
    pub fn saturating_add(self, e: Expr, mm: &MachineModel) -> Expr {
        assert!(self.typ.is_integer());
        assert_eq!(self.typ, e.typ);
        let typ = self.typ.clone();
        let res = self.clone().add_overflow(e.clone());
        // If negative + something overflowed, the something must have been negative too, so we saturate to min.
        // (self < 0) ? min_int : max_int
        let saturating_val = self.is_negative().ternary(typ.min_int_expr(mm), typ.max_int_expr(mm));
        res.overflowed.ternary(saturating_val, res.result)
    }

    /// subtraction that saturates at bounds
    pub fn saturating_sub(self, e: Expr, mm: &MachineModel) -> Expr {
        assert!(self.typ.is_integer());
        assert_eq!(self.typ, e.typ);
        let typ = self.typ.clone();
        let res = self.clone().sub_overflow(e.clone());
        // If something minus a negative overflowed, it must have overflowed past positive max. Saturate there.
        // Otherwise, if something minus a positive overflowed, it must have overflowed to past min. Saturate there.
        let saturating_val = e.is_negative().ternary(typ.max_int_expr(mm), typ.min_int_expr(mm));
        res.overflowed.ternary(saturating_val, res.result)
    }

    /// `"s"`
    /// only to be used when manually wrapped in `.array_to_ptr()`
    pub fn raw_string_constant(s: &str) -> Self {
        expr!(StringConstant { s: s.to_string() }, Type::c_char().array_of(s.len() + 1))
    }

    /// `"s"`
    pub fn string_constant(s: &str) -> Self {
        // Internally, CBMC distinguishes between the string constant, and the pointer to it.
        // The thing we actually manipulate is the pointer, so what is what we return from the constructor.
        // TODO: do we need the `.index(0)` here?
        expr!(StringConstant { s: s.to_string() }, Type::c_char().array_of(s.len() + 1))
            .array_to_ptr()
    }
}
/// Conversions to statements
/// The statement constructors do typechecking, so we don't redundantly do that here.
impl Expr {
    /// `self;`
    pub fn as_stmt(self, loc: Location) -> Stmt {
        Stmt::code_expression(self, loc)
    }

    /// `self = rhs;`
    pub fn assign(self, rhs: Expr, loc: Location) -> Stmt {
        Stmt::assign(self, rhs, loc)
    }

    /// `if (self) { t } else { e }` or `if (self) { t }`
    pub fn if_then_else(self, t: Stmt, e: Option<Stmt>, loc: Location) -> Stmt {
        Stmt::if_then_else(self, t, e, loc)
    }

    /// `return self;`
    pub fn ret(self, loc: Location) -> Stmt {
        Stmt::ret(Some(self), loc)
    }

    /// `switch (self) { cases }`
    pub fn switch(self, cases: Vec<SwitchCase>, default: Option<Stmt>, loc: Location) -> Stmt {
        Stmt::switch(self, cases, default, loc)
    }

    /// `case self: { body }`
    pub fn switch_case(self, body: Stmt) -> SwitchCase {
        SwitchCase::new(self, body)
    }
}

impl Expr {
    /// Given a struct value (Expr), construct a mapping from struct field names
    /// (Strings) to struct field values (Exprs).
    ///
    /// The Struct variant of the Expr enum models the fields of a struct as a
    /// list of pairs (data type components) consisting of a field name and a
    /// field value.  A pair may represent an actual field in the struct or just
    /// padding in the layout of the struct.  This function returns a mapping of
    /// field names (ignoring the padding fields) to field values.  The result
    /// is suitable for use in the struct_expr constructor.  This makes it
    /// easier to look up or modify field values of a struct.
    pub fn struct_field_exprs(&self, symbol_table: &SymbolTable) -> BTreeMap<String, Expr> {
        let struct_type = self.typ();
        assert!(struct_type.is_struct_tag());

        let mut exprs: BTreeMap<String, Expr> = BTreeMap::new();
        let fields = symbol_table.lookup_fields_in_type(struct_type).unwrap();
        match self.struct_expr_values() {
            Some(values) => {
                assert!(fields.len() == values.len());
                for i in 0..fields.len() {
                    if fields[i].is_padding() {
                        continue;
                    }
                    exprs.insert(fields[i].name().to_string(), values[i].clone());
                }
            }
            None => {
                for i in 0..fields.len() {
                    if fields[i].is_padding() {
                        continue;
                    }
                    let name = fields[i].name();
                    exprs.insert(name.to_string(), self.clone().member(name, &symbol_table));
                }
            }
        }
        exprs
    }
}
