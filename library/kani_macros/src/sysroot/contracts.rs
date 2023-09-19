// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use proc_macro::TokenStream;

use {
    quote::{quote, ToTokens},
    syn::{parse_macro_input, spanned::Spanned, visit::Visit, Expr, ItemFn},
};

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};

/// Create a unique hash for a token stream (basically a [`std::hash::Hash`]
/// impl for `proc_macro2::TokenStream`).
fn hash_of_token_stream<H: std::hash::Hasher>(hasher: &mut H, stream: proc_macro2::TokenStream) {
    use proc_macro2::TokenTree;
    use std::hash::Hash;
    for token in stream {
        match token {
            TokenTree::Ident(i) => i.hash(hasher),
            TokenTree::Punct(p) => p.as_char().hash(hasher),
            TokenTree::Group(g) => {
                std::mem::discriminant(&g.delimiter()).hash(hasher);
                hash_of_token_stream(hasher, g.stream());
            }
            TokenTree::Literal(lit) => lit.to_string().hash(hasher),
        }
    }
}

use syn::{visit_mut::VisitMut, Attribute, Block, Signature};

/// Hash this `TokenStream` and return an integer that is at most digits
/// long when hex formatted.
fn short_hash_of_token_stream(stream: &proc_macro::TokenStream) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hash_of_token_stream(&mut hasher, proc_macro2::TokenStream::from(stream.clone()));
    let long_hash = hasher.finish();
    long_hash % 0x1_000_000 // six hex digits
}

/// Makes consistent names for a generated function which was created for
/// `purpose`, from an attribute that decorates `related_function` with the
/// hash `hash`.
fn identifier_for_generated_function(related_function: &ItemFn, purpose: &str, hash: u64) -> Ident {
    let identifier = format!("{}_{purpose}_{hash:x}", related_function.sig.ident);
    Ident::new(&identifier, proc_macro2::Span::mixed_site())
}

pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_alt(attr, item, true)
}

pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_alt(attr, item, false)
}

/// Collect all named identifiers used in the argument patterns of a function.
struct ArgumentIdentCollector(HashSet<Ident>);

impl ArgumentIdentCollector {
    fn new() -> Self {
        Self(HashSet::new())
    }
}

impl<'ast> Visit<'ast> for ArgumentIdentCollector {
    fn visit_pat_ident(&mut self, i: &'ast syn::PatIdent) {
        self.0.insert(i.ident.clone());
        syn::visit::visit_pat_ident(self, i)
    }
    fn visit_receiver(&mut self, _: &'ast syn::Receiver) {
        self.0.insert(Ident::new("self", proc_macro2::Span::call_site()));
    }
}

/// Applies the contained renaming (key renamed to value) to every ident pattern
/// and ident expr visited.
struct Renamer<'a>(&'a HashMap<Ident, Ident>);

impl<'a> VisitMut for Renamer<'a> {
    fn visit_expr_path_mut(&mut self, i: &mut syn::ExprPath) {
        if i.path.segments.len() == 1 {
            i.path
                .segments
                .first_mut()
                .and_then(|p| self.0.get(&p.ident).map(|new| p.ident = new.clone()));
        }
    }

    /// This restores shadowing. Without this we would rename all ident
    /// occurrences, but not rebinding location. This is because our
    /// [`visit_expr_path_mut`] is scope-unaware.
    fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
        if let Some(new) = self.0.get(&i.ident) {
            i.ident = new.clone();
        }
    }
}

/// Does the provided path have the same chain of identifiers as `mtch` (match)
/// and no arguments anywhere?
///
/// So for instance (using some pseudo-syntax for the [`syn::Path`]s)
/// `matches_path(std::vec::Vec, &["std", "vec", "Vec"]) == true` but
/// `matches_path(std::Vec::<bool>::contains, &["std", "Vec", "contains"]) !=
/// true`.
///
/// This is intended to be used to match the internal `kanitool` family of
/// attributes which we know to have a regular structure and no arguments.
fn matches_path<E>(path: &syn::Path, mtch: &[E]) -> bool
where
    Ident: std::cmp::PartialEq<E>,
{
    path.segments.len() == mtch.len()
        && path.segments.iter().all(|s| s.arguments.is_empty())
        && path.leading_colon.is_none()
        && path.segments.iter().zip(mtch).all(|(actual, expected)| actual.ident == *expected)
}

/// Temporarily swap `$src` and `$target` using `std::mem::swap` for the
/// execution of `$code`, then swap them back.
macro_rules! swapped {
    ($src:expr, $target:expr, $code:expr) => {{
        std::mem::swap($src, $target);
        let result = $code;
        std::mem::swap($src, $target);
        result
    }};
}

/// Classifies the state a function is in in the contract handling pipeline.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContractFunctionState {
    /// This is the original code, re-emitted from a contract attribute.
    Original,
    /// This is the first time a contract attribute is evaluated on this
    /// function.
    Untouched,
    /// This is a check function that was generated from a previous evaluation
    /// of a contract attribute.
    Check,
    /// This is a replace function that was generated from a previous evaluation
    /// of a contract attribute.
    Replace,
}

impl ContractFunctionState {
    /// Find out if this attribute could be describing a "contract handling"
    /// state and if so return it.
    fn from_attribute(attribute: &syn::Attribute) -> Option<Self> {
        if let syn::Meta::List(lst) = &attribute.meta {
            if matches_path(&lst.path, &["kanitool", "is_contract_generated"]) {
                match syn::parse2::<Ident>(lst.tokens.clone()) {
                    Err(e) => {
                        lst.span().unwrap().error(format!("{e}")).emit();
                    }
                    Ok(ident) => {
                        let ident_str = ident.to_string();
                        return match ident_str.as_str() {
                            "check" => Some(Self::Check),
                            "replace" => Some(Self::Replace),
                            _ => {
                                lst.span()
                                    .unwrap()
                                    .error("Expected `check` or `replace` ident")
                                    .emit();
                                None
                            }
                        };
                    }
                }
            }
        }
        if let syn::Meta::NameValue(nv) = &attribute.meta {
            if matches_path(&nv.path, &["kanitool", "checked_with"]) {
                return Some(ContractFunctionState::Original);
            }
        }
        None
    }

    /// Do we need to emit the `is_contract_generated` tag attribute on the
    /// generated function(s)?
    fn emit_tag_attr(self) -> bool {
        matches!(self, ContractFunctionState::Untouched)
    }

    /// This function decides whether we will be emitting a check function, a
    /// replace function or both and emit a header into `output` if necessary.
    ///
    /// The return of this function essentially configures all the later parts
    /// of code generation and is structured as follows:
    /// `Some((Some((replace_function_name, use_dummy_function)),
    /// Some(check_function_name)))`. Either function name being present tells
    /// the codegen that that type of function should be emitted with the
    /// respective name. `use_dummy_function` indicates whether we should use
    /// the body of this function (`false`) or `kani::any` (`true`) as the
    /// nested body of the replace function. `kani::any` is only used when we
    /// generate a replace function for the first time.
    ///
    /// The following is going to happen depending on the state of `self`
    ///
    /// - On [`ContractFunctionState::Original`] we return an overall [`None`]
    ///   indicating to short circuit the code generation.
    /// - On [`ContractFunctionState::Replace`] and
    ///   [`ContractFunctionState::Check`] we return [`Some`] for one of the
    ///   tuple fields, indicating that only this type of function should be
    ///   emitted.
    /// - On [`ContractFunctionState::Untouched`] we return [`Some`] for both
    ///   tuple fields, indicating that both functions need to be emitted. We
    ///   also emit the original function with the `checked_with` and
    ///   `replaced_with` attributes added.
    ///
    /// The only reason the `item_fn` is mutable is I'm using `std::mem::swap`
    /// to avoid making copies.
    fn prepare_header(
        self,
        item_fn: &mut ItemFn,
        output: &mut TokenStream2,
        a_short_hash: u64,
    ) -> Option<(Option<(Ident, bool)>, Option<Ident>)> {
        match self {
            ContractFunctionState::Untouched => {
                // We are the first time a contract is handled on this function, so
                // we're responsible for
                //
                // 1. Generating a name for the check function
                // 2. Emitting the original, unchanged item and register the check
                //    function on it via attribute
                // 3. Renaming our item to the new name
                // 4. And (minor point) adding #[allow(dead_code)] and
                //    #[allow(unused_variables)] to the check function attributes

                let check_fn_name =
                    identifier_for_generated_function(item_fn, "check", a_short_hash);
                let replace_fn_name =
                    identifier_for_generated_function(item_fn, "replace", a_short_hash);

                // Constructing string literals explicitly here, because `stringify!`
                // doesn't work. Let's say we have an identifier `check_fn` and we were
                // to do `quote!(stringify!(check_fn))` to try to have it expand to
                // `"check_fn"` in the generated code. Then when the next macro parses
                // this it will *not* see the literal `"check_fn"` as you may expect but
                // instead the *expression* `stringify!(check_fn)`.
                let replace_fn_name_str =
                    syn::LitStr::new(&replace_fn_name.to_string(), Span::call_site());
                let check_fn_name_str =
                    syn::LitStr::new(&check_fn_name.to_string(), Span::call_site());

                // The order of `attrs` and `kanitool::{checked_with,
                // is_contract_generated}` is important here, because macros are
                // expanded outside in. This way other contract annotations in `attrs`
                // sees those attributes and can use them to determine
                // `function_state`.
                //
                // We're emitting the original here but the same applies later when we
                // emit the check function.
                let mut attrs = vec![];
                swapped!(&mut item_fn.attrs, &mut attrs, {
                    output.extend(quote!(
                        #(#attrs)*
                        #[kanitool::checked_with = #check_fn_name_str]
                        #[kanitool::replaced_with = #replace_fn_name_str]
                        #item_fn
                    ));
                });
                Some((Some((replace_fn_name, true)), Some(check_fn_name)))
            }
            ContractFunctionState::Original => None,
            ContractFunctionState::Check => Some((None, Some(item_fn.sig.ident.clone()))),
            ContractFunctionState::Replace => {
                Some((Some((item_fn.sig.ident.clone(), false)), None))
            }
        }
    }
}

/// A visitor which injects a copy of the token stream it holds before every
/// `return` expression.
///
/// This is intended to be used with postconditions and for that purpose it also
/// performs a rewrite where the return value is first bound to `result` so the
/// postconditions can access it.
///
/// # Example
///
/// The expression `return x;` turns into
///
/// ```rs
/// { // Always opens a new block
///     let result = x;
///     <injected tokenstream>
///     return result;
/// }
/// ```
struct PostconditionInjector(TokenStream2);

impl VisitMut for PostconditionInjector {
    /// We leave this emtpy to stop the recursion here. We don't want to look
    /// inside the closure, because the return statements contained within are
    /// for a different function.
    fn visit_expr_closure_mut(&mut self, _: &mut syn::ExprClosure) {}

    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let syn::Expr::Return(r) = i {
            let tokens = self.0.clone();
            let mut output = TokenStream2::new();
            if let Some(expr) = &mut r.expr {
                // In theory the return expression can contain itself a `return`
                // so we need to recurse here.
                self.visit_expr_mut(expr);
                output.extend(quote!(let result = #expr;));
                *expr = Box::new(Expr::Verbatim(quote!(result)));
            }
            *i = syn::Expr::Verbatim(quote!({
                #output
                #tokens
                #i
            }))
        } else {
            syn::visit_mut::visit_expr_mut(self, i)
        }
    }
}

/// A supporting function for creating shallow, unsafe copies of the arguments
/// for the postconditions.
///
/// This function:
/// - Collects all [`Ident`]s found in the argument patterns;
/// - Creates new names for them;
/// - Replaces all occurrences of those idents in `attrs` with the new names and;
/// - Returns the mapping of old names to new names.
fn rename_argument_occurrences(sig: &syn::Signature, attr: &mut Expr) -> HashMap<Ident, Ident> {
    let mut arg_ident_collector = ArgumentIdentCollector::new();
    arg_ident_collector.visit_signature(&sig);

    let mk_new_ident_for = |id: &Ident| Ident::new(&format!("{}_renamed", id), Span::mixed_site());
    let arg_idents = arg_ident_collector
        .0
        .into_iter()
        .map(|i| {
            let new = mk_new_ident_for(&i);
            (i, new)
        })
        .collect::<HashMap<_, _>>();

    let mut ident_rewriter = Renamer(&arg_idents);
    ident_rewriter.visit_expr_mut(attr);
    arg_idents
}

/// The information needed to generate the bodies of check and replacement
/// functions that integrate the conditions from this contract attribute.
struct ContractConditionsHandler {
    /// Information specific to the type of contract attribute we're expanding.
    condition_type: ContractConditionsType,
    /// The contents of the attribute.
    attr: Expr,
    /// Body of the function this attribute was found on.
    body: Block,
    /// An unparsed, unmodified copy of `attr`, used in the error messages.
    attr_copy: TokenStream2,
}

/// Information needed for generating check and replace handlers for different
/// contract attributes.
enum ContractConditionsType {
    Requires,
    Ensures {
        /// Translation map from original argument names to names of the copies
        /// we will be emitting.
        argument_names: HashMap<Ident, Ident>,
    },
}

impl ContractConditionsType {
    /// Constructs a [`Self::Ensures`] from the signature of the decorated
    /// function and the contents of the decorating attribute.
    ///
    /// Renames the [`Ident`]s used in `attr` and stores the translation map in
    /// `argument_names`.
    fn new_ensures(sig: &Signature, attr: &mut Expr) -> Self {
        let argument_names = rename_argument_occurrences(sig, attr);

        ContractConditionsType::Ensures { argument_names }
    }
}

impl ContractConditionsHandler {
    /// Initialize the handler. Constructs the required
    /// [`ContractConditionsType`] depending on `is_requires`.
    fn new(
        is_requires: bool,
        mut attr: Expr,
        fn_sig: &Signature,
        fn_body: Block,
        attr_copy: TokenStream2,
    ) -> Self {
        let condition_type = if is_requires {
            ContractConditionsType::Requires
        } else {
            ContractConditionsType::new_ensures(fn_sig, &mut attr)
        };

        Self { condition_type, attr, body: fn_body, attr_copy }
    }

    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    fn make_check_body(&self, sig: &Signature) -> TokenStream2 {
        let attr = &self.attr;
        let attr_copy = &self.attr_copy;
        let call_to_prior = &self.body;
        let return_type = return_type_to_type(&sig.output);
        match &self.condition_type {
            ContractConditionsType::Requires => quote!(
                kani::assume(#attr);
                #call_to_prior
            ),
            ContractConditionsType::Ensures { argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                let attr = &self.attr;

                // The code that enforces the postconditions and cleans up the shallow
                // argument copies (with `mem::forget`).
                let exec_postconditions = quote!(
                    kani::assert(#attr, stringify!(#attr_copy));
                    #copy_clean
                );

                // We make a copy here because we'll modify it. Technically not
                // necessary but could lead to weird results if
                // `make_replace_body` were called after this if we modified in
                // place.
                let mut call = call_to_prior.clone();

                let mut inject_conditions = PostconditionInjector(exec_postconditions.clone());
                inject_conditions.visit_block_mut(&mut call);
                quote!(
                    #arg_copies
                    let result : #return_type = #call;
                    #exec_postconditions
                    result
                )
            }
        }
    }

    /// Create the body of a stub for this contract.
    ///
    /// Wraps the conditions from this attribute around a prior call. If
    /// `use_dummy_fn` is `true` the prior call we wrap is `kani::any`,
    /// otherwise `self.body`.
    fn make_replace_body(&self, sig: &syn::Signature, use_dummy_fn_call: bool) -> TokenStream2 {
        let attr = &self.attr;
        let attr_copy = &self.attr_copy;
        let call_to_prior =
            if use_dummy_fn_call { quote!(kani::any()) } else { self.body.to_token_stream() };
        let return_type = return_type_to_type(&sig.output);
        match &self.condition_type {
            ContractConditionsType::Requires => quote!(
                kani::assert(#attr, stringify!(#attr_copy));
                #call_to_prior
            ),
            ContractConditionsType::Ensures { argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                quote!(
                    #arg_copies
                    let result: #return_type = #call_to_prior;
                    kani::assume(#attr);
                    #copy_clean
                    result
                )
            }
        }
    }
}

fn return_type_to_type(return_type: &syn::ReturnType) -> Cow<syn::Type> {
    match return_type {
        syn::ReturnType::Default => Cow::Owned(syn::Type::Tuple(syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems: Default::default(),
        })),
        syn::ReturnType::Type(_, typ) => Cow::Borrowed(typ.as_ref()),
    }
}

/// We make shallow copies of the argument for the postconditions in both
/// `requires` and `ensures` clauses and later clean them up.
///
/// This function creates the code necessary to both make the copies (first
/// tuple elem) and to clean them (second tuple elem).
fn make_unsafe_argument_copies(
    renaming_map: &HashMap<Ident, Ident>,
) -> (TokenStream2, TokenStream2) {
    let arg_names = renaming_map.values();
    let also_arg_names = renaming_map.values();
    let arg_values = renaming_map.keys();
    (
        quote!(#(let #arg_names = kani::untracked_deref(&#arg_values);)*),
        quote!(#(std::mem::forget(#also_arg_names);)*),
    )
}

/// The main meat of handling requires/ensures contracts.
///
/// Generates a "check" function used to verify the validity of the contract and
/// a "replace" function that can be used as a stub, generated from the contract
/// that can be used instead of the original function.
///
/// Each clause (requires or ensures) after the first clause will be ignored on
/// the original function (detected by finding the `kanitool::checked_with`
/// attribute). On the check function (detected by finding the
/// `kanitool::is_contract_generated` attribute) it expands into a new layer of
/// pre- or postconditions. This state machine is also explained in more detail
/// in comments in the body of this macro.
///
/// All named arguments of the function are unsafely shallow-copied with the
/// `kani::untracked_deref` function to circumvent the borrow checker for
/// postconditions. We must ensure that those copies are not dropped (causing a
/// double-free) so after the postconditions we call `mem::forget` on each copy.
///
/// ## Check function
///
/// Generates a `check_<fn_name>_<fn_hash>` function that assumes preconditions
/// and asserts postconditions. The check function is also marked as generated
/// with the `#[kanitool::is_contract_generated(check)]` attribute.
///
/// Decorates the original function with `#[kanitool::checked_by =
/// "check_<fn_name>_<fn_hash>"]`.
///
/// The check function is a copy of the original function with preconditions
/// added before the body and postconditions after as well as injected before
/// every `return` (see [`PostconditionInjector`]). Attributes on the original
/// function are also copied to the check function.
///
/// ## Replace Function
///
/// As the mirror to that also generates a `replace_<fn_name>_<fn_hash>`
/// function that asserts preconditions and assumes postconditions. The replace
/// function is also marked as generated with the
/// `#[kanitool::is_contract_generated(replace)]` attribute.
///
/// Decorates the original function with `#[kanitool::replaced_by =
/// "replace_<fn_name>_<fn_hash>"]`.
///
/// The replace function has the same signature as the original function but its
/// body is replaced by `kani::any()`, which generates a non-deterministic
/// value.
///
/// # Complete example
///
/// ```rs
/// #[kani::requires(divisor != 0)]
/// #[kani::ensures(result <= dividend)]
/// fn div(dividend: u32, divisor: u32) -> u32 {
///     dividend / divisor
/// }
/// ```
///
/// Turns into
///
/// ```rs
/// #[kanitool::checked_with = "div_check_965916"]
/// #[kanitool::replaced_with = "div_replace_965916"]
/// fn div(dividend: u32, divisor: u32) -> u32 { dividend / divisor }
///
/// #[allow(dead_code)]
/// #[allow(unused_variables)]
/// #[kanitool::is_contract_generated(check)]
/// fn div_check_965916(dividend: u32, divisor: u32) -> u32 {
///     let dividend_renamed = kani::untracked_deref(&dividend);
///     let divisor_renamed = kani::untracked_deref(&divisor);
///     let result = { kani::assume(divisor != 0); { dividend / divisor } };
///     kani::assert(result <= dividend_renamed, "result <= dividend");
///     std::mem::forget(dividend_renamed);
///     std::mem::forget(divisor_renamed);
///     result
/// }
///
/// #[allow(dead_code)]
/// #[allow(unused_variables)]
/// #[kanitool::is_contract_generated(replace)]
/// fn div_replace_965916(dividend: u32, divisor: u32) -> u32 {
///     kani::assert(divisor != 0, "divisor != 0");
///     let dividend_renamed = kani::untracked_deref(&dividend);
///     let divisor_renamed = kani::untracked_deref(&divisor);
///     let result = kani::any();
///     kani::assume(result <= dividend_renamed, "result <= dividend");
///     std::mem::forget(dividend_renamed);
///     std::mem::forget(divisor_renamed);
///     result
/// }
/// ```
fn requires_ensures_alt(attr: TokenStream, item: TokenStream, is_requires: bool) -> TokenStream {
    let attr_copy = TokenStream2::from(attr.clone());
    let attr = parse_macro_input!(attr as Expr);

    let mut output = proc_macro2::TokenStream::new();

    let a_short_hash = short_hash_of_token_stream(&item);
    let mut item_fn = parse_macro_input!(item as ItemFn);

    // If we didn't find any other contract handling related attributes we
    // assume this function has not been touched by a contract before.
    let function_state = item_fn
        .attrs
        .iter()
        .find_map(ContractFunctionState::from_attribute)
        .unwrap_or(ContractFunctionState::Untouched);

    let Some((emit_replace, emit_check)) =
        function_state.prepare_header(&mut item_fn, &mut output, a_short_hash)
    else {
        // If we're the original function that means we're *not* the first time
        // that a contract attribute is handled on this function. This means
        // there must exist a generated check function somewhere onto which the
        // attributes have been copied and where they will be expanded into more
        // checks. So we just return outselves unchanged.
        return item_fn.into_token_stream().into();
    };

    let ItemFn { attrs, vis: _, mut sig, block } = item_fn;
    let handler = ContractConditionsHandler::new(is_requires, attr, &sig, *block, attr_copy);
    let emit_common_header = |output: &mut TokenStream2| {
        if function_state.emit_tag_attr() {
            output.extend(quote!(
                    #[allow(dead_code, unused_variables)]
            ));
        }
        output.extend(attrs.iter().flat_map(Attribute::to_token_stream));
    };

    if let Some((replace_name, dummy)) = emit_replace {
        emit_common_header(&mut output);

        if function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            output.extend(quote!(#[kanitool::is_contract_generated(replace)]));
        }
        let body = handler.make_replace_body(&sig, dummy);
        sig.ident = replace_name;

        // Finally emit the check function itself.
        output.extend(quote!(
            #sig {
                #body
            }
        ));
    }

    if let Some(check_name) = emit_check {
        emit_common_header(&mut output);

        if function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            output.extend(quote!(#[kanitool::is_contract_generated(check)]));
        }
        let body = handler.make_check_body(&sig);
        sig.ident = check_name;
        output.extend(quote!(
            #sig {
                #body
            }
        ))
    }

    output.into()
}

/// This is very similar to the kani_attribute macro, but it instead creates
/// key-value style attributes which I find a little easier to parse.
macro_rules! passthrough {
    ($name:ident, $allow_dead_code:ident) => {
        pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
            let args = proc_macro2::TokenStream::from(attr);
            let fn_item = proc_macro2::TokenStream::from(item);
            let name = Ident::new(stringify!($name), proc_macro2::Span::call_site());
            let extra_attrs = if $allow_dead_code {
                quote!(#[allow(dead_code)])
            } else {
                quote!()
            };
            quote!(
                #extra_attrs
                #[kanitool::#name = stringify!(#args)]
                #fn_item
            )
            .into()
        }
    }
}

passthrough!(stub_verified, false);
passthrough!(proof_for_contract, true);
