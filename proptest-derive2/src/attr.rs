// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides a parser from syn attributes to our logical model.

use syn;

use util;
use error;

//==============================================================================
// Public API
//==============================================================================

/// Parsed attributes in our logical model.
#[derive(Clone)]
pub struct ParsedAttributes {
    /// If we've been ordered to skip this item.
    /// This is only valid for enum variants.
    pub skip: bool,
    /// The potential weight assigned to an enum variant.
    /// This must be `None` for things that are not enum variants.
    pub weight: PWeight,
    /// The mode for `Parameters` to use. See that type for more.
    pub params: ParamsMode,
    /// The mode for `Strategy` to use. See that type for more.
    pub strategy: StratMode,
    /// True if no_bound was specified.
    pub no_bound: bool,
}

/// The mode for the associated item `Strategy` to use.
#[derive(Clone)]
pub enum StratMode {
    /// This means that no explicit strategy was specified
    /// and that we thus should use `Arbitrary` for whatever
    /// it is that needs a strategy.
    Arbitrary,
    /// This means that an explicit value has been provided.
    /// The result of this is to use a strategy that always
    /// returns the given value.
    Value(syn::Expr),
    /// This means that an explicit strategy has been provided.
    /// This strategy will be used to generate whatever it
    /// is that the attribute was set on.
    Strategy(syn::Expr),
}

/// The mode for the associated item `Parameters` to use.
#[derive(Clone)]
pub enum ParamsMode {
    /// Nothing has been specified. The children are now free to
    /// specify their parameters, and if nothing is specified, then
    /// `<X as Arbitrary>::Parameters` will be used for a type `X`.
    Passthrough,
    /// We've been ordered to use the Default value of
    /// `<X as Arbitrary>::Parameters` for some field where applicable.
    /// For the top level item, this means that `Parameters` will be
    /// the unit type. For children, it means that this child should
    /// not count towards the product type that is being built up.
    Default,
    /// An explicit type has been specified on some item.
    /// If the top level item has this specified on it, this means
    /// that `Parameters` will have the given type.
    /// If it is specified on a child of the top level item, this
    /// entails that the given type will be added to the resultant
    /// product type.
    Specified(syn::Type),
}

impl ParamsMode {
    /// Returns `true` iff the mode was explicitly set.
    pub fn is_set(&self) -> bool {
        if let ParamsMode::Passthrough = *self { false } else { true }
    }

    /// Converts the mode to an `Option` of an `Option` of a type
    /// where the outer `Option` is `None` iff the mode wasn't set
    /// and the inner `Option` is `None` iff the mode was `Default`.
    pub fn to_option(self) -> Option<Option<syn::Type>> {
        use self::ParamsMode::*;
        match self {
            Passthrough => None,
            Specified(ty) => Some(Some(ty)),
            Default => Some(None),
        }
    }
}

impl StratMode {
    /// Returns `true` iff the mode was explicitly set.
    pub fn is_set(&self) -> bool {
        if let StratMode::Arbitrary = self { false } else { true }
    }
}

/// Parse the attributes specified on an item and parsed by syn
/// into our logical model that we work with.
pub fn parse_attributes(attrs: Vec<syn::Attribute>) -> ParsedAttributes {
    let attrs = parse_attributes_base(attrs);
    if attrs.no_bound {
        error::no_bound_set_on_non_tyvar();
    }
    attrs
}

/// Parses the attributes specified on an item and parsed by syn
/// and returns true if we've been ordered to not set an `Arbitrary`
/// bound on the given type variable the attributes are from,
/// no matter what.
pub fn has_no_bound(attrs: Vec<syn::Attribute>) -> bool {
    let attrs = parse_attributes_base(attrs);
    error::if_anything_specified(&attrs, error::TY_VAR);
    attrs.no_bound
}

/// Parse the attributes specified on an item and parsed by syn
/// into our logical model that we work with.
fn parse_attributes_base(attrs: Vec<syn::Attribute>) -> ParsedAttributes {
    let (skip, weight, no_params, ty_params, strategy, value, no_bound)
      = attrs.into_iter()
             // Get rid of attributes we don't care about:
             .filter(is_proptest_attr)
             // Flatten attributes so we deal with them uniformly.
             .flat_map(extract_modifiers)
             // Accumulate attributes into a form for final processing.
             .fold(init_parse_state(), dispatch_attribute);

    // Process params and no_params together to see which one to use.
    let params = parse_params_mode(no_params, ty_params);

    // Process strategy and value together to see which one to use.
    let strategy = parse_strat_mode(strategy, value);

    // Was skip set?
    let skip = skip.is_some();

    let no_bound = no_bound.is_some();

    // We're done.
    ParsedAttributes { skip, weight, params, strategy, no_bound }
}

//==============================================================================
// Internals: Initialization
//==============================================================================

type PSkip     = Option<()>;
type PWeight   = Option<u32>;
type PNoParams = Option<()>;
type PTyParams = Option<syn::Type>;
type PStrategy = Option<syn::Expr>;
type PNoBound  = Option<()>;
type PAll      = (PSkip, PWeight,
                  PNoParams, PTyParams,
                  PStrategy, PStrategy,
                  PNoBound);

/// The initial state in the accumulator inside `parse_attributes`.
fn init_parse_state() -> PAll { (None, None, None, None, None, None, None) }

//==============================================================================
// Internals: Extraction & Filtering
//==============================================================================

/// Returns `true` iff the attribute has to do with proptest.
/// Otherwise, the attribute is irrevant to us and we will simply
/// ignore it in our processing.
fn is_proptest_attr(attr: &syn::Attribute) -> bool {
    util::eq_simple_path("proptest", &attr.path)
}

/// Extract all individual attributes inside one `#[proptest(..)]`.
/// We do this to treat all pieces uniformly whether a single
/// `#[proptest(..)]` was used or many. This simplifies the
/// logic somewhat.
fn extract_modifiers(attr: syn::Attribute) -> Box<Iterator<Item = syn::Meta>> {
    use syn::Meta::*;
    use syn::NestedMeta::*;
    use syn::MetaList;

    // Ensure we've been given an outer attribute form.
    if !is_outer_attr(&attr) { error::inner_attr(); }

    if let Some(meta) = attr.interpret_meta() {
        match meta {
            Word(_) => error::bare_proptest_attr(),
            NameValue(_) => error::literal_set_proptest(),
            List(MetaList { nested, .. }) =>
                Box::new(nested.into_iter().map(|nmi| match nmi {
                    Literal(_) => error::immediate_literals(),
                    // This is the only valid form.
                    Meta(mi) => mi,
                })),
        }
    } else {
        panic!("TODO");
    }
}

/// Returns true iff the given attribute is an outer one, i.e: `#[<attr>]`.
/// An inner attribute is the other possibility and has the syntax `#![<attr>]`.
/// Note that `<attr>` is a meta-variable for the contents inside.
pub fn is_outer_attr(attr: &syn::Attribute) -> bool {
    syn::AttrStyle::Outer == attr.style
}

//==============================================================================
// Internals: Dispatch
//==============================================================================

/// Dispatches an attribute modifier to handlers and
/// let's them add stuff into our accumulartor.
fn dispatch_attribute(mut acc: PAll, meta: syn::Meta) -> PAll {
    // TODO: revisit when we have NLL.

    // Dispatch table for attributes:
    //
    // N.B: We use this trick to return function pointers to avoid cloning.
    // Once we have NLL this might not be necessary.
    let parser = {
        let name = meta.name().to_string();
        let name = name.as_ref();
        match name {
            // Valid modifiers:
            "skip"          => parse_skip,
            "w" | "weight"  => parse_weight,
            "no_params"     => parse_no_params,
            "params"        => parse_params,
            "strategy"      => parse_strategy,
            "value"         => parse_value,
            "no_bound"      => parse_no_bound,
            // Invalid modifiers:
            "no_bounds"     => error::did_you_mean(name, "no_bound"),
            "weights" |
            "weighted"      => error::did_you_mean(name, "weight"),
            "strat" |
            "strategies"    => error::did_you_mean(name, "strategy"),
            "values" |
            "valued" |
            "fix" |
            "fixed"         => error::did_you_mean(name, "value"),
            "param" |
            "parameters"    => error::did_you_mean(name, "params"),
            "no_param" |
            "no_parameters" => error::did_you_mean(name, "no_params"),
            modifier        => error::unkown_modifier(modifier),
        }
    };

    // We now have a parser that we can dispatch to.
    parser(&mut acc, meta);

    acc
}

//==============================================================================
// Internals: no_bound
//==============================================================================

/// Parse a no_bound attribute.
/// Valid forms are:
/// + `#[proptest(no_bound)]`
fn parse_no_bound(set: &mut PAll, meta: syn::Meta) {
    parse_bare_modifier(&mut set.6, meta, error::no_bound_malformed);
}

//==============================================================================
// Internals: Skip
//==============================================================================

/// Parse a skip attribute.
/// Valid forms are:
/// + `#[proptest(skip)]`
fn parse_skip(set: &mut PAll, meta: syn::Meta) {
    parse_bare_modifier(&mut set.0, meta, error::skip_malformed);
}

//==============================================================================
// Internals: Weight
//==============================================================================

/// Parses a weight.
/// Valid forms are:
/// + `#[proptest(weight = <integer>)]`.
/// The `<integer>` must also fit within an `u32` and be unsigned.
fn parse_weight(set: &mut PAll, meta: syn::Meta) {
    use std::u32;
    error_if_set(&set.1, &meta);

    if let Some(syn::Lit::Int(lit)) = get_mnv_lit(&meta) {
        let value = lit.value();
        // Ensure that `val` fits within an `u32` as proptest requires that.
        if value <= u32::MAX as u64 &&
           is_int_suffix_unsigned(lit.suffix()) {
            set.1 = Some(value as u32);  
        } else {
            error::weight_malformed(&meta);
        }
    } else {
        error::weight_malformed(&meta);
    }
}

/// Returns `true` iff the given type is unsigned and false otherwise.
fn is_int_suffix_unsigned(suffix: syn::IntSuffix) -> bool {
    use syn::IntSuffix::*;
    match suffix {
        U8 | U16 | U32 | U64 | U128 | Usize | None => true,
        _ => false,
    }
}

//==============================================================================
// Internals: Strategy
//==============================================================================

/// Parses an explicit value as a strategy.
/// Valid forms are:
/// + `#[proptest(value = "<expr>")]`.
fn parse_value(set: &mut PAll, meta: syn::Meta) {
    parse_strategy_base(&mut set.5, meta);
}

/// Parses an explicit strategy.
/// Valid forms are:
/// + `#[proptest(strategy = "<expr>")]`.
fn parse_strategy(set: &mut PAll, meta: syn::Meta) {
    parse_strategy_base(&mut set.4, meta);
}

/// Parses an explicit strategy. This is a helper.
/// Valid forms are:
/// + `#[proptest(<meta.name()> = "<expr>")]`.
fn parse_strategy_base(set: &mut PStrategy, meta: syn::Meta) {
    error_if_set(&set, &meta);

    if let Some(lit) = get_str_lit(&meta) {
        if let Ok(expr) = syn::parse_str(&lit) {
            *set = Some(expr);
        } else {
            error::strategy_malformed_expr(&meta);
        }
    } else {
        error::strategy_malformed(&meta);
    }
}

/// Combines any parsed explicit strategy and value into a single value
/// and fails if both an explicit strategy and value was set.
/// Only one of them can be set, or none.
fn parse_strat_mode(strat: PStrategy, value: PStrategy) -> StratMode {
    match (strat, value) {
        (None,     None    ) => StratMode::Arbitrary,
        (None,     Some(ty)) => StratMode::Value(ty),
        (Some(ty), None    ) => StratMode::Strategy(ty),
        (Some(_), Some(_) )  => error::overspecified_strat(),
    }
}

//==============================================================================
// Internals: Parameters
//==============================================================================

/// Combines a potentially set `params` and `no_params` into a single value
/// and fails if both have been set. Only one of them can be set, or none.
fn parse_params_mode(no_params: PNoParams, ty_params: PTyParams) -> ParamsMode {
    match (no_params, ty_params) {
        (None,    None    ) => ParamsMode::Passthrough,
        (None,    Some(ty)) => ParamsMode::Specified(ty),
        (Some(_), None    ) => ParamsMode::Default,
        (Some(_), Some(_) ) => error::overspecified_param(),
    }
}

/// Parses an explicit Parameters type.
///
/// Valid forms are:
/// + `#[proptest(params(<type>)]`
/// + `#[proptest(params = "<type>"]`
///
/// The latter form is required for more complex types.
fn parse_params(set: &mut PAll, meta: syn::Meta) {
    let set = &mut set.3;

    error_if_set(&set, &meta);

    if let Some(lit) = get_str_lit(&meta) {
        // Form is: `#[proptest(params = "<type>"]`.
        if let Ok(ty) = syn::parse_str(&lit) {
            *set = Some(ty);
        } else {
            error::param_malformed_type();
        }
    } else if let Some(ident) = get_nested_metas(meta)
                                .and_then(util::match_singleton)
                                .and_then(get_nmw) {
        // Form is: `#[proptest(params(<type>)]`.
        *set = Some(ident_to_type(ident));
    } else {
        error::param_malformed();
    }
}

/// Parses an order to use the default Parameters type and value.
/// Valid forms are:
/// + `#[proptest(no_params)]`
fn parse_no_params(set: &mut PAll, meta: syn::Meta) {
    parse_bare_modifier(&mut set.2, meta, error::no_params_malformed);
}

//==============================================================================
// Internals: Utilities
//==============================================================================

/// Parses a bare attribute of the form `#[proptest(<attr>)]` and sets `set`.
fn parse_bare_modifier
    (set: &mut Option<()>, meta: syn::Meta, malformed: fn() -> !) {
    error_if_set(set, &meta);
    if let syn::Meta::Word(_) = meta { *set = Some(()); }
    else { malformed() }
}

/// Emits a "set again" error iff the given option `.is_some()`.
fn error_if_set<T>(set: &Option<T>, meta: &syn::Meta) {
    if set.is_some() { error::set_again(meta); }
}

fn get_str_lit(meta: &syn::Meta) -> Option<String> {
    if let syn::Lit::Str(lit) = get_mnv_lit(&meta)? {
        Some(lit.value())
    } else {
        None
    }
}

fn get_mnv_lit(meta: &syn::Meta) -> Option<&syn::Lit> {
    if let syn::Meta::NameValue(syn::MetaNameValue { lit, .. }) = meta {
        Some(lit)
    } else {
        None
    }
}

fn get_nmw(meta: syn::NestedMeta) -> Option<syn::Ident> {
    if let syn::NestedMeta::Meta(syn::Meta::Word(ident)) = meta {
        Some(ident)
    } else {
        None
    }
}

fn get_nested_metas(meta: syn::Meta)
    -> Option<syn::punctuated::Punctuated<syn::NestedMeta, Token![,]>>
{
    if let syn::Meta::List(syn::MetaList { nested, .. }) = meta {
        Some(nested)
    } else {
        None
    }
}

/// Constructs a type out of an identifier.
fn ident_to_type(ident: syn::Ident) -> syn::Type {
    syn::Type::Path(syn::TypePath { qself: None, path: ident.into() })
}
