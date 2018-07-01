// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides actual deriving logic for the crate.

use proc_macro2::TokenStream;
use syn;

use util::{is_unit_type, self_ty, DeriveInput, fields_to_vec};//, variant_data_to_fields};
use void::IsUninhabited;
use error::{self, Ctx, Context, DeriveResult};
use attr::{parse_attributes, ParamsMode, ParsedAttributes, StratMode};
use use_tracking::{UseMarkable, UseTracker};
use ast::*;

// TODO: Handle recursive types.

//==============================================================================
// API
//==============================================================================

pub fn impl_proptest_arbitrary(ast: syn::DeriveInput) -> TokenStream {
    let mut ctx = Context::new();
    let result = derive_proptest_arbitrary(&mut ctx, ast);
    match (result, ctx.check()) {
        (Ok(derive), Ok(())) => derive,
        (_, Err(err)) => err,
        (Err(result), Ok(())) =>
            panic!("[proptest_derive]: internal error, this is a bug! \
                    result: {:?}", result),
    }
}

/// Entry point for deriving `Arbitrary`.
pub fn derive_proptest_arbitrary(ctx: Ctx, ast: syn::DeriveInput)
    -> DeriveResult<TokenStream>
{
    use syn::Data::*;

    // Deny lifetimes on type.
    error::if_has_lifetimes(ctx, &ast);

    let tracker = UseTracker::new(ast.generics);

    // Compile into our own high level IR for the impl:
    let the_impl = match ast.data {
        // Deal with structs:
        Struct(data) => derive_struct(ctx, DeriveInput {
            tracker,
            ident: ast.ident,
            attrs: ast.attrs,
            body: fields_to_vec(data.fields),
        }),
        // Deal with enums:
        Enum(data) => derive_enum(ctx, DeriveInput {
            tracker,
            ident: ast.ident,
            attrs: ast.attrs,
            body: data.variants.into_iter().collect(),
        }),
        // Unions are not supported:
        _ => { error::not_struct_or_enum(ctx)? }
    }?;

    // Linearise the IR into Rust code:
    let q = the_impl.to_tokens(ctx)?;

    // TODO: remove! (perhaps keep it behind a feature flag...?):
    //println!("{}\n", q);

    // We're done!
    Ok(q)
}

//==============================================================================
// Struct
//==============================================================================

/// Entry point for deriving `Arbitrary` for `struct`s.
fn derive_struct(ctx: Ctx, mut ast: DeriveInput<Vec<syn::Field>>) -> DeriveResult<Impl> {
    let t_attrs = parse_attributes(ctx, ast.attrs)?;

    // Deny attributes that are only for enum variants:
    error::if_enum_attrs_present(ctx, &t_attrs, error::STRUCT);

    // Deny an explicit strategy directly on the struct.
    error::if_strategy_present(ctx, &t_attrs, error::STRUCT);

    let v_path = ast.ident.clone().into();
    let parts = if ast.body.is_empty() {
        // Deriving for a unit struct.
        error::if_params_present_on_unit_struct(ctx, &t_attrs);
        let (strat, ctor) = pair_unit_self(v_path);
        (Params::empty(), strat, ctor)
    } else {
        // Not a unit struct.

        // Ensures that the fields of the given struct has fields which are all
        // inhabited. If one field is uninhabited, the entire product type is
        // uninhabited.
        //
        // A unit struct in the other branch is by definition always inhabited.
        if (&*ast.body).is_uninhabited() {
            error::uninhabited_struct(ctx);
        }

        // Construct the closure for `.prop_map`:
        let closure = map_closure(v_path, &ast.body);

        // The complexity of the logic depends mostly now on whether
        // parameters were set directly on the type or not.
        if let Some(param_ty) = t_attrs.params.to_option() {
            // Parameters was set on the struct itself, the logic is simpler.
            add_top_params(param_ty,
                derive_product_has_params(ctx, &mut ast.tracker,
                    error::STRUCT_FIELD, closure, ast.body)?)
        } else {
            // We need considerably more complex logic.
            derive_product_no_params(
                ctx, &mut ast.tracker,
                ast.body, error::STRUCT_FIELD
            )?.finish(closure)
        }
    };

    // We're done!
    Ok(Impl::new(ast.ident, ast.tracker, parts))
}

/// Determine the `Parameters` part. We've already handled everything else.
/// After this, we have all parts needed for an impl. If `None` is given,
/// then the unit type `()` will be used for `Parameters`.
fn add_top_params(param_ty: Option<syn::Type>, (strat, ctor): StratPair)
    -> ImplParts
{
    let params = Params::empty();
    if let Some(params_ty) = param_ty {
        // We need to add `let params = _top;`.
        (params + params_ty, strat, extract_api(ctor, FromReg::Top))
    } else {
        (params, strat, ctor)
    }
}

/// Deriving for a list of fields (product type) on
/// which `params` or `no_params` was set directly.
fn derive_product_has_params(
    ctx: Ctx,
    ut: &mut UseTracker, item: &str, closure: MapClosure, fields: Vec<syn::Field>)
    -> DeriveResult<StratPair>
{
    // Fold into an accumulator of the strategy types and the expressions
    // that produces the strategy. Finally turn the accumulator into
    // a `.prop_map(..)` that produces the composite strategy.
    let len = fields.len();
    fields.into_iter().try_fold(StratAcc::new(len), |acc, field| {
        let attrs = parse_attributes(ctx, field.attrs)?;

        // Deny attributes that are only for enum variants:
        error::if_enum_attrs_present(ctx, &attrs, item);

        // Deny setting parameters on the field since it has been set on parent:
        error::if_specified_params(ctx, &attrs, item)?;

        // Determine the strategy for this field and add it to acc.
        let r = acc.add(product_handle_default_params(ut, field.ty, attrs.strategy));
        Ok(r)
    }).map(|acc| acc.finish(closure))
}

/// Determine strategy using "Default" semantics  for a product.
fn product_handle_default_params
    (ut: &mut UseTracker, ty: syn::Type, strategy: StratMode) -> StratPair {
    match strategy {
        // Specific strategy - use the given expr and erase the type
        // (since we don't know about it):
        StratMode::Strategy(strat) => pair_existential(ty, strat),
        // Specific value - use the given expr:
        StratMode::Value(value) => pair_value(ty, value),
        // Use Arbitrary for the given type and mark the type as used:
        StratMode::Arbitrary => { ty.mark_uses(ut); pair_any(ty) },
    }
}

/// Deriving for a list of fields (product type) on
/// which `params` or `no_params` was NOT set directly.
fn derive_product_no_params
    (ctx: Ctx, ut: &mut UseTracker, fields: Vec<syn::Field>, item: &str)
    -> DeriveResult<PartsAcc<Ctor>>
{
    // Fold into an accumulator of the strategy types and the expressions
    // that produces the strategy. We then just return that accumulator
    // and let the caller of this function determine what to do with it.
    let acc = PartsAcc::new(fields.len());
    fields.into_iter().try_fold(acc, |mut acc, field| {
        let attrs = parse_attributes(ctx, field.attrs)?;

        // Deny attributes that are only for enum variants:
        error::if_enum_attrs_present(ctx, &attrs, item);

        let ty = field.ty;
        let strat = match attrs.params {
            // Parameters were not set on the field:
            ParamsMode::Passthrough => match attrs.strategy {
                // Specific strategy - use the given expr and erase the type:
                StratMode::Strategy(strat) => pair_existential(ty, strat),
                // Specific value - use the given expr:
                StratMode::Value(value) => pair_value(ty, value),
                // Use Arbitrary for the given type and mark the type as used:
                StratMode::Arbitrary => {
                    ty.mark_uses(ut);

                    // We use the Parameters type of the field's type.
                    let pref = acc.add_param(arbitrary_param(ty.clone()));
                    pair_any_with(ty, pref)
                },
            },
            // no_params set on the field:
            ParamsMode::Default =>
                product_handle_default_params(ut, ty, attrs.strategy),
            // params(<type>) set on the field:
            ParamsMode::Specified(params_ty) => match attrs.strategy {
                // Specific strategy - use the given expr and erase the type:
                StratMode::Strategy(strat) =>
                    // We need to extract the param as the binding `params`:
                    extract_nparam(&mut acc, params_ty,
                        pair_existential(ty, strat)),
                // Specific value - can't handle this atm:
                StratMode::Value(_) =>
                    error::cant_set_param_and_value(ctx, item)?,
                // Logic error by user. Pointless to specify params and not
                // the strategy. Bail!
                StratMode::Arbitrary =>
                    error::cant_set_param_but_not_strat(ctx, &ty, item)?,
            },
        };
        Ok(acc.add_strat(strat))
    })
}

/// Wrap the given constructor with a let binding
/// moving `param_<x>` into `params`.
fn extract_nparam<C>
    (acc: &mut PartsAcc<C>, params_ty: syn::Type, (strat, ctor): StratPair)
    -> StratPair
{
    (strat, extract_api(ctor, FromReg::Num(acc.add_param(params_ty))))
}

//==============================================================================
// Enum
//==============================================================================

/// Entry point for deriving `Arbitrary` for `enum`s.
fn derive_enum(ctx: Ctx, mut ast: DeriveInput<Vec<syn::Variant>>) -> DeriveResult<Impl> {
    use void::IsUninhabited;

    let t_attrs = parse_attributes(ctx, ast.attrs)?;

    // An enum can't be skipped, ensure it hasn't been:
    error::if_skip_present(ctx, &t_attrs, error::ENUM);

    // We don't allow a strategy on the enum directly:
    error::if_strategy_present(ctx, &t_attrs, error::ENUM);

    // TODO: how to handle this?
    error::if_weight_present(ctx, &t_attrs, error::ENUM);

    // Bail if there are no variants:
    if ast.body.is_empty() {
        error::uninhabited_enum_with_no_variants(ctx)?;
    }

    // Bail if all variants are uninhabited:
    if (&*ast.body).is_uninhabited() {
        error::uninhabited_enum_variants_uninhabited(ctx)?;
    }

    // The complexity of the logic depends mostly now on whether
    // parameters were set directly on the type or not.
    let parts = if let Some(sty) = t_attrs.params.to_option() {
        // The logic is much simpler in this branch.
        derive_enum_has_params(ctx, &mut ast.tracker, &ast.ident, ast.body, sty)
    } else {
        // And considerably more complex here.
        derive_enum_no_params(ctx, &mut ast.tracker, &ast.ident, ast.body)
    }?;

    // We're done!
    Ok(Impl::new(ast.ident, ast.tracker, parts))
}

/// Deriving for a enum on which `params` or `no_params` was NOT set directly.
fn derive_enum_no_params(
    ctx: Ctx,
    ut: &mut UseTracker, _self: &syn::Ident, variants: Vec<syn::Variant>)
    -> DeriveResult<ImplParts>
{
    // Initialize the accumulator:
    let mut acc = PartsAcc::new(variants.len());

    /*
    // Keep the inhabited variants:
    let inhabited = variants.into_iter().filter_map(|var|
        keep_inhabited_variant(_self, var)
    );
    */

    // Fold into the accumulator the strategies for each variant:
    for variant in variants {
        if let Some((weight, ident, fields, attrs))
             = keep_inhabited_variant(ctx, _self, variant)? {
            let path = parse_quote!( #_self::#ident );
            let (strat, ctor) = if fields.is_empty() {
                // Unit variant:
                pair_unit_variant(ctx, &attrs, path)
            } else {
                // Not a unit variant:
                derive_variant_with_fields(ctx, ut, path, attrs, fields, &mut acc)?
            };
            acc = acc.add_strat((strat, (weight, ctor)));
        }
    }

    /*
    let acc = inhabited.try_fold(acc, |mut acc,| {
    })?;
    */

    ensure_union_has_strategies(ctx, &acc.strats);

    // Package the strategies into a union.
    Ok(acc.finish())
}

/// Ensure that there's at least one generatable variant for a union.
fn ensure_union_has_strategies<C>(ctx: Ctx, strats: &StratAcc<C>) {
    if strats.is_empty() {
        // We didn't accumulate any strategies,
        // so we can't construct any variant.
        error::uninhabited_enum_because_of_skipped_variants(ctx);
    }
}

/// Derive for a variant which has fields and where the
/// variant or its fields may specify `params` or `no_params`.
fn derive_variant_with_fields<C>
    (ctx: Ctx,
     ut: &mut UseTracker, v_path: syn::Path, attrs: ParsedAttributes,
     fields: Vec<syn::Field>, acc: &mut PartsAcc<C>)
    -> DeriveResult<StratPair>
{
    let r = match attrs.params {
        // Parameters were not set on the field:
        ParamsMode::Passthrough => match attrs.strategy {
            // Specific strategy - use the given expr and erase the type:
            StratMode::Strategy(strat) => {
                deny_all_attrs_on_fields(ctx, fields)?;
                pair_existential_self(strat)
            },
            // Specific value - use the given expr:
            StratMode::Value(value) => {
                deny_all_attrs_on_fields(ctx, fields)?;
                pair_value_self(value)
            },
            // Use Arbitrary for the factors (fields) of variant:
            StratMode::Arbitrary => {
                // Compute parts for the inner product:
                let closure = map_closure(v_path, &fields);
                let fields_acc = derive_product_no_params(ctx, ut, fields,
                                    error::ENUM_VARIANT_FIELD)?;
                let (params, count) = fields_acc.params.consume();
                let (strat, ctor) = fields_acc.strats.finish(closure);

                // Add params types from inner derive as a single type
                // in the outer params types.
                let params_ty = params.into();
                (strat, if is_unit_type(&params_ty) { ctor } else {
                    let pref = acc.add_param(params_ty);
                    if pref + 1 == count {
                        ctor
                    } else {
                        extract_all(ctor, count, FromReg::Num(pref))
                    }
                })
            },
        },
        // no_params set on the field:
        ParamsMode::Default =>
            variant_handle_default_params(ctx, ut, v_path, attrs, fields)?,
        // params(<type>) set on the field:
        ParamsMode::Specified(params_ty) => match attrs.strategy {
            // Specific strategy - use the given expr and erase the type:
            StratMode::Strategy(strat) => {
                deny_all_attrs_on_fields(ctx, fields)?;
                extract_nparam(acc, params_ty, pair_existential_self(strat))
            },
            // Specific value - not allowed yet.
            StratMode::Value(_) =>
                error::cant_set_param_and_value(ctx, error::ENUM_VARIANT)?,
            // Logic error by user. Pointless to specify params and not
            // the strategy. Bail!
            StratMode::Arbitrary => {
                let ty = self_ty();
                error::cant_set_param_but_not_strat(ctx, &ty, error::ENUM_VARIANT)?
            },
        },
    };
    Ok(r)
}

/// Determine strategy using "Default" semantics for a variant.
fn variant_handle_default_params(
    ctx: Ctx,
    ut: &mut UseTracker, v_path: syn::Path, attrs: ParsedAttributes,
    fields: Vec<syn::Field>)
    -> DeriveResult<StratPair> {
    let r = match attrs.strategy {
        // Specific strategy - use the given expr and erase the type:
        StratMode::Strategy(strat) => {
            deny_all_attrs_on_fields(ctx, fields)?;
            pair_existential_self(strat)
        },
        // Specific value - use the given expr:
        StratMode::Value(value) => {
            deny_all_attrs_on_fields(ctx, fields)?;
            pair_value_self(value)
        },
        // Use Arbitrary for the factors (fields) of variant:
        StratMode::Arbitrary =>
            // Fields are not allowed to specify params.
            derive_product_has_params(ctx, ut, error::ENUM_VARIANT_FIELD,
                map_closure(v_path, &fields), fields)?,
    };
    Ok(r)
}

/// Ensures that there are no proptest attributes on any of the fields.
fn deny_all_attrs_on_fields(ctx: Ctx, fields: Vec<syn::Field>) -> DeriveResult<()> {
    fields.into_iter().try_for_each(|field| {
        let f_attr = parse_attributes(ctx, field.attrs)?;
        error::if_anything_specified(ctx, &f_attr, error::ENUM_VARIANT_FIELD)
    })
}

/// Derive for a variant which has fields and where the
/// variant or its fields may NOT specify `params` or `no_params`.
fn derive_enum_has_params(
    ctx: Ctx,
    ut: &mut UseTracker, _self: &syn::Ident, variants: Vec<syn::Variant>,
    sty: Option<syn::Type>)
    -> DeriveResult<ImplParts>
{
    /*
    // Keep the inhabited variants:
    let inhabited = variants.into_iter().filter_map(|var|
        keep_inhabited_variant(ctx, _self, var)?
    );
    */

    // Initialize the accumulator:
    let mut acc = StratAcc::new(variants.len());

    // Fold into the accumulator the strategies for each variant:
    for variant in variants {
        let parts = keep_inhabited_variant(ctx, _self, variant)?;
        if let Some((weight, ident, fields, attrs)) = parts {
            let path = parse_quote!( #_self::#ident );
            let (strat, ctor) = if fields.is_empty() {
                // Unit variant:
                pair_unit_variant(ctx, &attrs, path)
            } else {
                // Not a unit variant:
                variant_handle_default_params(ctx, ut, path, attrs, fields)?
            };
            acc = acc.add((strat, (weight, ctor)));
        }
    }

    ensure_union_has_strategies(ctx, &acc);

    Ok(add_top_params(sty, acc.finish()))
}

/// Filters out uninhabited and variants that we've been ordered to skip.
fn keep_inhabited_variant(ctx: Ctx, _self: &syn::Ident, variant: syn::Variant)
    -> DeriveResult<Option<(u32, syn::Ident, Vec<syn::Field>, ParsedAttributes)>>
{
    use void::IsUninhabited;

    let attrs = parse_attributes(ctx, variant.attrs)?;
    let fields = fields_to_vec(variant.fields);

    if attrs.skip {
        // We've been ordered to skip this variant!
        // Check that all other attributes are not set.
        ensure_has_only_skip_attr(ctx, attrs, error::ENUM_VARIANT);
        fields.into_iter().try_for_each(|field| {
            let f_attrs = parse_attributes(ctx, field.attrs)?;
            error::if_skip_present(ctx, &f_attrs, error::ENUM_VARIANT_FIELD);
            ensure_has_only_skip_attr(ctx, f_attrs, error::ENUM_VARIANT_FIELD);
            Ok(())
        })?;

        return Ok(None)
    }

    // If the variant is uninhabited, we can't generate it, so skip it.
    if (&*fields).is_uninhabited() { return Ok(None) }

    // Compute the weight:
    let weight = attrs.weight.unwrap_or(1);

    Ok(Some((weight, variant.ident, fields, attrs)))
}

/// Ensures that no other attributes than skip are present.
fn ensure_has_only_skip_attr(ctx: Ctx, attrs: ParsedAttributes, item: &str) {
    if attrs.params.is_set() {
        error::skipped_variant_has_param(ctx, item);
    }

    if attrs.strategy.is_set() {
        error::skipped_variant_has_strat(ctx, item);
    }

    if attrs.weight.is_some() {
        error::skipped_variant_has_weight(ctx, item);
    }
}

/// Deal with a unit variant.
fn pair_unit_variant(ctx: Ctx, attrs: &ParsedAttributes, v_path: syn::Path)
    -> StratPair
{
    error::if_strategy_present_on_unit_variant(ctx, attrs);
    error::if_params_present_on_unit_variant(ctx, attrs);
    pair_unit_self(v_path)
}

//==============================================================================
// Combined accumulator
//==============================================================================

/// Combined accumulator for the parameters and strategies.
struct PartsAcc<C> {
    /// The accumulator for the parameters.
    params: ParamAcc,
    /// The accumulator for the strategies.
    strats: StratAcc<C>,
}

impl<C> PartsAcc<C> {
    /// Constructs a new accumulator with the size
    /// passed on to the accumulator for the strategies.
    fn new(size: usize) -> Self {
        Self {
            params: ParamAcc::empty(),
            strats: StratAcc::new(size),
        }
    }

    /// Adds a strategy to the accumulator.
    fn add_strat(self, pair: (Strategy, C)) -> Self {
        Self {
            strats: self.strats.add(pair),
            params: self.params
        }
    }

    /// Adds a parameter type to the accumulator and returns how many types
    /// there were before adding.
    fn add_param(&mut self, ty: syn::Type) -> usize {
        self.params.add(ty)
    }
}

impl PartsAcc<Ctor> {
    /// Finishes off the accumulator by returning the parts needed for
    /// deriving. The resulting strategy is a mapping of the parts into
    /// the `Self` type.
    fn finish(self, closure: MapClosure) -> ImplParts {
        let (params, count) = self.params.consume();
        let (strat, ctor) = self.strats.finish(closure);
        (params, strat, extract_all(ctor, count, FromReg::Top))
    }
}

impl PartsAcc<(u32, Ctor)> {
    /// Finishes off the accumulator by returning the parts needed for
    /// deriving. The resultant strategy is one that randomly picks
    /// one of the parts based on the relative weights in the `u32`.
    fn finish(self) -> ImplParts {
        let (params, count) = self.params.consume();
        let (strat, ctor) = self.strats.finish();
        (params, strat, extract_all(ctor, count, FromReg::Top))
    }
}

//==============================================================================
// Param accumulator
//==============================================================================

/// Accumulator of the parameter types.
struct ParamAcc {
    /// The accumulated parameters types.
    types: Params,
}

impl ParamAcc {
    /// Returns an empty accumulator.
    fn empty() -> Self {
        Self { types: Params::empty(), }
    }

    /// Adds a type to the accumulator and returns the type count before adding.
    fn add(&mut self, ty: syn::Type) -> usize {
        let var = self.types.len();
        self.types += ty;
        var
    }

    /// Consumes the accumulator returning the types and the count.
    fn consume(self) -> (Params, usize) {
        let count = self.types.len();
        (self.types, count)
    }
}

//==============================================================================
// Strategy accumulator
//==============================================================================

/// Accumulator of a sequence of strategies (both type and constructor).
struct StratAcc<C> {
    /// The type half of the accumulator:
    types: Vec<Strategy>,
    /// The constructors (Rust expression that makes the strategy) half:
    ctors: Vec<C>,
}

impl<C> StratAcc<C> {
    /// Construct the given accumulator with
    /// initial capacity according to `size`.
    fn new(size: usize) -> Self {
        Self {
            types: Vec::with_capacity(size),
            ctors: Vec::with_capacity(size),
        }
    }

    /// Add the given type and constructor pair to
    /// the accumulator which is moved and returned.
    fn add(mut self, (strat, ctor): (Strategy, C)) -> Self {
        self.types.push(strat);
        self.ctors.push(ctor);
        self
    }

    /// Consume the accumulator returning the:
    /// + sequence of strategies
    /// + sequence of constructors
    fn consume(self) -> (Vec<Strategy>, Vec<C>) {
        (self.types, self.ctors)
    }

    /// Returns `true` iff nothing has been accumulated yet.
    fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

impl StratAcc<Ctor> {
    /// Finishes off the accumulator by returning
    /// a `.prop_map(<closure>)` of the strategies.
    fn finish(self, closure: MapClosure) -> StratPair {
        pair_map(self.consume(), closure)
    }
}

impl StratAcc<(u32, Ctor)> {
    /// Finishes off the accumulator by returning a union of the
    /// strategies where the resultant strategy randomly picks
    /// one of the summands based on the relative weights provided.
    fn finish(self) -> StratPair {
        pair_oneof(self.consume())
    }
}
