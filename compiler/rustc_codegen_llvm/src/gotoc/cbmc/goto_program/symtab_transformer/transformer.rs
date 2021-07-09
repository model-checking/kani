// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::{
    BinaryOperand, CIntType, DatatypeComponent, Expr, ExprValue, Location, Parameter, SelfOperand,
    Stmt, StmtBody, SwitchCase, Symbol, SymbolTable, SymbolValues, Type, UnaryOperand,
};
use crate::btree_map;
use num::bigint::BigInt;
use std::collections::{BTreeMap, HashSet};

/// The `Transformer` trait is a visitor pattern for the `SymbolTable`.
/// To use it, you just need to implement the three symbol table accessor methods,
/// and then override any methods that you want to change the behavior of.
///
/// The entry point is `transform_symbol_table`. It then:
/// - calls `preprocess`
/// - transforms and inserts type symbols
/// - transforms and inserts expr/stmt symbols
/// - calls `postprocess`
///
/// To transform a symbol, we call `transform_type` on its type,
/// and then `transform_value` on its value, which redirects to
/// either `transform_expr` or `transform_stmt`.
///
/// The three methods `transform_type`, `transform_expr`, and `transform_stmt`
/// perform a recursive descent on their corresponding structures.
/// They default to just reconstruct the structure, but can be overridden.
pub trait Transformer: Sized {
    /// Get reference to symbol table.
    fn symbol_table(&self) -> &SymbolTable;
    /// Get mutable reference to symbol table.
    fn mut_symbol_table(&mut self) -> &mut SymbolTable;
    /// Get owned symbol table.
    fn extract_symbol_table(self) -> SymbolTable;

    /// Perform recursive descent on a `Type` data structure.
    /// Extracts the variant's field data, and passes them into
    /// the corresponding type transformer method.
    fn transform_type(&self, typ: &Type) -> Type {
        match typ {
            Type::Array { typ, size } => self.transform_array_type(typ, size),
            Type::Bool => self.transform_bool_type(),
            Type::CBitField { typ, width } => self.transform_c_bit_field_type(typ, width),
            Type::CInteger(c_int_type) => self.transform_c_integer_type(c_int_type),
            Type::Code { parameters, return_type } => {
                self.transform_code_type(parameters, return_type)
            }
            Type::Constructor => self.transform_constructor_type(),
            Type::Double => self.transform_double_type(),
            Type::Empty => self.transform_empty_type(),
            Type::FlexibleArray { typ } => self.transform_flexible_array_type(typ),
            Type::Float => self.transform_float_type(),
            Type::IncompleteStruct { tag } => self.transform_incomplete_struct_type(tag),
            Type::IncompleteUnion { tag } => self.transform_incomplete_union_type(tag),
            Type::InfiniteArray { typ } => self.transform_infinite_array_type(typ),
            Type::Pointer { typ } => self.transform_pointer_type(typ),
            Type::Signedbv { width } => self.transform_signedbv_type(width),
            Type::Struct { tag, components } => self.transform_struct_type(tag, components),
            Type::StructTag(tag) => self.transform_struct_tag_type(tag),
            Type::Union { tag, components } => self.transform_union_type(tag, components),
            Type::UnionTag(tag) => self.transform_union_tag_type(tag),
            Type::Unsignedbv { width } => self.transform_unsignedbv_type(width),
            Type::VariadicCode { parameters, return_type } => {
                self.transform_variadic_code_type(parameters, return_type)
            }
            Type::Vector { typ, size } => self.transform_vector_type(typ, size),
        }
    }

    /// Transforms an array type (`typ x[size]`)
    fn transform_array_type(&self, typ: &Box<Type>, size: &u64) -> Type {
        let transformed_typ = self.transform_type(typ.as_ref());
        transformed_typ.array_of(*size)
    }

    /// Transforms a CPROVER boolean type (`__CPROVER_bool x`)
    fn transform_bool_type(&self) -> Type {
        Type::bool()
    }

    /// Transforms a c bit field type (`typ x : width`)
    fn transform_c_bit_field_type(&self, typ: &Box<Type>, width: &u64) -> Type {
        let transformed_typ = self.transform_type(typ.as_ref());
        transformed_typ.as_bitfield(*width)
    }

    /// Transforms a machine-dependent integer type (`bool`, `char`, `int`, `size_t`)
    fn transform_c_integer_type(&self, c_int_type: &CIntType) -> Type {
        match c_int_type {
            CIntType::Bool => Type::c_bool(),
            CIntType::Char => Type::c_char(),
            CIntType::Int => Type::c_int(),
            CIntType::SizeT => Type::size_t(),
            CIntType::SSizeT => Type::ssize_t(),
        }
    }

    /// Transforms a parameter for a function
    fn transform_parameter(&self, parameter: &Parameter) -> Parameter {
        Type::parameter(
            parameter.identifier().cloned(),
            parameter.base_name().cloned(),
            self.transform_type(parameter.typ()),
        )
    }

    /// Transforms a function type (`return_type x(parameters)`)
    fn transform_code_type(&self, parameters: &[Parameter], return_type: &Box<Type>) -> Type {
        let transformed_parameters =
            parameters.iter().map(|parameter| self.transform_parameter(parameter)).collect();
        let transformed_return_type = self.transform_type(return_type);
        Type::code(transformed_parameters, transformed_return_type)
    }

    /// Transforms a constructor type (`__attribute__(constructor)`)
    fn transform_constructor_type(&self) -> Type {
        Type::constructor()
    }

    /// Transforms a double type (`double`)
    fn transform_double_type(&self) -> Type {
        Type::double()
    }

    /// Transforms an empty type (`void`)
    fn transform_empty_type(&self) -> Type {
        Type::empty()
    }

    /// Transforms a flexible array type (`typ x[]`)
    fn transform_flexible_array_type(&self, typ: &Box<Type>) -> Type {
        let transformed_typ = self.transform_type(typ);
        Type::flexible_array_of(transformed_typ)
    }

    /// Transforms a float type (`float`)
    fn transform_float_type(&self) -> Type {
        Type::float()
    }

    /// Transforms an incomplete struct type (`struct x {}`)
    fn transform_incomplete_struct_type(&self, tag: &str) -> Type {
        Type::incomplete_struct(tag)
    }

    /// Transforms an incomplete union type (`union x {}`)
    fn transform_incomplete_union_type(&self, tag: &str) -> Type {
        Type::incomplete_union(tag)
    }

    /// Transforms an infinite array type (`typ x[__CPROVER_infinity()]`)
    fn transform_infinite_array_type(&self, typ: &Box<Type>) -> Type {
        let transformed_typ = self.transform_type(typ.as_ref());
        transformed_typ.infinite_array_of()
    }

    /// Transforms a pointer type (`typ*`)
    fn transform_pointer_type(&self, typ: &Box<Type>) -> Type {
        let transformed_typ = self.transform_type(typ.as_ref());
        transformed_typ.to_pointer()
    }

    /// Transforms a signed bitvector type (`int<width>_t`)
    fn transform_signedbv_type(&self, width: &u64) -> Type {
        Type::signed_int(*width)
    }

    /// Transforms a datatype component
    fn transform_datatype_component(&self, component: &DatatypeComponent) -> DatatypeComponent {
        match component {
            DatatypeComponent::Field { name, typ } => {
                DatatypeComponent::Field { name: name.to_string(), typ: self.transform_type(typ) }
            }
            DatatypeComponent::Padding { name, bits } => {
                DatatypeComponent::Padding { name: name.to_string(), bits: *bits }
            }
        }
    }

    /// Transforms a struct type (`struct tag {component1.typ component1.name; component2.typ component2.name ... }`)
    fn transform_struct_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(tag, transformed_components)
    }

    /// Transforms a struct tag type (`tag-<tag>`)
    fn transform_struct_tag_type(&self, tag: &str) -> Type {
        Type::struct_tag_raw(tag)
    }

    /// Transforms a union type (`union tag {component1.typ component1.name; component2.typ component2.name ... }`)
    fn transform_union_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(tag, transformed_components)
    }

    /// Transforms a uniont tag type (`tag-<tag>`)
    fn transform_union_tag_type(&self, tag: &str) -> Type {
        Type::union_tag_raw(tag)
    }

    /// Transforms an unsigned bitvector type (`uint<width>_t`)
    fn transform_unsignedbv_type(&self, width: &u64) -> Type {
        Type::unsigned_int(*width)
    }

    /// Transforms a variadic function type (`return_type x(parameters, ...)`)
    fn transform_variadic_code_type(
        &self,
        parameters: &[Parameter],
        return_type: &Box<Type>,
    ) -> Type {
        let transformed_parameters =
            parameters.iter().map(|parameter| self.transform_parameter(parameter)).collect();
        let transformed_return_type = self.transform_type(return_type.as_ref());
        Type::variadic_code(transformed_parameters, transformed_return_type)
    }

    /// Transforms a vector type (`typ __attribute__((vector_size (size * sizeof(typ)))) var;`)
    fn transform_vector_type(&self, typ: &Box<Type>, size: &u64) -> Type {
        let transformed_typ = self.transform_type(typ.as_ref());
        Type::vector(transformed_typ, *size)
    }

    /// Perform recursive descent on a `Expr` data structure.
    /// Extracts the variant's field data, and passes them into
    /// the corresponding expr transformer method along with the expr type.
    fn transform_expr(&self, e: &Expr) -> Expr {
        let typ = e.typ();
        match e.value() {
            ExprValue::AddressOf(child) => self.transform_address_of_expr(typ, child),
            ExprValue::Array { elems } => self.transform_array_expr(typ, elems),
            ExprValue::ArrayOf { elem } => self.transform_array_of_expr(typ, elem),
            ExprValue::Assign { left, right } => self.transform_assign_expr(typ, left, right),
            ExprValue::BinOp { op, lhs, rhs } => self.transform_bin_op_expr(typ, op, lhs, rhs),
            ExprValue::BoolConstant(value) => self.transform_bool_constant_expr(typ, value),
            ExprValue::ByteExtract { e, offset } => {
                self.transform_byte_extract_expr(typ, e, offset)
            }
            ExprValue::CBoolConstant(value) => self.transform_c_bool_constant_expr(typ, value),
            ExprValue::Dereference(child) => self.transform_dereference_expr(typ, child),
            ExprValue::DoubleConstant(value) => self.transform_double_constant_expr(typ, value),
            ExprValue::FloatConstant(value) => self.transform_float_constant_expr(typ, value),
            ExprValue::FunctionCall { function, arguments } => {
                self.transform_function_call_expr(typ, function, arguments)
            }
            ExprValue::If { c, t, e } => self.transform_if_expr(typ, c, t, e),
            ExprValue::Index { array, index } => self.transform_index_expr(typ, array, index),
            ExprValue::IntConstant(value) => self.transform_int_constant_expr(typ, value),
            ExprValue::Member { lhs, field } => self.transform_member_expr(typ, lhs, field),
            ExprValue::Nondet => self.transform_nondet_expr(typ),
            ExprValue::PointerConstant(value) => self.transform_pointer_constant_expr(typ, value),
            ExprValue::SelfOp { op, e } => self.transform_self_op_expr(typ, op, e),
            ExprValue::StatementExpression { statements } => {
                self.transform_statement_expression_expr(typ, statements)
            }
            ExprValue::StringConstant { s } => self.transform_string_constant_expr(typ, s),
            ExprValue::Struct { values } => self.transform_struct_expr(typ, values),
            ExprValue::Symbol { identifier } => self.transform_symbol_expr(typ, identifier),
            ExprValue::Typecast(child) => self.transform_typecast_expr(typ, child),
            ExprValue::Union { value, field } => self.transform_union_expr(typ, value, field),
            ExprValue::UnOp { op, e } => self.transform_un_op_expr(typ, op, e),
        }
        .with_location(e.location().clone())
    }

    /// Transforms a reference expr (`&self`)
    fn transform_address_of_expr(&self, _typ: &Type, child: &Expr) -> Expr {
        self.transform_expr(child).address_of()
    }

    /// Transform an array expr (`typ x[] = >>> {elems0, elems1 ...} <<<`)
    fn transform_array_expr(&self, typ: &Type, elems: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_elems = elems.iter().map(|elem| self.transform_expr(elem)).collect();
        Expr::array_expr(transformed_typ, transformed_elems)
    }

    /// Transforms an array of expr (`typ x[width] = >>> {elem} <<<`)
    fn transform_array_of_expr(&self, typ: &Type, elem: &Expr) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_elem = self.transform_expr(elem);
        if let Type::Array { typ: _typ, size } = transformed_typ {
            transformed_elem.array_constant(size)
        } else {
            unreachable!()
        }
    }

    /// Transform an assign expr (`left == right`)
    /// Currently not able to be constructed, as does not exist in Rust
    fn transform_assign_expr(&self, _typ: &Type, _left: &Expr, _right: &Expr) -> Expr {
        unreachable!()
    }

    /// Transform a binary operation expr (`lhs op rhs`)
    fn transform_bin_op_expr(
        &self,
        _typ: &Type,
        op: &BinaryOperand,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Expr {
        let lhs = self.transform_expr(lhs);
        let rhs = self.transform_expr(rhs);

        match op {
            BinaryOperand::And => lhs.and(rhs),
            BinaryOperand::Ashr => lhs.ashr(rhs),
            BinaryOperand::Bitand => lhs.bitand(rhs),
            BinaryOperand::Bitor => lhs.bitor(rhs),
            BinaryOperand::Bitxor => lhs.bitxor(rhs),
            BinaryOperand::Div => lhs.div(rhs),
            BinaryOperand::Equal => lhs.eq(rhs),
            BinaryOperand::Ge => lhs.ge(rhs),
            BinaryOperand::Gt => lhs.gt(rhs),
            BinaryOperand::IeeeFloatEqual => lhs.feq(rhs),
            BinaryOperand::IeeeFloatNotequal => lhs.fneq(rhs),
            BinaryOperand::Implies => lhs.implies(rhs),
            BinaryOperand::Le => lhs.le(rhs),
            BinaryOperand::Lshr => lhs.lshr(rhs),
            BinaryOperand::Lt => lhs.lt(rhs),
            BinaryOperand::Minus => lhs.sub(rhs),
            BinaryOperand::Mod => lhs.rem(rhs),
            BinaryOperand::Mult => lhs.mul(rhs),
            BinaryOperand::Notequal => lhs.neq(rhs),
            BinaryOperand::Or => lhs.or(rhs),
            BinaryOperand::OverflowMinus => lhs.sub_overflow_p(rhs),
            BinaryOperand::OverflowMult => lhs.mul_overflow_p(rhs),
            BinaryOperand::OverflowPlus => lhs.add_overflow_p(rhs),
            BinaryOperand::Plus => lhs.plus(rhs),
            BinaryOperand::Rol => lhs.rol(rhs),
            BinaryOperand::Ror => lhs.ror(rhs),
            BinaryOperand::Shl => lhs.shl(rhs),
            BinaryOperand::Xor => lhs.xor(rhs),
        }
    }

    /// Transforms a CPROVER boolean expression (`(__CPROVER_bool) >>> true/false <<<`)
    fn transform_bool_constant_expr(&self, _typ: &Type, value: &bool) -> Expr {
        Expr::bool_constant(*value)
    }

    /// Transforms a byte extraction expr (e as type self.typ)
    fn transform_byte_extract_expr(&self, typ: &Type, e: &Expr, _offset: &u64) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_e = self.transform_expr(e);
        transformed_e.transmute_to(transformed_typ, self.symbol_table())
    }

    /// Transforms a C boolean constant expr (`(bool) 1`)
    fn transform_c_bool_constant_expr(&self, _typ: &Type, value: &bool) -> Expr {
        Expr::c_bool_constant(*value)
    }

    /// Transforms a deref expr (`*self`)
    fn transform_dereference_expr(&self, _typ: &Type, child: &Expr) -> Expr {
        let transformed_child = self.transform_expr(child);
        transformed_child.dereference()
    }

    /// Transforms a double constant expr (`1.0`)
    fn transform_double_constant_expr(&self, _typ: &Type, value: &f64) -> Expr {
        Expr::double_constant(*value)
    }

    /// Transforms a float constant expr (`1.0f`)
    fn transform_float_constant_expr(&self, _typ: &Type, value: &f32) -> Expr {
        Expr::float_constant(*value)
    }

    /// Transforms a function call expr (`function(arguments)`)
    fn transform_function_call_expr(
        &self,
        _typ: &Type,
        function: &Expr,
        arguments: &[Expr],
    ) -> Expr {
        let transformed_function = self.transform_expr(function);
        let transformed_arguments =
            arguments.iter().map(|argument| self.transform_expr(argument)).collect();
        transformed_function.call(transformed_arguments)
    }

    /// Transforms an if expr (`c ? t : e`)
    fn transform_if_expr(&self, _typ: &Type, c: &Expr, t: &Expr, e: &Expr) -> Expr {
        let transformed_c = self.transform_expr(c);
        let transformed_t = self.transform_expr(t);
        let transformed_e = self.transform_expr(e);
        transformed_c.ternary(transformed_t, transformed_e)
    }

    /// Transforms an array index expr (`array[expr]`)
    fn transform_index_expr(&self, _typ: &Type, array: &Expr, index: &Expr) -> Expr {
        let transformed_array = self.transform_expr(array);
        let transformed_index = self.transform_expr(index);
        transformed_array.index(transformed_index)
    }

    /// Transforms an int constant expr (`123`)
    fn transform_int_constant_expr(&self, typ: &Type, value: &BigInt) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::int_constant(value.clone(), transformed_typ)
    }

    /// Transforms a member access expr (`lhs.field`)
    fn transform_member_expr(&self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(field, self.symbol_table())
    }

    /// Transforms a CPROVER nondet call (`__nondet()`)
    fn transform_nondet_expr(&self, typ: &Type) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::nondet(transformed_typ)
    }

    /// Transforms a pointer constant expr (`NULL`)
    fn transform_pointer_constant_expr(&self, typ: &Type, value: &u64) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::pointer_constant(*value, transformed_typ)
    }

    /// Transforms a self-op expr (`op++`, etc.)
    fn transform_self_op_expr(&self, _typ: &Type, op: &SelfOperand, e: &Expr) -> Expr {
        let transformed_e = self.transform_expr(e);
        match op {
            SelfOperand::Postdecrement => transformed_e.postdecr(),
            SelfOperand::Postincrement => transformed_e.postincr(),
            SelfOperand::Predecrement => transformed_e.predecr(),
            SelfOperand::Preincrement => transformed_e.preincr(),
        }
    }

    /// Transforms a statement expr (({ stmt1; stmt2; ...}))
    fn transform_statement_expression_expr(&self, typ: &Type, statements: &[Stmt]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_statements =
            statements.iter().map(|stmt| self.transform_stmt(stmt)).collect();
        Expr::statement_expression(transformed_statements, transformed_typ)
    }

    /// Transforms a string constant expr (`"s"`)
    fn transform_string_constant_expr(&self, _typ: &Type, value: &str) -> Expr {
        Expr::raw_string_constant(value)
    }

    /// Transforms a struct initializer expr (`struct foo the_foo = >>> {field1, field2, ... } <<<`)
    fn transform_struct_expr(&self, typ: &Type, values: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        assert!(
            transformed_typ.is_struct_tag(),
            "Transformed StructTag must be StructTag; got {:?}",
            transformed_typ
        );
        let transformed_values: Vec<_> =
            values.into_iter().map(|value| self.transform_expr(value)).collect();
        Expr::struct_expr_from_padded_values(
            transformed_typ,
            transformed_values,
            self.symbol_table(),
        )
    }

    /// Transforms a symbol expr (`self`)
    fn transform_symbol_expr(&self, typ: &Type, identifier: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(identifier.to_string(), transformed_typ)
    }

    /// Transforms a typecast expr (`(typ) self`)
    fn transform_typecast_expr(&self, typ: &Type, child: &Expr) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_child = self.transform_expr(child);
        transformed_child.cast_to(transformed_typ)
    }

    /// Transforms a union initializer expr (`union foo the_foo = >>> {.field = value } <<<`)
    fn transform_union_expr(&self, typ: &Type, value: &Expr, field: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(transformed_typ, field, transformed_value, self.symbol_table())
    }

    /// Transforms a unary operator expr (`op self`)
    fn transform_un_op_expr(&self, _typ: &Type, op: &UnaryOperand, e: &Expr) -> Expr {
        let transformed_e = self.transform_expr(e);
        match op {
            UnaryOperand::Bitnot => transformed_e.bitnot(),
            UnaryOperand::Bswap => transformed_e.bswap(),
            UnaryOperand::IsDynamicObject => transformed_e.dynamic_object(),
            UnaryOperand::Not => transformed_e.not(),
            UnaryOperand::ObjectSize => transformed_e.object_size(),
            UnaryOperand::PointerObject => transformed_e.pointer_object(),
            UnaryOperand::PointerOffset => transformed_e.pointer_offset(),
            UnaryOperand::Popcount => transformed_e.popcount(),
            UnaryOperand::CountTrailingZeros { allow_zero } => transformed_e.cttz(*allow_zero),
            UnaryOperand::CountLeadingZeros { allow_zero } => transformed_e.ctlz(*allow_zero),
            UnaryOperand::UnaryMinus => transformed_e.neg(),
        }
    }

    /// Perform recursive descent on a `Stmt` data structure.
    /// Extracts the variant's field data, and passes them into
    /// the corresponding stmt transformer method.
    fn transform_stmt(&self, stmt: &Stmt) -> Stmt {
        match stmt.body() {
            StmtBody::Assign { lhs, rhs } => self.transform_assign_stmt(lhs, rhs),
            StmtBody::Assume { cond } => self.transform_assume_stmt(cond),
            StmtBody::AtomicBlock(block) => self.transform_atomic_block_stmt(block),
            StmtBody::Block(block) => self.transform_block_stmt(block),
            StmtBody::Break => self.transform_break_stmt(),
            StmtBody::Continue => self.transform_continue_stmt(),
            StmtBody::Decl { lhs, value } => self.transform_decl_stmt(lhs, value),
            StmtBody::Expression(expr) => self.transform_expression_stmt(expr),
            StmtBody::For { init, cond, update, body } => {
                self.transform_for_stmt(init, cond, update, body)
            }
            StmtBody::FunctionCall { lhs, function, arguments } => {
                self.transform_function_call_stmt(lhs, function, arguments)
            }
            StmtBody::Goto(label) => self.transform_goto_stmt(label),
            StmtBody::Ifthenelse { i, t, e } => self.transform_ifthenelse_stmt(i, t, e),
            StmtBody::Label { label, body } => self.transform_label_stmt(label, body),
            StmtBody::Return(value) => self.transform_return_stmt(value),
            StmtBody::Skip => self.transform_skip_stmt(),
            StmtBody::Switch { control, cases, default } => {
                self.transform_switch_stmt(control, cases, default)
            }
            StmtBody::While { cond, body } => self.transform_while_stmt(cond, body),
        }
        .with_location(stmt.location().clone())
    }

    /// Transforms an assign stmt (`lhs = rhs;`)
    fn transform_assign_stmt(&self, lhs: &Expr, rhs: &Expr) -> Stmt {
        let transformed_lhs = self.transform_expr(lhs);
        let transformed_rhs = self.transform_expr(rhs);
        transformed_lhs.assign(transformed_rhs, Location::none())
    }

    /// Transforms a CPROVER assume stmt (`__CPROVER_assume(cond);`)
    fn transform_assume_stmt(&self, cond: &Expr) -> Stmt {
        let transformed_cond = self.transform_expr(cond);
        Stmt::assume(transformed_cond, Location::none())
    }

    /// Transforms an atomic block stmt (`{ ATOMIC_BEGIN stmt1; stmt2; ... ATOMIC_END }`)
    fn transform_atomic_block_stmt(&self, block: &[Stmt]) -> Stmt {
        let transformed_block = block.iter().map(|stmt| self.transform_stmt(stmt)).collect();
        Stmt::atomic_block(transformed_block, Location::none())
    }

    /// Transforms a block stmt (`{ stmt1; stmt2; ... }`)
    fn transform_block_stmt(&self, block: &[Stmt]) -> Stmt {
        let transformed_block = block.iter().map(|stmt| self.transform_stmt(stmt)).collect();
        Stmt::block(transformed_block, Location::none())
    }

    /// Transform a break stmt (`break;`)
    fn transform_break_stmt(&self) -> Stmt {
        Stmt::break_stmt(Location::none())
    }

    /// Transform a continue stmt (`continue;`)
    fn transform_continue_stmt(&self) -> Stmt {
        Stmt::continue_stmt(Location::none())
    }

    /// Transform a decl stmt (`lhs.typ lhs = value;` or `lhs.typ lhs;`)
    fn transform_decl_stmt(&self, lhs: &Expr, value: &Option<Expr>) -> Stmt {
        let transformed_lhs = self.transform_expr(lhs);
        let transformed_value = value.as_ref().map(|value| self.transform_expr(value));
        Stmt::decl(transformed_lhs, transformed_value, Location::none())
    }

    /// Transform an expression stmt (`e;`)
    fn transform_expression_stmt(&self, expr: &Expr) -> Stmt {
        let transformed_expr = self.transform_expr(expr);
        transformed_expr.as_stmt(Location::none())
    }

    /// Transform a for loop stmt (`for (init; cond; update) {body}`)
    fn transform_for_stmt(&self, init: &Stmt, cond: &Expr, update: &Stmt, body: &Stmt) -> Stmt {
        let transformed_init = self.transform_stmt(init);
        let transformed_cond = self.transform_expr(cond);
        let transformed_update = self.transform_stmt(update);
        let transformed_body = self.transform_stmt(body);

        Stmt::for_loop(
            transformed_init,
            transformed_cond,
            transformed_update,
            transformed_body,
            Location::none(),
        )
    }

    /// Transforms a function call stmt (`lhs = function(arguments);` or `function(arguments);`)
    fn transform_function_call_stmt(
        &self,
        lhs: &Option<Expr>,
        function: &Expr,
        arguments: &[Expr],
    ) -> Stmt {
        let transformed_lhs = lhs.as_ref().map(|lhs| self.transform_expr(lhs));
        let transformed_function = self.transform_expr(function);
        let transformed_arguments =
            arguments.iter().map(|argument| self.transform_expr(argument)).collect();
        Stmt::function_call(
            transformed_lhs,
            transformed_function,
            transformed_arguments,
            Location::none(),
        )
    }

    /// Transforms a goto stmt (`goto dest;`)
    fn transform_goto_stmt(&self, label: &str) -> Stmt {
        Stmt::goto(label.to_string(), Location::none())
    }

    /// Transforms an if-then-else stmt (`if (i) { t } else { e }`)
    fn transform_ifthenelse_stmt(&self, i: &Expr, t: &Stmt, e: &Option<Stmt>) -> Stmt {
        let transformed_i = self.transform_expr(i);
        let transformed_t = self.transform_stmt(t);
        let transformed_e = e.as_ref().map(|e| self.transform_stmt(e));

        Stmt::if_then_else(transformed_i, transformed_t, transformed_e, Location::none())
    }

    /// Transforms a label stmt (`label: body`)
    fn transform_label_stmt(&self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(label.to_string())
    }

    /// Transforms a return stmt (`return e;` or `return;`)
    fn transform_return_stmt(&self, value: &Option<Expr>) -> Stmt {
        let transformed_value = value.as_ref().map(|value| self.transform_expr(value));
        Stmt::ret(transformed_value, Location::none())
    }

    /// Transforms a skip stmt (`;`)
    fn transform_skip_stmt(&self) -> Stmt {
        Stmt::skip(Location::none())
    }

    /// Transforms a switch stmt (`switch (control) { case1.case: cast1.body; case2.case: case2.body; ... }`)
    fn transform_switch_stmt(
        &self,
        control: &Expr,
        cases: &[SwitchCase],
        default: &Option<Stmt>,
    ) -> Stmt {
        let transformed_control = self.transform_expr(control);
        let transformed_cases = cases
            .iter()
            .map(|case| {
                SwitchCase::new(self.transform_expr(case.case()), self.transform_stmt(case.body()))
            })
            .collect();
        let transformed_default = default.as_ref().map(|default| self.transform_stmt(default));

        Stmt::switch(transformed_control, transformed_cases, transformed_default, Location::none())
    }

    /// Transforms a while loop stmt (`while (cond) { body }`)
    fn transform_while_stmt(&self, cond: &Expr, body: &Stmt) -> Stmt {
        let transformed_cond = self.transform_expr(cond);
        let transformed_body = self.transform_stmt(body);
        Stmt::while_loop(transformed_cond, transformed_body, Location::none())
    }

    /// Transforms a symbol's type and value
    fn transform_symbol(&self, symbol: &Symbol) -> Symbol {
        let new_typ = self.transform_type(&symbol.typ);
        let new_value = self.transform_value(&symbol.value);
        let mut new_symbol = symbol.clone();
        new_symbol.value = new_value;
        new_symbol.typ = new_typ;
        new_symbol
    }

    /// Transforms a symbol value
    fn transform_value(&self, value: &SymbolValues) -> SymbolValues {
        match value {
            SymbolValues::None => SymbolValues::None,
            SymbolValues::Expr(expr) => SymbolValues::Expr(self.transform_expr(expr)),
            SymbolValues::Stmt(stmt) => SymbolValues::Stmt(self.transform_stmt(stmt)),
        }
    }

    /// Preprocessing to perform before adding transformed symbols
    fn preprocess(&mut self) {}

    /// Postprocessing to perform after adding transformed symbols
    fn postprocess(&mut self) {}

    /// Transforms the orig_symtab, producing a new one.
    /// See `Transformer` trait documentation for details.
    fn transform_symbol_table(mut self, orig_symtab: &SymbolTable) -> SymbolTable {
        self.preprocess();

        let mut added: HashSet<String> = HashSet::new();

        for (name, _symbol) in self.symbol_table().iter() {
            added.insert(name.clone());
        }

        for (name, symbol) in orig_symtab.iter() {
            if !self.symbol_table().contains(name) && symbol.value.is_none() {
                let new_symbol = self.transform_symbol(symbol);
                self.mut_symbol_table().insert(new_symbol);
                added.insert(name.clone());
            }
        }

        for (name, symbol) in orig_symtab.iter() {
            if !added.contains(name) {
                assert!(
                    !symbol.value.is_none(),
                    "Symbol should have been inserted in first pass: {:?}",
                    symbol
                );
                let new_symbol = self.transform_symbol(symbol);
                self.mut_symbol_table().insert(new_symbol);
            }
        }

        self.postprocess();

        self.extract_symbol_table()
    }
}
