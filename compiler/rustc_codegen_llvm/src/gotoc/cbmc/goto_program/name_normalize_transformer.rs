use super::{
    DatatypeComponent, Expr, Location, Parameter, Stmt, Symbol, SymbolTable, SymbolValues,
    Transformer, Type,
};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct NameTransformer {
    new_symbol_table: SymbolTable,
}

impl NameTransformer {
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        NameTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

fn normalize_identifier(name: &str) -> String {
    let valid_chars = name.replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");
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

    let mut illegal_names: HashMap<_, _> = [("case", "case_"), ("main", "main_")]
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();

    illegal_names.remove(&new_name).unwrap_or(new_name)
}

impl Transformer for NameTransformer {
    fn symbol_table(&self) -> &SymbolTable {
        &self.new_symbol_table
    }

    fn mut_symbol_table(&mut self) -> &mut SymbolTable {
        &mut self.new_symbol_table
    }

    fn extract_symbol_table(self) -> SymbolTable {
        self.new_symbol_table
    }

    fn transform_parameter(&self, parameter: &Parameter) -> Parameter {
        Type::parameter(
            parameter.identifier().map(|name| normalize_identifier(name)),
            parameter.base_name().map(|name| normalize_identifier(name)),
            self.transform_type(parameter.typ()),
        )
    }

    fn transform_member_expr(&self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(&normalize_identifier(field), self.symbol_table())
    }

    fn transform_symbol_expr(&self, typ: &Type, identifier: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(normalize_identifier(identifier), transformed_typ)
    }

    fn transform_union_expr(&self, typ: &Type, value: &Expr, field: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(
            transformed_typ,
            &normalize_identifier(field),
            transformed_value,
            self.symbol_table(),
        )
    }

    fn transform_incomplete_struct_type(&self, tag: &str) -> Type {
        Type::incomplete_struct(&normalize_identifier(tag))
    }

    fn transform_incomplete_union_type(&self, tag: &str) -> Type {
        Type::incomplete_union(&normalize_identifier(tag))
    }

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

    fn transform_struct_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(&normalize_identifier(tag), transformed_components)
    }

    fn transform_struct_tag_type(&self, tag: &str) -> Type {
        Type::struct_tag_raw(&normalize_identifier(tag))
    }

    fn transform_union_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(&normalize_identifier(tag), transformed_components)
    }

    fn transform_union_tag_type(&self, tag: &str) -> Type {
        Type::union_tag_raw(&normalize_identifier(tag))
    }

    fn transform_goto_stmt(&self, label: &str) -> Stmt {
        Stmt::goto(normalize_identifier(label), Location::none())
    }

    fn transform_label_stmt(&self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(normalize_identifier(label))
    }

    fn transform_symbol(&self, symbol: &Symbol) -> Symbol {
        let new_typ = self.transform_type(&symbol.typ);
        let new_value = self.transform_value(&symbol.value);
        let mut new_symbol = symbol.clone();
        new_symbol.value = new_value;
        new_symbol.typ = new_typ;
        new_symbol.name = normalize_identifier(&new_symbol.name);
        new_symbol.base_name = new_symbol.base_name.map(|name| normalize_identifier(&name));
        new_symbol.pretty_name = new_symbol.pretty_name.map(|name| normalize_identifier(&name));
        new_symbol
    }
}
