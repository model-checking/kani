// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::utils::aggr_tag;
use super::{DatatypeComponent, Expr, Location, Parameter, Stmt, Type};
use crate::{InternStringOption, InternedString};

use std::fmt::Display;

/// Based off the CBMC symbol implementation here:
/// <https://github.com/diffblue/cbmc/blob/develop/src/util/symbol.h>
///
/// TODO: We should consider using BitFlags for all the boolean flags.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// Unique identifier. Mangled name from compiler `foo12_bar17_x@1`
    pub name: InternedString,
    pub location: Location,
    pub typ: Type,
    pub value: SymbolValues,
    /// Contracts to be enforced (only supported for functions)
    pub contract: Option<Box<FunctionContract>>,

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

    /// This flag marks a variable as constant (IrepId: `ID_C_constant`).
    ///
    /// In CBMC, this is a property of the type or expression. However, we keep it here to avoid
    /// having to propagate the attribute to all variants of `Type` and `Expr`.
    ///
    /// During contract verification, CBMC will not havoc static variables marked as constant.
    pub is_static_const: bool,
}

/// The equivalent of a "mathematical function" in CBMC. Semantically this is an
/// anonymous function object, similar to a closure, but without closing over an
/// environment.
///
/// This is only valid for use as a function contract. It may not perform side
/// effects, a property that is enforced on the CBMC side.
///
/// The precise nomenclature is that in CBMC a contract value has *type*
/// `mathematical_function` and values of that type are `lambda`s. Since this
/// struct represents such values it is named `Lambda`.
#[derive(Debug, Clone)]
pub struct Lambda {
    pub arguments: Vec<Parameter>,
    pub body: Expr,
}

impl Lambda {
    pub fn as_contract_for(
        fn_ty: &Type,
        return_var_name: Option<InternedString>,
        body: Expr,
    ) -> Self {
        let arguments = match fn_ty {
            Type::Code { parameters, return_type } => {
                [Parameter::new(None, return_var_name, (**return_type).clone())]
                    .into_iter()
                    .chain(parameters.iter().cloned())
                    .collect()
            }
            _ => panic!(
                "Contract lambdas can only be generated for `Code` types, received {fn_ty:?}"
            ),
        };
        Self { arguments, body }
    }
}

/// The CBMC representation of a function contract. Represents
/// https://diffblue.github.io/cbmc/contracts-user.html but currently only assigns clauses are
/// supported.
#[derive(Clone, Debug)]
pub struct FunctionContract {
    pub(crate) assigns: Vec<Lambda>,
}

impl FunctionContract {
    pub fn new(assigns: Vec<Lambda>) -> Self {
        Self { assigns }
    }
}

/// Currently, only C is understood by CBMC.
// TODO: <https://github.com/model-checking/kani/issues/1>
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
        // See https://github.com/model-checking/kani/issues/1361#issuecomment-1181499683
        assert!(
            name.to_string().ends_with(&base_name.map_or(String::new(), |s| s.to_string())),
            "Symbol's base_name must be the suffix of its name.\nName: {name:?}\nBase name: {base_name:?}"
        );
        Symbol {
            name,
            location,
            typ,
            value,
            base_name,
            pretty_name,

            contract: None,
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
            is_auxiliary: true,
            is_extern: false,
            is_file_local: false,
            is_lvalue: false,
            is_parameter: false,
            is_static_lifetime: false,
            is_static_const: false,
            is_thread_local: false,
            is_volatile: false,
            is_weak: false,
        }
    }

    /// Add this contract to the symbol (symbol must be a function) or fold the
    /// conditions into an existing contract.
    pub fn attach_contract(&mut self, contract: FunctionContract) {
        assert!(self.typ.is_code());
        match self.contract {
            Some(ref mut prior) => {
                prior.assigns.extend(contract.assigns);
            }
            None => self.contract = Some(Box::new(contract)),
        }
    }

    /// The symbol that defines the type of the struct or union.
    /// For a struct foo this is the symbol "tag-foo" that maps to the type struct foo.
    pub fn aggr_ty<T: Into<InternedString>>(t: Type, pretty_name: T) -> Symbol {
        //TODO take location
        let pretty_name = pretty_name.into();
        let base_name = t.tag().unwrap();
        let name = aggr_tag(base_name);
        Symbol::new(
            name,
            Location::none(),
            t,
            SymbolValues::None,
            Some(base_name),
            Some(pretty_name),
        )
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
            name,
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
        pretty_name: U,
        loc: Location,
    ) -> Symbol {
        let name = name.into();
        let pretty_name = pretty_name.into();
        Symbol::new(
            name.to_string(),
            loc,
            typ,
            body.map_or(SymbolValues::None, SymbolValues::Stmt),
            Some(name),
            Some(pretty_name),
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
        pretty_name: InternedString,
        components: Vec<DatatypeComponent>,
    ) -> Symbol {
        let name = name.into();
        Symbol::aggr_ty(Type::struct_type(name, components), pretty_name)
    }

    pub fn union_type<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        pretty_name: U,
        components: Vec<DatatypeComponent>,
    ) -> Symbol {
        let name = name.into();
        let pretty_name = pretty_name.into();
        Symbol::aggr_ty(Type::union_type(name, components), pretty_name)
    }

    pub fn empty_struct(name: InternedString, pretty_name: InternedString) -> Symbol {
        Symbol::aggr_ty(Type::empty_struct(name), pretty_name)
    }

    pub fn empty_union(name: InternedString, pretty_name: InternedString) -> Symbol {
        Symbol::aggr_ty(Type::empty_union(name), pretty_name)
    }

    pub fn incomplete_struct<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        pretty_name: U,
    ) -> Symbol {
        Symbol::aggr_ty(Type::incomplete_struct(name), pretty_name)
    }

    pub fn incomplete_union<T: Into<InternedString>, U: Into<InternedString>>(
        name: T,
        pretty_name: U,
    ) -> Symbol {
        let name = name.into();
        let pretty_name = pretty_name.into();
        Symbol::aggr_ty(Type::incomplete_union(name), pretty_name)
    }
}

/// Setters
impl Symbol {
    pub fn update_fn_declaration_with_definition(&mut self, body: Stmt) {
        assert!(self.is_function_declaration(), "Expected function declaration, got {self:?}");
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

    pub fn with_is_parameter(mut self, v: bool) -> Symbol {
        self.is_parameter = v;
        self
    }

    pub fn with_is_static_lifetime(mut self, v: bool) -> Symbol {
        self.is_static_lifetime = v;
        self
    }

    pub fn set_is_static_const(&mut self, v: bool) -> &mut Symbol {
        self.is_static_const = v;
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

    pub fn set_pretty_name<T: Into<InternedString>>(&mut self, pretty_name: T) -> &mut Symbol {
        self.pretty_name = Some(pretty_name.into());
        self
    }

    pub fn with_is_hidden(mut self, hidden: bool) -> Symbol {
        self.is_auxiliary = hidden;
        self
    }

    pub fn set_is_hidden(&mut self, hidden: bool) -> &mut Symbol {
        self.is_auxiliary = hidden;
        self
    }

    /// Set `is_property`.
    pub fn with_is_property(mut self, v: bool) -> Self {
        self.is_property = v;
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

/// Display
impl Display for SymbolModes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = match self {
            SymbolModes::C => "C",
            SymbolModes::Rust => "Rust",
        };
        write!(f, "{mode}")
    }
}
