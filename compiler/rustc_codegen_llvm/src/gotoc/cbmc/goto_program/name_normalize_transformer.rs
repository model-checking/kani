use super::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, SymbolValues, Transformer, Type,
};
use std::collections::{BTreeMap, HashSet};

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
    match valid_chars.chars().next() {
        Some(first) => {
            if first.is_alphanumeric() {
                let mut name = "_".to_string();
                name.push_str(&valid_chars);
                name
            } else {
                valid_chars
            }
        }
        None => "_".to_string(),
    }
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

    fn transform_member_expr(&self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(&normalize_identifier(field), self.symbol_table())
    }

    fn transform_struct_expr(&self, typ: &Type, values: &[Expr]) -> Expr {
        if let Type::Struct { tag: _tag, components } = typ {
            let transformed_typ = self.transform_type(typ);

            let transformed_components: BTreeMap<_, _> = {
                let transformed_values = values.iter().map(|value| self.transform_expr(value));
                let component_names = components.iter().filter_map(|component| match component {
                    DatatypeComponent::Field { name, .. } => Some(normalize_identifier(name)),
                    DatatypeComponent::Padding { .. } => None,
                });
                component_names.zip(transformed_values).collect()
            };

            Expr::struct_expr(transformed_typ, transformed_components, self.symbol_table())
        } else {
            unreachable!()
        }
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
        Type::struct_tag(&normalize_identifier(tag))
    }

    fn transform_union_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(&normalize_identifier(tag), transformed_components)
    }

    fn transform_union_tag_type(&self, tag: &str) -> Type {
        Type::union_tag(&normalize_identifier(tag))
    }

    fn transform_goto_stmt(&self, label: &str) -> Stmt {
        Stmt::goto(normalize_identifier(label), Location::none())
    }

    fn transform_label_stmt(&self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(normalize_identifier(label))
    }
}
