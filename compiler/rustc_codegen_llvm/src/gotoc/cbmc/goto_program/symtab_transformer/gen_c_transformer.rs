// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::{
    BinaryOperand, CIntType, DatatypeComponent, Expr, Location, Parameter, Stmt, Symbol,
    SymbolTable, SymbolValues, Type,
};
use super::Transformer;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use std::cell::RefCell;

thread_local!(static NONDET_TYPES: RefCell<FxHashMap<String, Type>> = RefCell::new(FxHashMap::default()));

thread_local!(static NEW_SYMS: RefCell<FxHashMap<String, Symbol>> = RefCell::new(FxHashMap::default()));

/// Add identifier to a transformed parameter if it's missing.
/// Necessary when function wasn't originally a definition.
fn add_identifier(parameter: &Parameter) -> Parameter {
    if parameter.identifier().is_some() {
        parameter.clone()
    } else {
        let new_name = format!("__{}", type_to_string(parameter.typ()));
        let parameter_sym = Symbol::variable(
            new_name.clone(),
            new_name.clone(),
            parameter.typ().clone(),
            Location::none(),
        );
        let parameter = parameter_sym.to_function_parameter();
        NEW_SYMS.with(|cell| cell.borrow_mut().insert(new_name, parameter_sym));
        parameter
    }
}

thread_local!(static MAPPED_NAMES: RefCell<FxHashMap<String, String>> = RefCell::new(FxHashMap::default()));
thread_local!(static USED_NAMES: RefCell<FxHashSet<String>> = RefCell::new(FxHashSet::default()));

/// Converts an arbitrary identifier into a valid C identifier.
fn normalize_identifier(name: &str) -> String {
    assert!(!name.is_empty(), "Received empty identifier.");

    // If name already encountered, return same result
    match MAPPED_NAMES.with(|map| map.borrow().get(name).cloned()) {
        Some(result) => return result.clone(),
        None => (),
    }

    // Convert non-(alphanumeric + underscore) characters to underscore
    let valid_chars = name.replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");

    // If the first character is a number, prefix with underscore
    let new_name = match valid_chars.chars().next() {
        Some(first) => {
            if first.is_numeric() {
                let mut name = "_".to_string();
                name.push_str(&valid_chars);
                name
            } else {
                valid_chars
            }
        }
        None => "_".to_string(),
    };

    // Replace reserved names with alternatives
    let mut illegal_names: FxHashMap<_, _> = [("case", "case_"), ("main", "main_")]
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let result = illegal_names.remove(&new_name).unwrap_or(new_name);

    // Ensure result has not been used before
    let result = if USED_NAMES.with(|set| set.borrow().contains(&result)) {
        let mut suffix = 0;
        loop {
            let result = format!("{}_{}", result, suffix);
            if !USED_NAMES.with(|set| set.borrow().contains(&result)) {
                break result;
            }
            suffix += 1;
        }
    } else {
        result
    };

    // Remember result and return
    MAPPED_NAMES.with(|map| {
        map.borrow_mut().insert(name.to_string(), result);
        map.borrow().get(name).unwrap().clone()
    })
}

fn type_to_string(typ: &Type) -> String {
    match typ {
        Type::Array { typ, size } => format!("array_of_{}_{}", size, type_to_string(typ.as_ref())),
        Type::Bool => format!("bool"),
        Type::CBitField { typ, .. } => format!("cbitfield_of_{}", type_to_string(typ.as_ref())),
        Type::CInteger(_) => format!("c_int"),
        Type::Code { .. } => format!("code"),
        Type::Constructor => format!("constructor"),
        Type::Double => format!("double"),
        Type::Empty => format!("empty"),
        Type::FlexibleArray { typ } => format!("flexarray_of_{}", type_to_string(typ.as_ref())),
        Type::Float => format!("float"),
        Type::IncompleteStruct { tag } => tag.clone(),
        Type::IncompleteUnion { tag } => tag.clone(),
        Type::InfiniteArray { typ } => {
            format!("infinite_array_of_{}", type_to_string(typ.as_ref()))
        }
        Type::Pointer { typ } => format!("pointer_to_{}", type_to_string(typ.as_ref())),
        Type::Signedbv { width } => format!("signed_bv_{}", width),
        Type::Struct { tag, .. } => format!("struct_{}", tag),
        Type::StructTag(tag) => format!("struct_{}", tag),
        Type::Union { tag, .. } => format!("union_{}", tag),
        Type::UnionTag(tag) => format!("union_{}", tag),
        Type::Unsignedbv { width } => format!("unsigned_bv_{}", width),
        Type::VariadicCode { .. } => format!("variadic_code"),
        Type::Vector { typ, .. } => format!("vec_of_{}", type_to_string(typ.as_ref())),
    }
}

/// Struct for performing the gen-c transformation on a symbol table.
pub struct GenCTransformer {
    new_symbol_table: SymbolTable,
}

impl GenCTransformer {
    /// Transform all identifiers in the symbol table to be valid C identifiers;
    /// perform other clean-up operations to make valid C code.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        GenCTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

impl Transformer for GenCTransformer {
    /// Get reference to symbol table.
    fn symbol_table(&self) -> &SymbolTable {
        &self.new_symbol_table
    }

    /// Get mutable reference to symbol table.
    fn mut_symbol_table(&mut self) -> &mut SymbolTable {
        &mut self.new_symbol_table
    }

    /// Get owned symbol table.
    fn extract_symbol_table(self) -> SymbolTable {
        self.new_symbol_table
    }

    /// Normalize parameter identifier.
    fn transform_type_parameter(&self, parameter: &Parameter) -> Parameter {
        Type::parameter(
            parameter.identifier().map(|name| normalize_identifier(name)),
            parameter.base_name().map(|name| normalize_identifier(name)),
            self.transform_type(parameter.typ()),
        )
    }

    /// Translate Implies into Or/Not
    fn transform_expr_bin_op(
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
            // `lhs ==> rhs` <==> `!lhs || rhs`
            BinaryOperand::Implies => lhs.not().or(rhs),
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

    /// Normalize field names.
    fn transform_expr_member(&self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(&normalize_identifier(field), self.symbol_table())
    }

    /// Transform nondets to missing functions so they get headers
    fn transform_expr_nondet(&self, typ: &Type) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let typ_string = type_to_string(&transformed_typ);
        let identifier = format!("non_det_{}", typ_string);
        let function_type = Type::code(vec![], transformed_typ);
        NONDET_TYPES
            .with(|cell| cell.borrow_mut().insert(identifier.clone(), function_type.clone()));
        Expr::symbol_expression(identifier, function_type).call(vec![])
    }

    /// Don't transform padding fields so that they are ignored by CBMC --dump-c.
    fn transform_expr_struct(&self, typ: &Type, values: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        assert!(
            transformed_typ.is_struct_tag(),
            "Transformed StructTag must be StructTag; got {:?}",
            transformed_typ
        );
        let fields = self.symbol_table().lookup_fields_in_type(&transformed_typ).unwrap();
        let transformed_values: Vec<_> = fields
            .into_iter()
            .zip(values.into_iter())
            .map(
                |(field, value)| {
                    if field.is_padding() { value.clone() } else { self.transform_expr(value) }
                },
            )
            .collect();
        Expr::struct_expr_from_padded_values(
            transformed_typ,
            transformed_values,
            self.symbol_table(),
        )
    }

    /// Normalize name in identifier expression.
    fn transform_expr_symbol(&self, typ: &Type, identifier: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(normalize_identifier(identifier), transformed_typ)
    }

    /// Normalize union field names.
    fn transform_expr_union(&self, typ: &Type, value: &Expr, field: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(
            transformed_typ,
            &normalize_identifier(field),
            transformed_value,
            self.symbol_table(),
        )
    }

    /// Normalize incomplete struct tag name.
    fn transform_type_incomplete_struct(&self, tag: &str) -> Type {
        Type::incomplete_struct(&normalize_identifier(tag))
    }

    /// Normalize incomplete union tag name.
    fn transform_type_incomplete_union(&self, tag: &str) -> Type {
        Type::incomplete_union(&normalize_identifier(tag))
    }

    /// Normalize union/struct component name.
    fn transform_datatype_component(&self, component: &DatatypeComponent) -> DatatypeComponent {
        match component {
            DatatypeComponent::Field { name, typ } => DatatypeComponent::Field {
                name: normalize_identifier(name),
                typ: self.transform_type(typ),
            },
            DatatypeComponent::Padding { name, bits } => {
                DatatypeComponent::Padding { name: normalize_identifier(name), bits: *bits }
            }
        }
    }

    /// Normalize struct type name.
    fn transform_type_struct(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(&normalize_identifier(tag), transformed_components)
    }

    /// Normalize struct tag name.
    fn transform_type_struct_tag(&self, tag: &str) -> Type {
        Type::struct_tag_raw(&normalize_identifier(tag))
    }

    /// Normalize union type name.
    fn transform_type_union(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(&normalize_identifier(tag), transformed_components)
    }

    /// Normalize union tag name.
    fn transform_type_union_tag(&self, tag: &str) -> Type {
        Type::union_tag_raw(&normalize_identifier(tag))
    }

    /// Normalize goto label name.
    fn transform_stmt_goto(&self, label: &str) -> Stmt {
        Stmt::goto(normalize_identifier(label), Location::none())
    }

    /// Normalize label name.
    fn transform_stmt_label(&self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(normalize_identifier(label))
    }

    /// Normalize symbol names.
    fn transform_symbol(&self, symbol: &Symbol) -> Symbol {
        let mut new_symbol = if symbol.is_extern {
            // Replace extern functions with nondet body so linker doesn't break
            assert!(
                symbol.typ.is_code() || symbol.typ.is_variadic_code(),
                "Extern symbol should be function."
            );
            assert!(symbol.value.is_none(), "Extern function should have no body.");
            let new_typ = self.transform_type(&symbol.typ);
            let nondet_expr =
                self.transform_expr(&Expr::nondet(symbol.typ.return_type().unwrap().clone()));
            let new_value = SymbolValues::Stmt(Stmt::ret(Some(nondet_expr), Location::none()));

            // Fill missing parameter names with dummy name
            let parameters = new_typ
                .parameters()
                .unwrap()
                .iter()
                .map(|parameter| add_identifier(parameter))
                .collect();
            let new_typ = if new_typ.is_code() {
                Type::code(parameters, new_typ.return_type().unwrap().clone())
            } else {
                Type::variadic_code(parameters, new_typ.return_type().unwrap().clone())
            };

            let mut new_symbol = symbol.clone();
            new_symbol.value = new_value;
            new_symbol.typ = new_typ;
            new_symbol.with_is_extern(false)
        } else {
            let new_typ = self.transform_type(&symbol.typ);
            let new_value = self.transform_value(&symbol.value);

            let mut new_symbol = symbol.clone();
            new_symbol.value = new_value;
            new_symbol.typ = new_typ;
            new_symbol
        };

        new_symbol.name = normalize_identifier(&new_symbol.name);
        new_symbol.base_name = new_symbol.base_name.map(|name| normalize_identifier(&name));
        new_symbol.pretty_name = new_symbol.pretty_name.map(|name| normalize_identifier(&name));
        new_symbol
    }

    /// Perform cleanup necessary to make C code valid.
    fn postprocess(&mut self) {
        // Remove predefined macros that cause issues being re-declared.
        let memcpy = self.mut_symbol_table().remove("memcpy");
        assert!(memcpy.is_some());
        let memmove = self.mut_symbol_table().remove("memmove");
        assert!(memmove.is_some());
        let memcmp = self.mut_symbol_table().remove("memcmp");
        assert!(memcmp.is_some());

        // Redefine main function to return an `int`.
        // Moves `main` to `main_`, and create `main` to call now `main_`.
        let old_main = self.symbol_table().lookup("main_");
        if let Some(old_main) = old_main {
            let new_main = Symbol::function(
                "main",
                Type::code(Vec::new(), Type::CInteger(CIntType::Int)),
                Some(Stmt::block(
                    vec![
                        Stmt::code_expression(
                            Expr::symbol_expression("main_".to_string(), old_main.typ.clone())
                                .call(Vec::new()),
                            Location::none(),
                        ),
                        Stmt::ret(
                            Some(Expr::int_constant(0, Type::CInteger(CIntType::Int))),
                            Location::none(),
                        ),
                    ],
                    Location::none(),
                )),
                Some("main".to_string()),
                Location::none(),
            );
            self.mut_symbol_table().insert(new_main);
        }

        for (identifier, typ) in NONDET_TYPES.with(|cell| cell.take()) {
            let value = typ.return_type().unwrap().default(self.symbol_table());
            let sym = Symbol::function(
                &identifier,
                typ,
                Some(Stmt::ret(Some(value), Location::none())),
                Some(identifier.clone()),
                Location::none(),
            );
            self.mut_symbol_table().insert(sym);
        }

        for (_, symbol) in NEW_SYMS.with(|cell| cell.take()) {
            self.mut_symbol_table().insert(symbol);
        }
    }
}
