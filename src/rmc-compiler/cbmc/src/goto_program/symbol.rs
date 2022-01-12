// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::utils::aggr_tag;
use super::{DatatypeComponent, Expr, Location, Parameter, Stmt, Type};
use crate::{InternStringOption, InternedString};

/// Based off the CBMC symbol implementation here:
/// https://github.com/diffblue/cbmc/blob/develop/src/util/symbol.h
#[derive(Clone, Debug)]
pub struct Symbol {
    /// Unique identifier. Mangled name from compiler `foo12_bar17_x@1`
    pub name: InternedString,
    pub location: Location,
    pub typ: Type,
    pub value: SymbolValues,

    /// Optional debugging information

    /// Local name `x`
    pub base_name: Option<InternedString>,
    /// Fully qualifier name `foo::bar::x`
    pub pretty_name: Option<InternedString>,
    /// Only used by verilog
    pub module: Option<InternedString>,
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
    fn new<T: Into<InternedString>, U: Into<InternedString>, V: Into<InternedString>>(
        name: T,
        location: Location,
        typ: Type,
        value: SymbolValues,
        base_name: Option<U>,
        pretty_name: Option<V>,
    ) -> Self {
        let name = name.into();
        let base_name = base_name.intern();
        let pretty_name = pretty_name.intern();
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
    pub fn aggr_ty<T: Into<InternedString>>(t: Type, pretty_name: Option<T>) -> Symbol {
        //TODO take location
        let pretty_name = pretty_name.intern();
        let base_name = t.tag().unwrap();
        let name = aggr_tag(base_name);
        Symbol::new(name, Location::none(), t, SymbolValues::None, Some(base_name), pretty_name)
            .with_is_type(true)
    }

    pub fn builtin_function<T: Into<InternedString>>(
        name: T,
        param_types: Vec<Type>,
        return_type: Type,
    ) -> Symbol {
        let name = name.into();
        Symbol::function(
            name,
            Type::code_with_unnamed_parameters(param_types, return_type),
            None,
            None::<InternedString>,
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

    pub fn function<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        typ: Type,
        body: Option<Stmt>,
        pretty_name: Option<U>,
        loc: Location,
    ) -> Symbol {
        let name = name.into();
        let pretty_name = pretty_name.intern();
        Symbol::new(
            name.to_string(),
            loc,
            typ,
            body.map_or(SymbolValues::None, |x| SymbolValues::Stmt(x)),
            Some(name),
            pretty_name,
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

    pub fn variable<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        base_name: U,
        t: Type,
        l: Location,
    ) -> Symbol {
        let name = name.into();
        let base_name: InternedString = base_name.into();
        Symbol::new(name, l, t, SymbolValues::None, Some(base_name), None::<InternedString>)
            .with_is_thread_local(true)
            .with_is_lvalue(true)
            .with_is_state_var(true)
    }

    pub fn static_variable<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        base_name: U,
        t: Type,
        l: Location,
    ) -> Symbol {
        let name = name.into();
        let base_name: InternedString = base_name.into();
        Symbol::variable(name, base_name, t, l)
            .with_is_thread_local(false)
            .with_is_static_lifetime(true)
    }

    pub fn struct_type<T: Into<InternedString>>(
        name: T,
        pretty_name: Option<InternedString>,
        components: Vec<DatatypeComponent>,
    ) -> Symbol {
        let name = name.into();
        Symbol::aggr_ty(Type::struct_type(name, components), pretty_name)
    }

    pub fn union_type<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        pretty_name: Option<U>,
        components: Vec<DatatypeComponent>,
    ) -> Symbol {
        let name = name.into();
        let pretty_name = pretty_name.intern();
        Symbol::aggr_ty(Type::union_type(name, components), pretty_name)
    }

    pub fn empty_struct(name: InternedString, pretty_name: Option<InternedString>) -> Symbol {
        Symbol::aggr_ty(Type::empty_struct(name), pretty_name)
    }

    pub fn empty_union(name: InternedString, pretty_name: Option<InternedString>) -> Symbol {
        Symbol::aggr_ty(Type::empty_union(name), pretty_name)
    }

    pub fn incomplete_struct<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        pretty_name: Option<U>,
    ) -> Symbol {
        Symbol::aggr_ty(Type::incomplete_struct(name), pretty_name)
    }

    pub fn incomplete_union<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        pretty_name: Option<U>,
    ) -> Symbol {
        let name = name.into();
        let pretty_name = pretty_name.intern();
        Symbol::aggr_ty(Type::incomplete_union(name), pretty_name)
    }
}

/// Setters
impl Symbol {
    pub fn update_fn_declaration_with_definition(&mut self, body: Stmt) {
        assert!(self.is_function_declaration(), "Expected function declaration, got {:?}", self);
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

    pub fn with_pretty_name<T: Into<InternedString>>(mut self, pretty_name: T) -> Symbol {
        self.pretty_name = Some(pretty_name.into());
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
        self.typ.clone().as_parameter(Some(self.name), self.base_name)
    }

    /// Makes an expression from a symbol.
    pub fn to_expr(&self) -> Expr {
        Expr::symbol_expression(self.name, self.typ.clone())
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
