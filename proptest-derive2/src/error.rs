// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides error messages and some checkers

#![warn(missing_docs)]

use proc_macro2::TokenStream;

use syn;
use attr::ParsedAttributes;

//==============================================================================
// Item descriptions
//==============================================================================

/// Item name of structs.
pub const STRUCT: &'static str = "struct";

/// Item name of struct fields.
pub const STRUCT_FIELD: &'static str = "struct field";

/// Item name of enums.
pub const ENUM: &'static str = "enum";

/// Item name of enum variants.
pub const ENUM_VARIANT: &'static str = "enum variant";

/// Item name of enum variant fields.
pub const ENUM_VARIANT_FIELD: &'static str = "enum variant field";

/// Item name for a type variable.
pub const TY_VAR: &'static str = "a type variable";

//==============================================================================
// Checkers
//==============================================================================

/// Ensures that the type is not parametric over lifetimes.
pub fn if_has_lifetimes(ctx: Ctx, ast: &syn::DeriveInput) {
    if ast.generics.lifetimes().count() > 0 {
        has_lifetimes(ctx);
    }
}

/// Ensures that no attributes were specified on `item`.
pub fn if_anything_specified(ctx: Ctx, attrs: &ParsedAttributes, item: &str)
    -> DeriveResult<()>
{
    if_enum_attrs_present(ctx, attrs, item);
    if_strategy_present(ctx, attrs, item);
    if_specified_params(ctx, attrs, item)?;
    Ok(())
}

/// Ensures that things only allowed on an enum variant is not present on
/// `item` which is not an enum variant.
pub fn if_enum_attrs_present(ctx: Ctx, attrs: &ParsedAttributes, item: &str) {
    if_skip_present(ctx, attrs, item);
    if_weight_present(ctx, attrs, item);
}

/// Ensures that parameters is not present on `item`.
pub fn if_specified_params(ctx: Ctx, attrs: &ParsedAttributes, item: &str)
    -> DeriveResult<()>
{
    if attrs.params.is_set() { parent_has_param(ctx, item)?; }
    Ok(())
}

/// Ensures that an explicit strategy or value is not present on `item`.
pub fn if_strategy_present(ctx: Ctx, attrs: &ParsedAttributes, item: &str) {
    use attr::StratMode::*;
    match attrs.strategy {
        Arbitrary   => {},
        Strategy(_) => illegal_strategy(ctx, "strategy", item),
        Value(_)    => illegal_strategy(ctx, "value", item),
    }
}

/// Ensures that an explicit strategy or value is not present on a unit variant.
pub fn if_strategy_present_on_unit_variant(ctx: Ctx, attrs: &ParsedAttributes) {
    use attr::StratMode::*;
    match attrs.strategy {
        Arbitrary   => {},
        Strategy(_) => strategy_on_unit_variant(ctx, "strategy"),
        Value(_)    => strategy_on_unit_variant(ctx, "value"),
    }
}

/// Ensures that parameters is not present on a unit variant.
pub fn if_params_present_on_unit_variant(ctx: Ctx, attrs: &ParsedAttributes) {
    if attrs.params.is_set() {
        params_on_unit_variant(ctx)
    }
}

/// Ensures that parameters is not present on a unit struct.
pub fn if_params_present_on_unit_struct(ctx: Ctx, attrs: &ParsedAttributes) {
    if attrs.params.is_set() {
        params_on_unit_struct(ctx)
    }
}

/// Ensures that skip is not present on `item`.
pub fn if_skip_present(ctx: Ctx, attrs: &ParsedAttributes, item: &str) {
    if attrs.skip {
        illegal_skip(ctx, item)
    }
}

/// Ensures that a weight is not present on `item`.
pub fn if_weight_present(ctx: Ctx, attrs: &ParsedAttributes, item: &str) {
    if attrs.weight.is_some() {
        illegal_weight(ctx, item)
    }
}

//==============================================================================
// Messages
//==============================================================================

use std::fmt::Display;

#[derive(Debug)]
pub struct Fatal;
pub type DeriveResult<T> = Result<T, Fatal>;
pub type Ctx<'ctx> = &'ctx mut Context;

pub struct Context {
    errors: Vec<String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            //RefCell::new(Some(Vec::new())),
        }
    }

    pub fn error<T: Display>(&mut self, msg: T) {
        self.errors.push(msg.to_string());
    }

    pub fn fatal<T: Display, A>(&mut self, msg: T) -> DeriveResult<A> {
        self.error(msg);
        Err(Fatal)
    }

    pub fn check(mut self) -> Result<(), TokenStream> {
        fn compile_error(msg: String) -> TokenStream {
            quote! {
                compile_error!(#msg);
            }
        }

        match self.errors.len() {
            0 => Ok(()),
            1 => Err(compile_error(self.errors.pop().unwrap())),
            n => {
                let mut msg = format!("{} errors:", n);
                for err in self.errors {
                    msg.push_str("\n\t# ");
                    msg.push_str(&err);
                }
                Err(compile_error(msg))
            }
        }
    }
}

//==============================================================================
// Messages
//==============================================================================

macro_rules! mk_err_msg {
    ($code: ident, $msg: expr) => {
        concat!(
            "[proptest_derive, ", stringify!($code), "]",
            " during #[derive(Arbitrary)]:\n",
            $msg,
            " Please see: https://PATH/TO/foo#", stringify!($code),
            " for more information.")
    }
}

/// A macro to emit an error with a code by panicing.
macro_rules! error {
    ($error: ident, $code: ident, $msg: expr) => {
        pub fn $error<T>(ctx: Ctx) -> DeriveResult<T> {
            ctx.fatal(mk_err_msg!($code, $msg))
        }
    };
    (continue $error: ident, $code: ident, $msg: expr) => {
        pub fn $error(ctx: Ctx) {
            ctx.error(mk_err_msg!($code, $msg))
        }
    };
    ($error: ident ($($arg: ident: $arg_ty: ty),*), $code: ident,
     $msg: expr, $($fmt: tt)+) => {
        pub fn $error<T>(ctx: Ctx, $($arg: $arg_ty),*) -> DeriveResult<T> {
            ctx.fatal(format!(mk_err_msg!($code, $msg), $($fmt)+))
        }
    };
    (continue $error: ident ($($arg: ident: $arg_ty: ty),*), $code: ident,
     $msg: expr, $($fmt: tt)+) => {
        pub fn $error(ctx: Ctx, $($arg: $arg_ty),*) {
            ctx.error(format!(mk_err_msg!($code, $msg), $($fmt)+))
        }
    };
}

/// Happens when we've been asked to derive `Arbitrary` for a type
/// that is parametric over lifetimes. Since proptest does not support
/// such types (yet), neither can we.
error!(continue has_lifetimes, E0001,
    "Can't derive `Arbitrary` for types with generic lifetimes, such as: \
    `struct Foo<'a> { bar: &'a str }`. Currently, strategies for such types \
    are impossible to define.");

/// Happens when we've been asked to derive `Arbitrary` for something
/// that is neither an enum nor a struct. Most likely, we've been given
/// a union type. This might be supported in the future, but not yet.
error!(not_struct_or_enum, E0002,
    // Overspecified atm, to catch future support in syn for unions.
    "Deriving is only possible for structs and enums. \
    It is currently not defined unions.");

/// Happens when a struct has at least one field that is uninhabited.
/// There must at least exist one variant that we can construct.
error!(continue uninhabited_struct, E0003,
    "The struct you are deriving `Arbitrary` for is uninhabited since one of \
    its fields is uninhabited. An uninhabited type is by definition impossible \
    to generate.");

/// Happens when an enum has zero variants. Such an enum is obviously
/// uninhabited and can not be constructed. There must at least exist
/// one variant that we can construct.
error!(uninhabited_enum_with_no_variants, E0004, // TODO: intentionally fatal.
    "The enum you are deriving `Arbitrary` for is uninhabited since it has no \
    variants. An example of such an `enum` is: `enum Void {}`. \
    An uninhabited type is by definition impossible to generate.");

/// Happens when an enum is uninhabited due all its variants being
/// uninhabited (why has the user given us such a weird enum?..
/// Nonetheless, we do our best to ensure soundness).
/// There must at least exist one variant that we can construct.
error!(uninhabited_enum_variants_uninhabited, E0005, // TODO: intentionally fatal.
    "The enum you are deriving `Arbitrary` for is uninhabited since all its \
    variants are uninhabited. \
    An uninhabited type is by definition impossible to generate.");

/// Happens when an enum becomes effectively uninhabited due
/// to all inhabited variants having been skipped. There must
/// at least exist one variant that we can construct.
error!(continue uninhabited_enum_because_of_skipped_variants, E0006,
    "The enum you are deriving `Arbitrary` for is uninhabited for all intents \
    and purposes since you have `#[proptest(skip)]`ed all inhabited variants. \
    An uninhabited type is by definition impossible to generate.");

/// Happens when `#[proptest(strategy = "<expr>")]` or
/// `#[proptest(value = "<expr>")]` is specified on an `item`
/// that does not support setting an explicit value or strategy.
/// An enum or struct does not support that.
error!(continue illegal_strategy(attr: &str, item: &str), E0007,
    "`#[proptest({0} = \"<expr>\")]` is not allowed on {1}. Only struct fields, \
    enum variants and fields inside those can use an explicit {0}.",
    attr, item);

/// Happens when `#[proptest(skip)]` is specified on an `item` that does
/// not support skipping. Only enum variants support skipping.
error!(continue illegal_skip(item: &str), E0008,
    "A {} can't be `#[proptest(skip)]`ed, only enum variants can be skipped.",
    item);

/// Happens when `#[proptest(weight = <integer>)]` is specified on an
/// `item` that does not support weighting.
error!(continue illegal_weight(item: &str), E0009,
    // TODO: determine if the form should be allowed on the enum itself.
    "`#[proptest(weight = <integer>)]` is not allowed on {} as it is \
    meaningless. Only enum variants can be assigned weights.",
    item);

/// Happens when `#[proptest(params = <type>)]` is set on `item`
/// but also on the parent of `item`. If the parent has set `params`
/// then that applies, and the `params` on `item` would be meaningless
/// wherefore it is forbidden.
error!(parent_has_param(item: &str), E0010,
    "Can not set the associated type `Parameters` of `Arbitrary` with either \
    `#[proptest(no_params)]` or `#[proptest(params(<type>)]` on {} since it \
    was set on the parent.",
    item);

/// Happens when `#[proptest(params = <type>)]` is set on `item`
/// but not `#[proptest(strategy = <type>)]`.
/// This does not apply to the top level type declaration.
error!(cant_set_param_but_not_strat(self_ty: &syn::Type, item: &str), E0011,
    "Can not set `#[proptest(params = <type>)]` on {0} while not providing a \
    strategy for the {0} to use it since `<{1} as Arbitrary<'a>>::Strategy` \
    may require a different type than the one provided in `<type>`.",
    item, quote! { #self_ty });

/// Happens when `#[proptest(params = <type>)]` and
/// `#[proptest(value = <type>)]` is set together on `item`.
///
/// This temporary restriction is due to the fact that we can't
/// move parameters into function items. Once we get
/// `type Strategy = impl Trait;`, in stable it will be possible
/// to use closures instead and this restriction can be lifted.
error!(cant_set_param_and_value(item: &str), E0012,
    "Can not set `#[proptest(params = <type>)]` on {0} and set a value via \
    `#[proptest(value = <expr>)]` since `move || <expr>` closures can not be \
    coerced into function pointers. This is most likely a temporary \
    restriction while `type Assoc = impl Trait;` is not yet stable.",
    item);

// TODO: OBSOLETE THIS ^^^ ERROR via BoxedStrategy!

/// Happens when the form `#![proptest<..>]` is used. This will probably never
/// happen - but just in case it does, we catch it and emit an error.
error!(continue inner_attr, E0013,
    "Inner attributes `#![proptest(..)]` are not currently supported.");

/// Happens when the form `#[proptest]` is used. The form contains no
/// information for us to process, so we disallow it.
error!(continue bare_proptest_attr, E0014,
    "Bare `#[proptest]` attributes are not allowed.");

/// Happens when the form `#[proptest = <literal>)]` is used.
/// Only the form `#[proptest(<contents>)]` is supported.
error!(continue literal_set_proptest, E0015,
    "The attribute form `#[proptest = <literal>]` is not allowed.");

/// Happens when `<modifier>` in `#[proptest(<modifier>)]` is a literal and
/// not a real modifier.
error!(continue immediate_literals, E0016,
    "Literals immediately inside `#[proptest(..)]` as in \
    `#[proptest(<lit>, ..)]` are not allowed.");

/// Happens when `<modifier>` in `#[proptest(<modifier>)]` is set more than
/// once.
error!(continue set_again(meta: &syn::Meta), E0017,
    "The attribute modifier `{}` inside `#[proptest(..)]` has already been \
    set. To fix the error, please remove at least one such modifier.",
    meta.name());

/// Happens when `<modifier>` in `#[proptest(<modifier>)]` is unknown to
/// us but we can make an educated guess as to what the user meant.
error!(continue did_you_mean(found: &str, expected: &str), E0018,
    "Unknown attribute modifier `{}` inside #[proptest(..)] is not allowed. \
    Did you mean to use `{}` instead?",
    found, expected);

/// Happens when `<modifier>` in `#[proptest(<modifier>)]` is unknown to us.
error!(continue unkown_modifier(modifier: &str), E0018,
    "Unknown attribute modifier `{}` inside `#[proptest(..)]` is not allowed.",
    modifier);

/// Happens when `#[proptest(no_params)]` is malformed.
error!(continue no_params_malformed, E0019,
    "The attribute modifier `no_params` inside `#[proptest(..)]` does not \
    support any further configuration and must be a plain modifier as in \
    `#[proptest(no_params)]`.");

/// Happens when `#[proptest(skip)]` is malformed.
error!(continue skip_malformed, E0020,
    "The attribute modifier `skip` inside `#[proptest(..)]` does not support \
    any further configuration and must be a plain modifier as in \
    `#[proptest(skip)]`.");

/// Happens when `#[proptest(weight..)]` is malformed.
error!(continue weight_malformed(meta: &syn::Meta), E0021,
    "The attribute modifier `{0}` inside `#[proptest(..)]` must have the \
    format `#[proptest({0} = <integer>)]` where `<integer>` is an integer that \
    fits within a `u32`. An example: `#[proptest({0} = 2)]` to set a relative \
    weight of 2.",
    meta.name());

/// Happens when both `#[proptest(params = "<type>")]` and
/// `#[proptest(no_params)]` were specified. They are mutually
/// exclusive choices. The user can resolve this by picking one.
error!(overspecified_param, E0022,
    "Can not set `#[proptest(no_params)]` as well as \
    `#[proptest(params(<type>))]` simultaneously. \
    Please pick one of those attributes.");

/// This happens when `#[proptest(params..)]` is malformed.
/// For example, `#[proptest(params)]` is malformed. Another example is when
/// `<type>` inside `#[proptest(params = "<type>")]` or
/// `#[proptest(params("<type>"))]` is malformed. In other words, `<type>` is
/// not a valid Rust type. Note that `syn` may not cover all valid Rust types.
error!(continue param_malformed, E0023,
    "The attribute modifier `params` inside #[proptest(..)] must have the \
    format `#[proptest(params = \"<type>\")]` where `<type>` is a valid type \
    in Rust. An example: `#[proptest(params = \"ComplexType<Foo>\")]`.");

/// Happens when syn can't interpret <tts> in `#[proptest <tts>]`.
error!(continue no_interp_meta, E0024,
    "The tokens `<tts>` in #[proptest <tts>] do not make for a valid attribute.");

/// Happens when both `#[proptest(strategy..)]` and `#[proptest(value..)]`
/// were specified. They are mutually exclusive choices. The user can resolve
/// this by picking one.
error!(overspecified_strat, E0025,
    "Can not set `#[proptest(value = \"<expr>\")]` as well as \
    `#[proptest(params(strategy = \"<expr>\"))]` simultaneously. \
    Please pick one of those attributes.");

/// Happens when `#[proptest(strategy..)]` or `#[proptest(value..)]` is
/// malformed. For example, `<expr>` inside `#[proptest(strategy = "<expr>")]`
/// or `#[proptest(value = "<expr>")]` is malformed. In other words, `<expr>`
/// is not a valid Rust expression.
error!(continue strategy_malformed(meta: &syn::Meta), E0026,
    "The attribute modifier `{0}` inside `#[proptest(..)]` must have the \
    format `#[proptest({0} = \"<expr>\")]` where `<expr>` is a valid Rust \
    expression.",
    meta.name());

/// Any attributes on a skipped variant has no effect - so we emit this error
/// to the user so that they are aware.
error!(continue skipped_variant_has_weight(item: &str), E0028,
    "A variant has been skipped. Setting `#[proptest(weight = <value>)]` on \
    the {} is meaningless and is not allowed.",
    item);

/// Any attributes on a skipped variant has no effect - so we emit this error
/// to the user so that they are aware.
error!(continue skipped_variant_has_param(item: &str), E0028,
    "A variant has been skipped. Setting `#[proptest(no_param)]` or \
    `#[proptest(params(<type>))]` on the {} is meaningless and is not allowed.",
    item);

/// Any attributes on a skipped variant has no effect - so we emit this error
/// to the user so that they are aware. Unfortunately, there's no way to
/// emit a warning to the user, so we emit an error instead.
error!(continue skipped_variant_has_strat(item: &str), E0028,
    "A variant has been skipped. Setting `#[proptest(value = \"<expr>\")]` or \
    `#[proptest(strategy = \"<expr>\")]` on the {} is meaningless and is not \
    allowed.",
    item);

/// There's only one way to produce a specific unit variant, so setting
/// `#[proptest(strategy = "<expr>")]` or `#[proptest(value = "<expr>")]`
/// would be pointless.
error!(continue strategy_on_unit_variant(what: &str), E0029,
    "Setting `#[proptest({0} = \"<expr>\")]` on a unit variant has no effect \
    and is redundant because there is nothing to configure.",
    what);

/// There's only one way to produce a specific unit variant, so setting
/// `#[proptest(params = "<type>")]` would be pointless.
error!(continue params_on_unit_variant, E0029,
    "Setting `#[proptest(params = \"<type>\")]` on a unit variant has \
    no effect and is redundant because there is nothing to configure.");

/// Occurs when `#[proptest(params = "<type>")]` is specified on a unit
/// struct. There's only one way to produce a unit struct, so specifying
/// `Parameters` would be pointless.
error!(continue params_on_unit_struct, E0030,
    "Setting `#[proptest(params = \"<type>\")]` on a unit struct has no effect \
    and is redundant because there is nothing to configure.");

/// Occurs when `#[proptest(no_bound)]` is specified
/// on something that is not a type variable.
error!(continue no_bound_set_on_non_tyvar, E0031,
    "Setting `#[proptest(no_bound)]` on something that is not a type variable \
    has no effect and is redundant. Therefore it is not allowed.");

/// Happens when `#[proptest(no_bound)]` is malformed.
error!(continue no_bound_malformed, E0032,
    "The attribute modifier `no_bound` inside `#[proptest(..)]` does not \
    support any further configuration and must be a plain modifier as in \
    `#[proptest(no_bound)]`.");
