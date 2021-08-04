// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::{
    BinaryOperand, CIntType, DatatypeComponent, Expr, Location, Parameter, Stmt, Symbol,
    SymbolTable, SymbolValues, Type,
};
use super::Transformer;
use num::bigint::BigInt;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use std::cell::RefCell;

thread_local!(static NONDET_TYPES: RefCell<FxHashMap<String, Type>> = RefCell::new(FxHashMap::default()));

thread_local!(static NEW_SYMS: RefCell<FxHashMap<String, Symbol>> = RefCell::new(FxHashMap::default()));

thread_local!(static EMPTY_STATICS: RefCell<FxHashMap<String, (Type, Type)>> = RefCell::new(FxHashMap::default()));

/// Add identifier to a transformed parameter if it's missing.
/// Necessary when function wasn't originally a definition.
fn add_identifier(parameter: &Parameter) -> Parameter {
    if parameter.identifier().is_some() {
        parameter.clone()
    } else {
        let new_name = normalize_identifier(&format!("__{}", type_to_string(parameter.typ())));
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
fn normalize_identifier(orig_name: &str) -> String {
    assert!(!orig_name.is_empty(), "Received empty identifier.");

    // If name already encountered, return same result
    match MAPPED_NAMES.with(|map| map.borrow().get(orig_name).cloned()) {
        Some(result) => return result.clone(),
        None => (),
    }

    let (prefix, name) = if orig_name.starts_with("tag-") {
        (&orig_name[..4], &orig_name[4..])
    } else {
        ("", orig_name)
    };

    // Convert non-(alphanumeric + underscore) characters to underscore
    let valid_chars =
        name.replace(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '$'), "_");

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
    let mut illegal_names: FxHashMap<_, _> =
        [("case", "case_"), ("main", "main_"), ("default", "_default")]
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

    let result = {
        let mut prefix = prefix.to_string();
        prefix.push_str(&result);
        prefix
    };

    // Remember result and return
    MAPPED_NAMES.with(|map| {
        map.borrow_mut().insert(orig_name.to_string(), result);
        map.borrow().get(orig_name).unwrap().clone()
    })
}

/// Create a string representation of type for use as variable name suffix.
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

/// Create an expr from an int constant using only values <= u64::MAX.
fn bignum_to_expr(num: &BigInt, typ: &Type) -> Expr {
    let u64_bigint = BigInt::from(u64::MAX);
    if num <= &u64_bigint {
        Expr::int_constant(num.clone(), typ.clone())
    } else {
        let quotient = num / &u64_bigint;
        let remainder = num % &u64_bigint;

        let quotient_expr = bignum_to_expr(&quotient, typ);
        let remainder_expr = bignum_to_expr(&remainder, typ);
        Expr::int_constant(u64_bigint, typ.clone()).mul(quotient_expr).plus(remainder_expr)
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
    /// Purpose-tag: normalize-name
    fn transform_type_parameter(&mut self, parameter: &Parameter) -> Parameter {
        Type::parameter(
            parameter.identifier().map(|name| normalize_identifier(name)),
            parameter.base_name().map(|name| normalize_identifier(name)),
            self.transform_type(parameter.typ()),
        )
    }

    /// Translate Implies into Or/Not
    /// Purpose-tag: replace
    fn transform_expr_bin_op(
        &mut self,
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

    /// Prevent error for too large constants with u128.
    /// Purpose-tag: replace
    fn transform_expr_int_constant(&mut self, typ: &Type, value: &BigInt) -> Expr {
        let transformed_typ = self.transform_type(typ);
        bignum_to_expr(value, typ)
    }

    /// When indexing into a SIMD vector, cast to a pointer first to make legal index in C
    /// Purpose-tag: replace
    fn transform_expr_index(&mut self, typ: &Type, array: &Expr, index: &Expr) -> Expr {
        let transformed_array = self.transform_expr(array);
        let transformed_index = self.transform_expr(index);
        if transformed_array.typ().is_vector() {
            let base_type = transformed_array.typ().base_type().unwrap().clone();
            transformed_array.address_of().cast_to(base_type.to_pointer()).index(transformed_index)
        } else {
            transformed_array.index(transformed_index)
        }
    }

    /// Normalize field names.
    /// Purpose-tag: normalize-name
    fn transform_expr_member(&mut self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(&normalize_identifier(field), self.symbol_table())
    }

    /// Transform nondets to create default values for the expected type.
    /// Purpose-tag: nondet
    fn transform_expr_nondet(&mut self, typ: &Type) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let typ_string = normalize_identifier(&type_to_string(&transformed_typ));
        let identifier = format!("non_det_{}", typ_string);
        let function_type = Type::code(vec![], transformed_typ);
        NONDET_TYPES
            .with(|cell| cell.borrow_mut().insert(identifier.clone(), function_type.clone()));
        Expr::symbol_expression(identifier, function_type).call(vec![])
    }

    /// Don't transform padding fields so that they are ignored by CBMC --dump-c.
    /// Purpose-tag: nondet
    fn transform_expr_struct(&mut self, typ: &Type, values: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        assert!(
            transformed_typ.is_struct_tag(),
            "Transformed StructTag must be StructTag; got {:?}",
            transformed_typ
        );
        let fields = self.symbol_table().lookup_fields_in_type(&transformed_typ).unwrap().clone();
        let mut transformed_values = Vec::new();
        for (field, value) in fields.into_iter().zip(values.into_iter()) {
            if field.is_padding() {
                transformed_values.push(value.clone());
            } else {
                transformed_values.push(self.transform_expr(value));
            }
        }
        Expr::struct_expr_from_padded_values(
            transformed_typ,
            transformed_values,
            self.symbol_table(),
        )
    }

    /// Normalize name in identifier expression.
    /// Purpose-tag: normalize-name
    fn transform_expr_symbol(&mut self, typ: &Type, identifier: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(normalize_identifier(identifier), transformed_typ)
    }

    /// Normalize union field names.
    /// Purpose-tag: normalize-name
    fn transform_expr_union(&mut self, typ: &Type, value: &Expr, field: &str) -> Expr {
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
    /// Purpose-tag: normalize-name
    fn transform_type_incomplete_struct(&mut self, tag: &str) -> Type {
        Type::incomplete_struct(&normalize_identifier(tag))
    }

    /// Normalize incomplete union tag name.
    /// Purpose-tag: normalize-name
    fn transform_type_incomplete_union(&mut self, tag: &str) -> Type {
        Type::incomplete_union(&normalize_identifier(tag))
    }

    /// Normalize union/struct component name.
    /// Purpose-tag: normalize-name
    fn transform_datatype_component(&mut self, component: &DatatypeComponent) -> DatatypeComponent {
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
    /// Purpose-tag: normalize-name
    fn transform_type_struct(&mut self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(&normalize_identifier(tag), transformed_components)
    }

    /// Normalize struct tag name.
    /// Purpose-tag: normalize-name
    fn transform_type_struct_tag(&mut self, tag: &str) -> Type {
        Type::struct_tag_raw(&normalize_identifier(tag))
    }

    /// Normalize union type name.
    /// Purpose-tag: normalize-name
    fn transform_type_union(&mut self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(&normalize_identifier(tag), transformed_components)
    }

    /// Normalize union tag name.
    /// Purpose-tag: normalize-name
    fn transform_type_union_tag(&mut self, tag: &str) -> Type {
        Type::union_tag_raw(&normalize_identifier(tag))
    }

    /// Normalize goto label name.
    /// Purpose-tag: normalize-name
    fn transform_stmt_goto(&mut self, label: &str) -> Stmt {
        Stmt::goto(normalize_identifier(label), Location::none())
    }

    /// Normalize label name.
    /// Purpose-tag: normalize-name
    fn transform_stmt_label(&mut self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(normalize_identifier(label))
    }

    /// Normalize symbol names.
    fn transform_symbol(&mut self, symbol: &Symbol) -> Symbol {
        let mut new_symbol = if symbol.is_extern {
            if symbol.typ.is_code() || symbol.typ.is_variadic_code() {
                // Purpose-tag: nondet
                // Replace extern functions with nondet body so linker doesn't break
                assert!(symbol.value.is_none(), "Extern function should have no body.");
                let new_typ = self.transform_type(&symbol.typ);

                // Fill missing parameter names with dummy name
                let parameters = new_typ
                    .parameters()
                    .unwrap()
                    .iter()
                    .map(|parameter| add_identifier(parameter))
                    .collect();

                let ret_typ = new_typ.return_type().unwrap();
                let (ret_typ, new_body) = if ret_typ.type_name() == Some("tag-Unit".to_string()) {
                    let ret_typ = Type::empty();
                    let new_body = Stmt::block(vec![], Location::none());
                    (ret_typ, new_body)
                } else {
                    let nondet_expr = self
                        .transform_expr(&Expr::nondet(symbol.typ.return_type().unwrap().clone()));
                    let new_body = Stmt::ret(Some(nondet_expr), Location::none());
                    (ret_typ.clone(), new_body)
                };

                let new_value = SymbolValues::Stmt(new_body);

                let new_typ = if new_typ.is_code() {
                    Type::code(parameters, ret_typ)
                } else {
                    Type::variadic_code(parameters, ret_typ)
                };

                let mut new_symbol = symbol.clone();
                new_symbol.value = new_value;
                new_symbol.typ = new_typ;
                new_symbol.with_is_extern(false)
            } else {
                assert!(
                    symbol.is_static_lifetime,
                    "Extern objects that aren't functions should be static variables."
                );
                let new_typ = self.transform_type(&symbol.typ);
                let new_value = SymbolValues::Expr(self.transform_expr_nondet(&symbol.typ));

                let mut new_symbol = symbol.clone();
                new_symbol.value = new_value;
                new_symbol.typ = new_typ.clone();

                EMPTY_STATICS.with(|cell| {
                    cell.borrow_mut().insert(
                        normalize_identifier(&new_symbol.name),
                        (symbol.typ.clone(), new_typ),
                    )
                });

                new_symbol.with_is_extern(false)
            }
        } else {
            let new_typ = self.transform_type(&symbol.typ);
            let new_value = self.transform_value(&symbol.value);

            let mut new_symbol = symbol.clone();
            new_symbol.value = new_value;
            new_symbol.typ = new_typ;
            new_symbol
        };

        // Purpose-tag: normalize-name
        new_symbol.name = normalize_identifier(&new_symbol.name);
        new_symbol.base_name = new_symbol.base_name.map(|name| normalize_identifier(&name));
        new_symbol.pretty_name = new_symbol.pretty_name.map(|name| normalize_identifier(&name));
        new_symbol
    }

    /// Perform cleanup necessary to make C code valid.
    fn postprocess(&mut self) {
        // Redefine main function to return an `int`.
        // Moves `main` to `main_`, and create `main` to call now `main_`.
        // Purpose-tag: replace
        let old_main_typ = self.symbol_table().lookup("main_").map(|old_main| old_main.typ.clone());
        if let Some(old_main_typ) = old_main_typ {
            let mut main_body = {
                let statics_map = EMPTY_STATICS.with(|cell| cell.take());
                let mut assgns = Vec::new();
                for (name, (orig_typ, new_typ)) in statics_map {
                    let sym_expr = Expr::symbol_expression(name, new_typ.clone());
                    let value = self.transform_expr_nondet(&orig_typ);
                    assgns.push(Stmt::assign(sym_expr, value, Location::none()));
                }
                assgns
            };
            main_body.push(Stmt::code_expression(
                Expr::symbol_expression("main_".to_string(), old_main_typ).call(Vec::new()),
                Location::none(),
            ));
            main_body.push(Stmt::ret(
                Some(Expr::int_constant(0, Type::CInteger(CIntType::Int))),
                Location::none(),
            ));

            let new_main = Symbol::function(
                "main",
                Type::code(Vec::new(), Type::CInteger(CIntType::Int)),
                Some(Stmt::block(main_body, Location::none())),
                Some("main".to_string()),
                Location::none(),
            );
            self.mut_symbol_table().insert(new_main);
        }

        // Purpose-tag: nondet
        for (identifier, typ) in NONDET_TYPES.with(|cell| cell.take()) {
            let ret_type = typ.return_type().unwrap();
            let (typ, body) = if ret_type.type_name() == Some("tag-Unit".to_string()) {
                let typ = Type::code(typ.parameters().unwrap().clone(), Type::empty());
                let body = Stmt::block(vec![], Location::none());
                (typ, body)
            } else {
                let ret_value = Some(ret_type.default(self.symbol_table()));
                let body = Stmt::ret(ret_value, Location::none());
                (typ, body)
            };

            let sym = Symbol::function(
                &identifier,
                typ,
                Some(body),
                Some(identifier.clone()),
                Location::none(),
            );

            self.mut_symbol_table().insert(sym);
        }

        // Purpose-tag: normalize-name
        for (_, symbol) in NEW_SYMS.with(|cell| cell.take()) {
            self.mut_symbol_table().insert(symbol);
        }
    }
}
