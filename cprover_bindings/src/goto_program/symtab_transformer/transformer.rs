// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::goto_program::{
    BinaryOperator, CIntType, DatatypeComponent, Expr, ExprValue, Location, Parameter,
    SelfOperator, Stmt, StmtBody, SwitchCase, Symbol, SymbolTable, SymbolValues, Type,
    UnaryOperator,
};
use crate::InternedString;
use num::bigint::BigInt;
use std::collections::HashSet;

/// The `Transformer` trait is a visitor pattern for the `SymbolTable`.
/// To use it, you just need to implement the three symbol table accessor methods,
/// and then override any methods that you want to change the behavior of.
///
/// The entry point is `transform_symbol_table`. The transformer then:
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
    fn transform_type(&mut self, typ: &Type) -> Type {
        match typ {
            Type::Array { typ, size } => self.transform_type_array(typ, size),
            Type::Bool => self.transform_type_bool(),
            Type::CBitField { typ, width } => self.transform_type_c_bit_field(typ, width),
            Type::CInteger(c_int_type) => self.transform_type_c_integer(c_int_type),
            Type::Code { parameters, return_type } => {
                self.transform_type_code(parameters, return_type)
            }
            Type::Constructor => self.transform_type_constructor(),
            Type::Double => self.transform_type_double(),
            Type::Empty => self.transform_type_empty(),
            Type::FlexibleArray { typ } => self.transform_type_flexible_array(typ),
            Type::Float => self.transform_type_float(),
            Type::IncompleteStruct { tag } => self.transform_type_incomplete_struct(*tag),
            Type::IncompleteUnion { tag } => self.transform_type_incomplete_union(*tag),
            Type::InfiniteArray { typ } => self.transform_type_infinite_array(typ),
            Type::Pointer { typ } => self.transform_type_pointer(typ),
            Type::Signedbv { width } => self.transform_type_signedbv(width),
            Type::Struct { tag, components } => self.transform_type_struct(*tag, components),
            Type::StructTag(tag) => self.transform_type_struct_tag(*tag),
            Type::TypeDef { name: tag, typ } => self.transform_type_def(*tag, typ),
            Type::Union { tag, components } => self.transform_type_union(*tag, components),
            Type::UnionTag(tag) => self.transform_type_union_tag(*tag),
            Type::Unsignedbv { width } => self.transform_type_unsignedbv(width),
            Type::VariadicCode { parameters, return_type } => {
                self.transform_type_variadic_code(parameters, return_type)
            }
            Type::Vector { typ, size } => self.transform_type_vector(typ, size),
        }
    }

    /// Transforms an array type (`typ x[size]`)
    fn transform_type_array(&mut self, typ: &Type, size: &u64) -> Type {
        let transformed_typ = self.transform_type(typ);
        transformed_typ.array_of(*size)
    }

    /// Transforms a CPROVER boolean type (`__CPROVER_bool x`)
    fn transform_type_bool(&mut self) -> Type {
        Type::bool()
    }

    /// Transforms a c bit field type (`typ x : width`)
    fn transform_type_c_bit_field(&mut self, typ: &Type, width: &u64) -> Type {
        let transformed_typ = self.transform_type(typ);
        transformed_typ.as_bitfield(*width)
    }

    /// Transforms a machine-dependent integer type (`bool`, `char`, `int`, `size_t`)
    fn transform_type_c_integer(&mut self, c_int_type: &CIntType) -> Type {
        match c_int_type {
            CIntType::Bool => Type::c_bool(),
            CIntType::Char => Type::c_char(),
            CIntType::Int => Type::c_int(),
            CIntType::SizeT => Type::size_t(),
            CIntType::SSizeT => Type::ssize_t(),
        }
    }

    /// Transforms a parameter for a function
    fn transform_type_parameter(&mut self, parameter: &Parameter) -> Parameter {
        self.transform_type(parameter.typ())
            .as_parameter(parameter.identifier(), parameter.base_name())
    }

    /// Transforms a function type (`return_type x(parameters)`)
    fn transform_type_code(&mut self, parameters: &[Parameter], return_type: &Type) -> Type {
        let transformed_parameters =
            parameters.iter().map(|parameter| self.transform_type_parameter(parameter)).collect();
        let transformed_return_type = self.transform_type(return_type);
        Type::code(transformed_parameters, transformed_return_type)
    }

    /// Transforms a constructor type (`__attribute__(constructor)`)
    fn transform_type_constructor(&mut self) -> Type {
        Type::constructor()
    }

    /// Transforms a double type (`double`)
    fn transform_type_double(&mut self) -> Type {
        Type::double()
    }

    /// Transforms an empty type (`void`)
    fn transform_type_empty(&mut self) -> Type {
        Type::empty()
    }

    /// Transforms a flexible array type (`typ x[]`)
    fn transform_type_flexible_array(&mut self, typ: &Type) -> Type {
        let transformed_typ = self.transform_type(typ);
        Type::flexible_array_of(transformed_typ)
    }

    /// Transforms a float type (`float`)
    fn transform_type_float(&mut self) -> Type {
        Type::float()
    }

    /// Transforms an incomplete struct type (`struct x {}`)
    fn transform_type_incomplete_struct(&mut self, tag: InternedString) -> Type {
        Type::incomplete_struct(tag)
    }

    /// Transforms an incomplete union type (`union x {}`)
    fn transform_type_incomplete_union(&mut self, tag: InternedString) -> Type {
        Type::incomplete_union(tag)
    }

    /// Transforms an infinite array type (`typ x[__CPROVER_infinity()]`)
    fn transform_type_infinite_array(&mut self, typ: &Type) -> Type {
        let transformed_typ = self.transform_type(typ);
        transformed_typ.infinite_array_of()
    }

    /// Transforms a pointer type (`typ*`)
    fn transform_type_pointer(&mut self, typ: &Type) -> Type {
        let transformed_typ = self.transform_type(typ);
        transformed_typ.to_pointer()
    }

    /// Transforms a signed bitvector type (`int<width>_t`)
    fn transform_type_signedbv(&mut self, width: &u64) -> Type {
        Type::signed_int(*width)
    }

    /// Transforms a datatype component
    fn transform_datatype_component(&mut self, component: &DatatypeComponent) -> DatatypeComponent {
        match component {
            DatatypeComponent::Field { name, typ } => {
                DatatypeComponent::field(*name, self.transform_type(typ))
            }
            DatatypeComponent::Padding { name, bits } => DatatypeComponent::padding(*name, *bits),
        }
    }

    /// Transforms a struct type (`struct tag {component1.typ component1.name; component2.typ component2.name ... }`)
    fn transform_type_struct(
        &mut self,
        tag: InternedString,
        components: &[DatatypeComponent],
    ) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(tag, transformed_components)
    }

    /// Transforms a struct tag type (`tag-<tag>`)
    fn transform_type_struct_tag(&mut self, tag: InternedString) -> Type {
        Type::struct_tag_raw(tag)
    }

    /// Transforms a union type (`union tag {component1.typ component1.name; component2.typ component2.name ... }`)
    fn transform_type_union(
        &mut self,
        tag: InternedString,
        components: &[DatatypeComponent],
    ) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(tag, transformed_components)
    }

    /// Transforms a union tag type (`tag-<tag>`)
    fn transform_type_union_tag(&mut self, tag: InternedString) -> Type {
        Type::union_tag_raw(tag)
    }

    /// Transforms an unsigned bitvector type (`uint<width>_t`)
    fn transform_type_unsignedbv(&mut self, width: &u64) -> Type {
        Type::unsigned_int(*width)
    }

    /// Transforms a variadic function type (`return_type x(parameters, ...)`)
    fn transform_type_variadic_code(
        &mut self,
        parameters: &[Parameter],
        return_type: &Type,
    ) -> Type {
        let transformed_parameters =
            parameters.iter().map(|parameter| self.transform_type_parameter(parameter)).collect();
        let transformed_return_type = self.transform_type(return_type);
        Type::variadic_code(transformed_parameters, transformed_return_type)
    }

    /// Transforms a vector type (`typ __attribute__((vector_size (size * sizeof(typ)))) var;`)
    fn transform_type_vector(&mut self, typ: &Type, size: &u64) -> Type {
        let transformed_typ = self.transform_type(typ);
        Type::vector(transformed_typ, *size)
    }

    /// Transforms a type def (`typedef typ tag`)
    fn transform_type_def(&mut self, tag: InternedString, typ: &Type) -> Type {
        let transformed_typ = self.transform_type(typ);
        transformed_typ.to_typedef(tag)
    }

    /// Perform recursive descent on a `Expr` data structure.
    /// Extracts the variant's field data, and passes them into
    /// the corresponding expr transformer method along with the expr type.
    fn transform_expr(&mut self, e: &Expr) -> Expr {
        let typ = e.typ();
        match e.value() {
            ExprValue::AddressOf(child) => self.transform_expr_address_of(typ, child),
            ExprValue::Array { elems } => self.transform_expr_array(typ, elems),
            ExprValue::ArrayOf { elem } => self.transform_expr_array_of(typ, elem),
            ExprValue::Assign { left, right } => self.transform_expr_assign(typ, left, right),
            ExprValue::BinOp { op, lhs, rhs } => self.transform_expr_bin_op(typ, op, lhs, rhs),
            ExprValue::BoolConstant(value) => self.transform_expr_bool_constant(typ, value),
            ExprValue::ByteExtract { e, offset } => {
                self.transform_expr_byte_extract(typ, e, offset)
            }
            ExprValue::CBoolConstant(value) => self.transform_expr_c_bool_constant(typ, value),
            ExprValue::Dereference(child) => self.transform_expr_dereference(typ, child),
            ExprValue::DoubleConstant(value) => self.transform_expr_double_constant(typ, value),
            ExprValue::FloatConstant(value) => self.transform_expr_float_constant(typ, value),
            ExprValue::FunctionCall { function, arguments } => {
                self.transform_expr_function_call(typ, function, arguments)
            }
            ExprValue::If { c, t, e } => self.transform_expr_if(typ, c, t, e),
            ExprValue::Index { array, index } => self.transform_expr_index(typ, array, index),
            ExprValue::IntConstant(value) => self.transform_expr_int_constant(typ, value),
            ExprValue::Member { lhs, field } => self.transform_expr_member(typ, lhs, *field),
            ExprValue::Nondet => self.transform_expr_nondet(typ),
            ExprValue::PointerConstant(value) => self.transform_expr_pointer_constant(typ, value),
            ExprValue::SelfOp { op, e } => self.transform_expr_self_op(typ, op, e),
            ExprValue::StatementExpression { statements } => {
                self.transform_expr_statement_expression(typ, statements)
            }
            ExprValue::StringConstant { s } => self.transform_expr_string_constant(typ, *s),
            ExprValue::Struct { values } => self.transform_expr_struct(typ, values),
            ExprValue::Symbol { identifier } => self.transform_expr_symbol(typ, *identifier),
            ExprValue::Typecast(child) => self.transform_expr_typecast(typ, child),
            ExprValue::Union { value, field } => self.transform_expr_union(typ, value, *field),
            ExprValue::UnOp { op, e } => self.transform_expr_un_op(typ, op, e),
            ExprValue::Vector { elems } => self.transform_expr_vector(typ, elems),
        }
        .with_location(*e.location())
    }

    /// Transforms a reference expr (`&self`)
    fn transform_expr_address_of(&mut self, _typ: &Type, child: &Expr) -> Expr {
        self.transform_expr(child).address_of()
    }

    /// Transform an array expr (`typ x[] = >>> {elems0, elems1 ...} <<<`)
    fn transform_expr_array(&mut self, typ: &Type, elems: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_elems = elems.iter().map(|elem| self.transform_expr(elem)).collect();
        Expr::array_expr(transformed_typ, transformed_elems)
    }

    /// Transform a vector expr (`vec_typ x[] = >>> {elems0, elems1 ...} <<<`)
    fn transform_expr_vector(&mut self, typ: &Type, elems: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_elems = elems.iter().map(|elem| self.transform_expr(elem)).collect();
        Expr::vector_expr(transformed_typ, transformed_elems)
    }

    /// Transforms an array of expr (`typ x[width] = >>> {elem} <<<`)
    fn transform_expr_array_of(&mut self, typ: &Type, elem: &Expr) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_elem = self.transform_expr(elem);
        if let Type::Array { typ: _typ, size } = transformed_typ {
            transformed_elem.array_constant(size)
        } else {
            unreachable!()
        }
    }

    /// Transform an assign expr (`left = right`)
    /// Currently not able to be constructed, as does not exist in Rust
    fn transform_expr_assign(&mut self, _typ: &Type, _left: &Expr, _right: &Expr) -> Expr {
        unreachable!()
    }

    /// Transform a binary operation expr (`lhs op rhs`)
    fn transform_expr_bin_op(
        &mut self,
        _typ: &Type,
        op: &BinaryOperator,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Expr {
        let lhs = self.transform_expr(lhs);
        let rhs = self.transform_expr(rhs);

        lhs.binop(*op, rhs)
    }

    /// Transforms a CPROVER boolean expression (`(__CPROVER_bool) >>> true/false <<<`)
    fn transform_expr_bool_constant(&mut self, _typ: &Type, value: &bool) -> Expr {
        Expr::bool_constant(*value)
    }

    /// Transforms a byte extraction expr (e as type self.typ)
    fn transform_expr_byte_extract(&mut self, typ: &Type, e: &Expr, _offset: &u64) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_e = self.transform_expr(e);
        transformed_e.transmute_to(transformed_typ, self.symbol_table())
    }

    /// Transforms a C boolean constant expr (`(bool) 1`)
    fn transform_expr_c_bool_constant(&mut self, _typ: &Type, value: &bool) -> Expr {
        Expr::c_bool_constant(*value)
    }

    /// Transforms a deref expr (`*self`)
    fn transform_expr_dereference(&mut self, _typ: &Type, child: &Expr) -> Expr {
        let transformed_child = self.transform_expr(child);
        transformed_child.dereference()
    }

    /// Transforms a double constant expr (`1.0`)
    fn transform_expr_double_constant(&mut self, _typ: &Type, value: &f64) -> Expr {
        Expr::double_constant(*value)
    }

    /// Transforms a float constant expr (`1.0f`)
    fn transform_expr_float_constant(&mut self, _typ: &Type, value: &f32) -> Expr {
        Expr::float_constant(*value)
    }

    /// Transforms a function call expr (`function(arguments)`)
    fn transform_expr_function_call(
        &mut self,
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
    fn transform_expr_if(&mut self, _typ: &Type, c: &Expr, t: &Expr, e: &Expr) -> Expr {
        let transformed_c = self.transform_expr(c);
        let transformed_t = self.transform_expr(t);
        let transformed_e = self.transform_expr(e);
        transformed_c.ternary(transformed_t, transformed_e)
    }

    /// Transforms an array index expr (`array[expr]`)
    fn transform_expr_index(&mut self, _typ: &Type, array: &Expr, index: &Expr) -> Expr {
        let transformed_array = self.transform_expr(array);
        let transformed_index = self.transform_expr(index);
        transformed_array.index(transformed_index)
    }

    /// Transforms an int constant expr (`123`)
    fn transform_expr_int_constant(&mut self, typ: &Type, value: &BigInt) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::int_constant(value.clone(), transformed_typ)
    }

    /// Transforms a member access expr (`lhs.field`)
    fn transform_expr_member(&mut self, _typ: &Type, lhs: &Expr, field: InternedString) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(field, self.symbol_table())
    }

    /// Transforms a CPROVER nondet call (`__nondet()`)
    fn transform_expr_nondet(&mut self, typ: &Type) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::nondet(transformed_typ)
    }

    /// Transforms a pointer constant expr (`NULL`)
    fn transform_expr_pointer_constant(&mut self, typ: &Type, value: &u64) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::pointer_constant(*value, transformed_typ)
    }

    /// Transforms a self-op expr (`op++`, etc.)
    fn transform_expr_self_op(&mut self, _typ: &Type, op: &SelfOperator, e: &Expr) -> Expr {
        let transformed_e = self.transform_expr(e);
        match op {
            SelfOperator::Postdecrement => transformed_e.postdecr(),
            SelfOperator::Postincrement => transformed_e.postincr(),
            SelfOperator::Predecrement => transformed_e.predecr(),
            SelfOperator::Preincrement => transformed_e.preincr(),
        }
    }

    /// Transforms a statement expr (({ stmt1; stmt2; ...}))
    fn transform_expr_statement_expression(&mut self, typ: &Type, statements: &[Stmt]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_statements =
            statements.iter().map(|stmt| self.transform_stmt(stmt)).collect();
        Expr::statement_expression(transformed_statements, transformed_typ)
    }

    /// Transforms a string constant expr (`"s"`)
    fn transform_expr_string_constant(&mut self, _typ: &Type, value: InternedString) -> Expr {
        Expr::raw_string_constant(value)
    }

    /// Transforms a struct initializer expr (`struct foo the_foo = >>> {field1, field2, ... } <<<`)
    fn transform_expr_struct(&mut self, typ: &Type, values: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        assert!(
            transformed_typ.is_struct_tag(),
            "Transformed StructTag must be StructTag; got {:?}",
            transformed_typ
        );
        let transformed_values: Vec<_> =
            values.iter().map(|value| self.transform_expr(value)).collect();
        Expr::struct_expr_from_padded_values(
            transformed_typ,
            transformed_values,
            self.symbol_table(),
        )
    }

    /// Transforms a symbol expr (`self`)
    fn transform_expr_symbol(&mut self, typ: &Type, identifier: InternedString) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(identifier, transformed_typ)
    }

    /// Transforms a typecast expr (`(typ) self`)
    fn transform_expr_typecast(&mut self, typ: &Type, child: &Expr) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_child = self.transform_expr(child);
        transformed_child.cast_to(transformed_typ)
    }

    /// Transforms a union initializer expr (`union foo the_foo = >>> {.field = value } <<<`)
    fn transform_expr_union(&mut self, typ: &Type, value: &Expr, field: InternedString) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(transformed_typ, field, transformed_value, self.symbol_table())
    }

    /// Transforms a unary operator expr (`op self`)
    fn transform_expr_un_op(&mut self, _typ: &Type, op: &UnaryOperator, e: &Expr) -> Expr {
        let transformed_e = self.transform_expr(e);
        match op {
            UnaryOperator::Bitnot => transformed_e.bitnot(),
            UnaryOperator::BitReverse => transformed_e.bitreverse(),
            UnaryOperator::Bswap => transformed_e.bswap(),
            UnaryOperator::IsDynamicObject => transformed_e.dynamic_object(),
            UnaryOperator::IsFinite => transformed_e.is_finite(),
            UnaryOperator::Not => transformed_e.not(),
            UnaryOperator::ObjectSize => transformed_e.object_size(),
            UnaryOperator::PointerObject => transformed_e.pointer_object(),
            UnaryOperator::PointerOffset => transformed_e.pointer_offset(),
            UnaryOperator::Popcount => transformed_e.popcount(),
            UnaryOperator::CountTrailingZeros { allow_zero } => transformed_e.cttz(*allow_zero),
            UnaryOperator::CountLeadingZeros { allow_zero } => transformed_e.ctlz(*allow_zero),
            UnaryOperator::UnaryMinus => transformed_e.neg(),
        }
    }

    /// Perform recursive descent on a `Stmt` data structure.
    /// Extracts the variant's field data, and passes them into
    /// the corresponding stmt transformer method.
    fn transform_stmt(&mut self, stmt: &Stmt) -> Stmt {
        match stmt.body() {
            StmtBody::Assign { lhs, rhs } => self.transform_stmt_assign(lhs, rhs),
            StmtBody::Assert { cond, property_class, msg } => {
                self.transform_stmt_assert(cond, *property_class, *msg)
            }
            StmtBody::Assume { cond } => self.transform_stmt_assume(cond),
            StmtBody::AtomicBlock(block) => self.transform_stmt_atomic_block(block),
            StmtBody::Block(block) => self.transform_stmt_block(block),
            StmtBody::Break => self.transform_stmt_break(),
            StmtBody::Continue => self.transform_stmt_continue(),
            StmtBody::Decl { lhs, value } => self.transform_stmt_decl(lhs, value),
            StmtBody::Expression(expr) => self.transform_stmt_expression(expr),
            StmtBody::For { init, cond, update, body } => {
                self.transform_stmt_for(init, cond, update, body)
            }
            StmtBody::FunctionCall { lhs, function, arguments } => {
                self.transform_stmt_function_call(lhs, function, arguments)
            }
            StmtBody::Goto(label) => self.transform_stmt_goto(*label),
            StmtBody::Ifthenelse { i, t, e } => self.transform_stmt_ifthenelse(i, t, e),
            StmtBody::Label { label, body } => self.transform_stmt_label(*label, body),
            StmtBody::Return(value) => self.transform_stmt_return(value),
            StmtBody::Skip => self.transform_stmt_skip(),
            StmtBody::Switch { control, cases, default } => {
                self.transform_stmt_switch(control, cases, default)
            }
            StmtBody::While { cond, body } => self.transform_stmt_while(cond, body),
        }
        .with_location(*stmt.location())
    }

    /// Transforms an assign stmt (`lhs = rhs;`)
    fn transform_stmt_assign(&mut self, lhs: &Expr, rhs: &Expr) -> Stmt {
        let transformed_lhs = self.transform_expr(lhs);
        let transformed_rhs = self.transform_expr(rhs);
        transformed_lhs.assign(transformed_rhs, Location::none())
    }

    /// Transforms a assert stmt (`Assert(cond, property_class, message);`)
    fn transform_stmt_assert(
        &mut self,
        cond: &Expr,
        property_name: InternedString,
        msg: InternedString,
    ) -> Stmt {
        let transformed_cond = self.transform_expr(cond);
        Stmt::assert(
            transformed_cond,
            property_name.to_string().as_str(),
            msg.to_string().as_str(),
            Location::none(),
        )
    }

    /// Transforms a CPROVER assume stmt (`__CPROVER_assume(cond);`)
    fn transform_stmt_assume(&mut self, cond: &Expr) -> Stmt {
        let transformed_cond = self.transform_expr(cond);
        Stmt::assume(transformed_cond, Location::none())
    }

    /// Transforms an atomic block stmt (`{ ATOMIC_BEGIN stmt1; stmt2; ... ATOMIC_END }`)
    fn transform_stmt_atomic_block(&mut self, block: &[Stmt]) -> Stmt {
        let transformed_block = block.iter().map(|stmt| self.transform_stmt(stmt)).collect();
        Stmt::atomic_block(transformed_block, Location::none())
    }

    /// Transforms a block stmt (`{ stmt1; stmt2; ... }`)
    fn transform_stmt_block(&mut self, block: &[Stmt]) -> Stmt {
        let transformed_block = block.iter().map(|stmt| self.transform_stmt(stmt)).collect();
        Stmt::block(transformed_block, Location::none())
    }

    /// Transform a break stmt (`break;`)
    fn transform_stmt_break(&mut self) -> Stmt {
        Stmt::break_stmt(Location::none())
    }

    /// Transform a continue stmt (`continue;`)
    fn transform_stmt_continue(&mut self) -> Stmt {
        Stmt::continue_stmt(Location::none())
    }

    /// Transform a decl stmt (`lhs.typ lhs = value;` or `lhs.typ lhs;`)
    fn transform_stmt_decl(&mut self, lhs: &Expr, value: &Option<Expr>) -> Stmt {
        let transformed_lhs = self.transform_expr(lhs);
        let transformed_value = value.as_ref().map(|value| self.transform_expr(value));
        Stmt::decl(transformed_lhs, transformed_value, Location::none())
    }

    /// Transform an expression stmt (`e;`)
    fn transform_stmt_expression(&mut self, expr: &Expr) -> Stmt {
        let transformed_expr = self.transform_expr(expr);
        transformed_expr.as_stmt(Location::none())
    }

    /// Transform a for loop stmt (`for (init; cond; update) {body}`)
    fn transform_stmt_for(&mut self, init: &Stmt, cond: &Expr, update: &Stmt, body: &Stmt) -> Stmt {
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
    fn transform_stmt_function_call(
        &mut self,
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
    fn transform_stmt_goto(&mut self, label: InternedString) -> Stmt {
        Stmt::goto(label, Location::none())
    }

    /// Transforms an if-then-else stmt (`if (i) { t } else { e }`)
    fn transform_stmt_ifthenelse(&mut self, i: &Expr, t: &Stmt, e: &Option<Stmt>) -> Stmt {
        let transformed_i = self.transform_expr(i);
        let transformed_t = self.transform_stmt(t);
        let transformed_e = e.as_ref().map(|e| self.transform_stmt(e));

        Stmt::if_then_else(transformed_i, transformed_t, transformed_e, Location::none())
    }

    /// Transforms a label stmt (`label: body`)
    fn transform_stmt_label(&mut self, label: InternedString, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(label)
    }

    /// Transforms a return stmt (`return e;` or `return;`)
    fn transform_stmt_return(&mut self, value: &Option<Expr>) -> Stmt {
        let transformed_value = value.as_ref().map(|value| self.transform_expr(value));
        Stmt::ret(transformed_value, Location::none())
    }

    /// Transforms a skip stmt (`;`)
    fn transform_stmt_skip(&mut self) -> Stmt {
        Stmt::skip(Location::none())
    }

    /// Transforms a switch stmt (`switch (control) { case1.case: cast1.body; case2.case: case2.body; ... }`)
    fn transform_stmt_switch(
        &mut self,
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
    fn transform_stmt_while(&mut self, cond: &Expr, body: &Stmt) -> Stmt {
        let transformed_cond = self.transform_expr(cond);
        let transformed_body = self.transform_stmt(body);
        Stmt::while_loop(transformed_cond, transformed_body, Location::none())
    }

    /// Transforms a symbol's type and value
    fn transform_symbol(&mut self, symbol: &Symbol) -> Symbol {
        let new_typ = self.transform_type(&symbol.typ);
        let new_value = self.transform_value(&symbol.value);
        let mut new_symbol = symbol.clone();
        new_symbol.value = new_value;
        new_symbol.typ = new_typ;
        new_symbol
    }

    /// Transforms a symbol value
    fn transform_value(&mut self, value: &SymbolValues) -> SymbolValues {
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

        let mut added: HashSet<InternedString> = HashSet::default();

        // New symbol tables come with some items in them by default. Skip over those.
        for (name, _symbol) in self.symbol_table().iter() {
            added.insert(*name);
        }

        // Expr and Stmt symbols might depend on symbols representing types (e.g. struct type).
        // Fill in the type symbols first so that these dependencies are in place.
        for (name, symbol) in orig_symtab.iter() {
            if !self.symbol_table().contains(*name) && symbol.value.is_none() {
                let new_symbol = self.transform_symbol(symbol);
                self.mut_symbol_table().insert(new_symbol);
                added.insert(*name);
            }
        }

        // Then, fill in everything else.
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
