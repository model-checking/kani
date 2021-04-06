// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::utils::aggr_name;
use super::{DatatypeComponent, Expr, Location, Parameter, Stmt, Type};

/// Based off the CBMC symbol implementation here:
/// https://github.com/diffblue/cbmc/blob/develop/src/util/symbol.h
#[derive(Clone, Debug)]
pub struct Symbol {
    /// Unique identifier. Mangled name from compiler `foo12_bar17_x@1`
    pub name: String,
    pub location: Location,
    pub typ: Type,
    pub value: SymbolValues,

    /// Optional debugging information

    /// Local name `x`
    pub base_name: Option<String>,
    /// Fully qualifier name `foo::bar::x`
    pub pretty_name: Option<String>,
    /// Only used by verilog
    pub module: Option<String>,
    pub mode: SymbolModes,
    // global properties
    pub is_exported: bool,
    pub is_input: bool,
    pub is_macro: bool,
    pub is_output: bool,
    pub is_property: bool,
    pub is_state_var: bool,
    pub is_type: bool,

    // ansi-C properties
    pub is_auxiliary: bool,
    pub is_extern: bool,
    pub is_file_local: bool,
    pub is_lvalue: bool,
    pub is_parameter: bool,
    pub is_static_lifetime: bool,
    pub is_thread_local: bool,
    pub is_volatile: bool,
    pub is_weak: bool,
}

/// Currently, only C is understood by CBMC.
// TODO: https://github.com/model-checking/rmc/issues/1
#[derive(Clone, Debug)]
pub enum SymbolModes {
    C,
    Rust,
}

#[derive(Clone, Debug)]
pub enum SymbolValues {
    Expr(Expr),
    Stmt(Stmt),
    None,
}
/// Constructors
impl Symbol {
    fn new(
        name: String,
        location: Location,
        typ: Type,
        value: SymbolValues,
        base_name: Option<String>,
        pretty_name: Option<String>,
    ) -> Self {
        Symbol {
            name,
            location,
            typ,
            value,
            base_name,
            pretty_name,

            module: None,
            mode: SymbolModes::C,
            // global properties
            is_exported: false,
            is_input: false,
            is_macro: false,
            is_output: false,
            is_property: false,
            is_state_var: false,
            is_type: false,
            // ansi-C properties
            is_auxiliary: false,
            is_extern: false,
            is_file_local: false,
            is_lvalue: false,
            is_parameter: false,
            is_static_lifetime: false,
            is_thread_local: false,
            is_volatile: false,
            is_weak: false,
        }
    }

    /// The symbol that defines the type of the struct or union.
    /// For a struct foo this is the symbol "tag-foo" that maps to the type struct foo.
    pub fn aggr_ty(t: Type) -> Symbol {
        //TODO take location
        let base_name = t.tag().unwrap().to_string();
        let name = aggr_name(&base_name);
        Symbol::new(name, Location::none(), t, SymbolValues::None, Some(base_name), None)
            .with_is_type(true)
    }

    pub fn builtin_function(name: &str, param_types: Vec<Type>, return_type: Type) -> Symbol {
        Symbol::function(
            name,
            Type::code_with_unnamed_parameters(param_types, return_type),
            None,
            Location::builtin_function(name, None),
        )
    }

    pub fn constant(
        name: &str,
        pretty_name: &str,
        base_name: &str,
        value: Expr,
        loc: Location,
    ) -> Symbol {
        Symbol::new(
            name.to_string(),
            loc,
            value.typ().clone(),
            SymbolValues::Expr(value),
            Some(base_name.to_string()),
            Some(pretty_name.to_string()),
        )
        .with_is_static_lifetime(true) //TODO with thread local was also true??
    }

    pub fn function(name: &str, typ: Type, body: Option<Stmt>, loc: Location) -> Symbol {
        // TODO should take pretty name
        Symbol::new(
            name.to_string(),
            loc,
            typ,
            body.map_or(SymbolValues::None, |x| SymbolValues::Stmt(x)),
            Some(name.to_string()),
            None,
        )
        .with_is_lvalue(true)
    }

    pub fn typedef(name: &str, pretty_name: &str, typ: Type, loc: Location) -> Symbol {
        Symbol::new(
            name.to_string(),
            loc,
            typ,
            SymbolValues::None,
            Some(name.to_string()),
            Some(pretty_name.to_string()),
        )
        .with_is_type(true)
        .with_is_file_local(true)
        .with_is_static_lifetime(true)
    }

    pub fn variable(name: String, base_name: String, t: Type, l: Location) -> Symbol {
        Symbol::new(name, l, t, SymbolValues::None, Some(base_name), None)
            .with_is_thread_local(true)
            .with_is_lvalue(true)
            .with_is_state_var(true)
    }

    pub fn struct_type(name: &str, components: Vec<DatatypeComponent>) -> Symbol {
        Symbol::aggr_ty(Type::struct_type(name, components))
    }

    pub fn union_type(name: &str, components: Vec<DatatypeComponent>) -> Symbol {
        Symbol::aggr_ty(Type::union_type(name, components))
    }

    pub fn empty_struct(name: &str) -> Symbol {
        Symbol::aggr_ty(Type::empty_struct(name))
    }

    pub fn empty_union(name: &str) -> Symbol {
        Symbol::aggr_ty(Type::empty_union(name))
    }

    pub fn incomplete_struct(name: &str) -> Symbol {
        Symbol::aggr_ty(Type::incomplete_struct(name))
    }

    pub fn incomplete_union(name: &str) -> Symbol {
        Symbol::aggr_ty(Type::incomplete_union(name))
    }
}

/// Setters
impl Symbol {
    pub fn update_fn_declaration_with_definition(&mut self, body: Stmt) {
        assert!(self.is_function_declaration());
        self.value = SymbolValues::Stmt(body);
    }

    pub fn with_is_extern(mut self, v: bool) -> Symbol {
        self.is_extern = v;
        self
    }

    pub fn with_is_file_local(mut self, v: bool) -> Symbol {
        self.is_file_local = v;
        self
    }

    pub fn with_is_lvalue(mut self, v: bool) -> Symbol {
        self.is_lvalue = v;
        self
    }

    pub fn with_is_static_lifetime(mut self, v: bool) -> Symbol {
        self.is_static_lifetime = v;
        self
    }

    pub fn with_is_state_var(mut self, v: bool) -> Symbol {
        self.is_state_var = v;
        self
    }

    pub fn with_is_thread_local(mut self, v: bool) -> Symbol {
        self.is_thread_local = v;
        self
    }

    pub fn with_is_type(mut self, v: bool) -> Symbol {
        self.is_type = v;
        self
    }

    pub fn with_pretty_name(mut self, pretty_name: &str) -> Symbol {
        self.pretty_name = Some(pretty_name.to_string());
        self
    }
}

/// Predicates
impl Symbol {
    /// This is a struct or union that completes an incomplete struct or union.
    pub fn completes(&self, old_symbol: Option<&Symbol>) -> bool {
        match old_symbol {
            Some(symbol) => self.typ.completes(&symbol.typ),
            None => false,
        }
    }

    pub fn is_function(&self) -> bool {
        self.typ.is_code() || self.typ.is_variadic_code()
    }

    pub fn is_function_declaration(&self) -> bool {
        self.is_function() && self.value.is_none()
    }

    pub fn is_function_definition(&self) -> bool {
        self.is_function() && self.value.is_stmt()
    }
}

/// Conversions to goto_program types
impl Symbol {
    /// Makes a formal function parameter from a symbol.
    pub fn to_function_parameter(&self) -> Parameter {
        Type::parameter(Some(self.name.to_string()), self.base_name.clone(), self.typ.clone())
    }

    /// Makes an expression from a symbol.
    pub fn to_expr(&self) -> Expr {
        Expr::symbol_expression(self.name.clone(), self.typ.clone())
    }
}

impl SymbolValues {
    pub fn is_expr(&self) -> bool {
        match self {
            SymbolValues::Expr(_) => true,
            SymbolValues::None | SymbolValues::Stmt(_) => false,
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            SymbolValues::None => true,
            SymbolValues::Expr(_) | SymbolValues::Stmt(_) => false,
        }
    }

    pub fn is_stmt(&self) -> bool {
        match self {
            SymbolValues::Stmt(_) => true,
            SymbolValues::Expr(_) | SymbolValues::None => false,
        }
    }
}

/// ToString

impl ToString for SymbolModes {
    fn to_string(&self) -> String {
        match self {
            SymbolModes::C => "C",
            SymbolModes::Rust => "Rust",
        }
        .to_string()
    }
}
