// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for processing Rust attributes (like `kani::proof`).

use std::collections::BTreeMap;

use kani_metadata::{CbmcSolver, HarnessAttributes, Stub};
use rustc_ast::{
    attr,
    token::Token,
    token::TokenKind,
    tokenstream::{TokenStream, TokenTree},
    AttrArgs, AttrArgsEq, AttrKind, Attribute, ExprKind, LitKind, MetaItem, MetaItemKind,
    NestedMetaItem,
};
use rustc_errors::ErrorGuaranteed;
use rustc_hir::{
    def::DefKind,
    def_id::{DefId, LocalDefId},
};
use rustc_middle::ty::{Instance, TyCtxt, TyKind};
use rustc_session::Session;
use rustc_smir::rustc_internal;
use rustc_span::{Span, Symbol};
use stable_mir::mir::mono::Instance as InstanceStable;
use stable_mir::mir::Local;
use stable_mir::CrateDef;
use std::str::FromStr;
use strum_macros::{AsRefStr, EnumString};

use tracing::{debug, trace};

use super::resolve::{self, resolve_fn, ResolveError};

#[derive(Debug, Clone, Copy, AsRefStr, EnumString, PartialEq, Eq, PartialOrd, Ord)]
#[strum(serialize_all = "snake_case")]
enum KaniAttributeKind {
    Proof,
    ShouldPanic,
    Solver,
    Stub,
    /// Attribute used to mark unstable APIs.
    Unstable,
    Unwind,
    /// A sound [`Self::Stub`] that replaces a function by a stub generated from
    /// its contract.
    StubVerified,
    /// A harness, similar to [`Self::Proof`], but for checking a function
    /// contract, e.g. the contract check is substituted for the target function
    /// before the the verification runs.
    ProofForContract,
    /// Attribute on a function with a contract that identifies the code
    /// implementing the check for this contract.
    CheckedWith,
    /// Internal attribute of the contracts implementation that identifies the
    /// name of the function which was generated as the sound stub from the
    /// contract of this function.
    ReplacedWith,
    /// Attribute on a function that was auto-generated from expanding a
    /// function contract.
    IsContractGenerated,
    /// Identifies a set of pointer arguments that should be added to the write
    /// set when checking a function contract. Placed on the inner check function.
    ///
    /// Emitted by the expansion of a `modifies` function contract clause.
    Modifies,
    /// A function used as the inner code of a contract check.
    ///
    /// Contains the original body of the contracted function. The signature is
    /// expanded with additional pointer arguments that are not used in the function
    /// but referenced by the `modifies` annotation.
    InnerCheck,
}

impl KaniAttributeKind {
    /// Returns whether an item is only relevant for harnesses.
    pub fn is_harness_only(self) -> bool {
        match self {
            KaniAttributeKind::Proof
            | KaniAttributeKind::ShouldPanic
            | KaniAttributeKind::Solver
            | KaniAttributeKind::Stub
            | KaniAttributeKind::ProofForContract
            | KaniAttributeKind::StubVerified
            | KaniAttributeKind::Unwind => true,
            KaniAttributeKind::Unstable
            | KaniAttributeKind::ReplacedWith
            | KaniAttributeKind::CheckedWith
            | KaniAttributeKind::Modifies
            | KaniAttributeKind::InnerCheck
            | KaniAttributeKind::IsContractGenerated => false,
        }
    }

    /// Is this an "active" function contract attribute? This means it is
    /// part of the function contract interface *and* it implies that a contract
    /// will be used (stubbed or checked) in some way, thus requiring that the
    /// user activate the unstable feature.
    ///
    /// If we find an "inactive" contract attribute we chose not to error,
    /// because it wouldn't have any effect anyway.
    pub fn demands_function_contract_use(self) -> bool {
        matches!(self, KaniAttributeKind::ProofForContract)
    }

    /// Would this attribute be placed on a function as part of a function
    /// contract. E.g. created by `requires`, `ensures`.
    pub fn is_function_contract(self) -> bool {
        use KaniAttributeKind::*;
        matches!(self, CheckedWith | IsContractGenerated)
    }
}

/// Bundles together common data used when evaluating the attributes of a given
/// function.
#[derive(Clone)]
pub struct KaniAttributes<'tcx> {
    /// Rustc type context/queries
    tcx: TyCtxt<'tcx>,
    /// The function which these attributes decorate.
    item: DefId,
    /// All attributes we found in raw format.
    map: BTreeMap<KaniAttributeKind, Vec<&'tcx Attribute>>,
}

impl<'tcx> std::fmt::Debug for KaniAttributes<'tcx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KaniAttributes")
            .field("item", &self.tcx.def_path_debug_str(self.item))
            .field("map", &self.map)
            .finish()
    }
}

impl<'tcx> KaniAttributes<'tcx> {
    /// Perform preliminary parsing and checking for the attributes on this
    /// function
    pub fn for_instance(tcx: TyCtxt<'tcx>, instance: InstanceStable) -> Self {
        KaniAttributes::for_item(tcx, rustc_internal::internal(instance.def.def_id()))
    }

    pub fn for_item(tcx: TyCtxt<'tcx>, def_id: DefId) -> Self {
        let all_attributes = tcx.get_attrs_unchecked(def_id);
        let map = all_attributes.iter().fold(
            <BTreeMap<KaniAttributeKind, Vec<&'tcx Attribute>>>::default(),
            |mut result, attribute| {
                // Get the string the appears after "kanitool::" in each attribute string.
                // Ex - "proof" | "unwind" etc.
                if let Some(kind) = attr_kind(tcx, attribute) {
                    result.entry(kind).or_default().push(attribute)
                }
                result
            },
        );
        Self { map, tcx, item: def_id }
    }

    /// Expect that at most one attribute of this kind exists on the function
    /// and return it.
    fn expect_maybe_one(&self, kind: KaniAttributeKind) -> Option<&'tcx Attribute> {
        match self.map.get(&kind)?.as_slice() {
            [one] => Some(one),
            _ => {
                self.tcx.sess.err(format!(
                    "Too many {} attributes on {}, expected 0 or 1",
                    kind.as_ref(),
                    self.tcx.def_path_debug_str(self.item)
                ));
                None
            }
        }
    }

    /// Parse, extract and resolve the target of `stub_verified(TARGET)`. The
    /// returned `Symbol` and `DefId` are respectively the name and id of
    /// `TARGET`. The `Span` is that of the contents of the attribute and used
    /// for error reporting.
    fn interpret_stub_verified_attribute(
        &self,
    ) -> Vec<Result<(Symbol, DefId, Span), ErrorGuaranteed>> {
        self.map
            .get(&KaniAttributeKind::StubVerified)
            .map_or([].as_slice(), Vec::as_slice)
            .iter()
            .map(|attr| {
                let name = expect_key_string_value(self.tcx.sess, attr)?;
                let ok = self.resolve_sibling(name.as_str()).map_err(|e| {
                    self.tcx.sess.span_err(
                        attr.span,
                        format!("Failed to resolve replacement function {}: {e}", name.as_str()),
                    )
                })?;
                Ok((name, ok, attr.span))
            })
            .collect()
    }

    /// Parse and extract the `proof_for_contract(TARGET)` attribute. The
    /// returned symbol and DefId are respectively the name and id of `TARGET`,
    /// the span in the span for the attribute (contents).
    pub(crate) fn interpret_the_for_contract_attribute(
        &self,
    ) -> Option<Result<(Symbol, DefId, Span), ErrorGuaranteed>> {
        self.expect_maybe_one(KaniAttributeKind::ProofForContract).map(|target| {
            let name = expect_key_string_value(self.tcx.sess, target)?;
            self.resolve_sibling(name.as_str()).map(|ok| (name, ok, target.span)).map_err(
                |resolve_err| {
                    self.tcx.sess.span_err(
                        target.span,
                        format!(
                            "Failed to resolve checking function {} because {resolve_err}",
                            name.as_str()
                        ),
                    )
                },
            )
        })
    }

    /// Extract the name of the sibling function this function's contract is
    /// checked with (if any).
    ///
    /// `None` indicates this function does not use a contract, `Some(Err(_))`
    /// indicates a contract does exist but an error occurred during resolution.
    pub fn checked_with(&self) -> Option<Result<Symbol, ErrorGuaranteed>> {
        self.expect_maybe_one(KaniAttributeKind::CheckedWith)
            .map(|target| expect_key_string_value(self.tcx.sess, target))
    }

    pub fn inner_check(&self) -> Option<Result<DefId, ErrorGuaranteed>> {
        self.eval_sibling_attribute(KaniAttributeKind::InnerCheck)
    }

    pub fn replaced_with(&self) -> Option<Result<Symbol, ErrorGuaranteed>> {
        self.expect_maybe_one(KaniAttributeKind::ReplacedWith)
            .map(|target| expect_key_string_value(self.tcx.sess, target))
    }

    /// Retrieves the global, static recursion tracker variable.
    pub fn checked_with_id(&self) -> Option<Result<DefId, ErrorGuaranteed>> {
        self.eval_sibling_attribute(KaniAttributeKind::CheckedWith)
    }

    /// Find the `mod` that `self.item` is defined in, then search in the items defined in this
    /// `mod` for an item that is named after the `name` in the `#[kanitool::<kind> = "<name>"]`
    /// annotation on `self.item`.
    ///
    /// This is similar to [`resolve_fn`] but more efficient since it only looks inside one `mod`.
    fn eval_sibling_attribute(
        &self,
        kind: KaniAttributeKind,
    ) -> Option<Result<DefId, ErrorGuaranteed>> {
        use rustc_hir::{Item, ItemKind, Mod, Node};
        self.expect_maybe_one(kind).map(|target| {
            let name = expect_key_string_value(self.tcx.sess, target)?;
            let hir_map = self.tcx.hir();
            let hir_id = self.tcx.local_def_id_to_hir_id(self.item.expect_local());
            let find_in_mod = |md: &Mod<'_>| {
                md.item_ids
                    .iter()
                    .find(|it| hir_map.item(**it).ident.name == name)
                    .unwrap()
                    .hir_id()
            };

            let result = match hir_map.get_parent(hir_id) {
                Node::Item(Item { kind, .. }) => match kind {
                    ItemKind::Mod(m) => find_in_mod(m),
                    ItemKind::Impl(imp) => {
                        imp.items.iter().find(|it| it.ident.name == name).unwrap().id.hir_id()
                    }
                    other => panic!("Odd parent item kind {other:?}"),
                },
                Node::Crate(m) => find_in_mod(m),
                other => panic!("Odd parent node type {other:?}"),
            }
            .expect_owner()
            .def_id
            .to_def_id();
            Ok(result)
        })
    }

    fn resolve_sibling(&self, path_str: &str) -> Result<DefId, ResolveError<'tcx>> {
        resolve_fn(
            self.tcx,
            self.tcx.parent_module_from_def_id(self.item.expect_local()).to_local_def_id(),
            path_str,
        )
    }

    /// Check that all attributes assigned to an item is valid.
    /// Errors will be added to the session. Invoke self.tcx.sess.abort_if_errors() to terminate
    /// the session and emit all errors found.
    pub(super) fn check_attributes(&self) {
        // Check that all attributes are correctly used and well formed.
        let is_harness = self.is_harness();
        for (&kind, attrs) in self.map.iter() {
            let local_error = |msg| self.tcx.sess.span_err(attrs[0].span, msg);

            if !is_harness && kind.is_harness_only() {
                local_error(format!(
                    "the `{}` attribute also requires the `#[kani::proof]` attribute",
                    kind.as_ref()
                ));
            }
            match kind {
                KaniAttributeKind::ShouldPanic => {
                    expect_single(self.tcx, kind, &attrs);
                    attrs.iter().for_each(|attr| {
                        expect_no_args(self.tcx, kind, attr);
                    })
                }
                KaniAttributeKind::Solver => {
                    expect_single(self.tcx, kind, &attrs);
                    attrs.iter().for_each(|attr| {
                        parse_solver(self.tcx, attr);
                    })
                }
                KaniAttributeKind::Stub => {
                    parse_stubs(self.tcx, self.item, attrs);
                }
                KaniAttributeKind::Unwind => {
                    expect_single(self.tcx, kind, &attrs);
                    attrs.iter().for_each(|attr| {
                        parse_unwind(self.tcx, attr);
                    })
                }
                KaniAttributeKind::Proof => {
                    if self.map.contains_key(&KaniAttributeKind::ProofForContract) {
                        local_error(
                            "`proof` and `proof_for_contract` may not be used on the same function.".to_string(),
                        );
                    }
                    expect_single(self.tcx, kind, &attrs);
                    attrs.iter().for_each(|attr| self.check_proof_attribute(attr))
                }
                KaniAttributeKind::Unstable => attrs.iter().for_each(|attr| {
                    let _ = UnstableAttribute::try_from(*attr).map_err(|err| err.report(self.tcx));
                }),
                KaniAttributeKind::ProofForContract => {
                    if self.map.contains_key(&KaniAttributeKind::Proof) {
                        local_error(
                            "`proof` and `proof_for_contract` may not be used on the same function.".to_string(),
                        );
                    }
                    expect_single(self.tcx, kind, &attrs);
                }
                KaniAttributeKind::StubVerified => {
                    expect_single(self.tcx, kind, &attrs);
                }
                KaniAttributeKind::CheckedWith | KaniAttributeKind::ReplacedWith => {
                    self.expect_maybe_one(kind)
                        .map(|attr| expect_key_string_value(&self.tcx.sess, attr));
                }
                KaniAttributeKind::IsContractGenerated => {
                    // Ignored here because this is only used by the proc macros
                    // to communicate with one another. So by the time it gets
                    // here we don't care if it's valid or not.
                }
                KaniAttributeKind::Modifies => {
                    self.modifies_contract();
                }
                KaniAttributeKind::InnerCheck => {
                    self.inner_check();
                }
            }
        }
    }

    /// Check that any unstable API has been enabled. Otherwise, emit an error.
    ///
    /// TODO: Improve error message by printing the span of the harness instead of the definition.
    pub fn check_unstable_features(&self, enabled_features: &[String]) {
        if !matches!(self.tcx.type_of(self.item).skip_binder().kind(), TyKind::FnDef(..)) {
            // Skip closures since it shouldn't be possible to add an unstable attribute to them.
            // We have to explicitly skip them though due to an issue with rustc:
            // https://github.com/model-checking/kani/pull/2406#issuecomment-1534333862
            return;
        }

        // If the `function-contracts` unstable feature is not enabled then no
        // function should use any of those APIs.
        if !enabled_features.iter().any(|feature| feature == "function-contracts") {
            for kind in self.map.keys().copied().filter(|a| a.demands_function_contract_use()) {
                let msg = format!(
                    "Using the {} attribute requires activating the unstable `function-contracts` feature",
                    kind.as_ref()
                );
                if let Some(attr) = self.map.get(&kind).unwrap().first() {
                    self.tcx.sess.span_err(attr.span, msg);
                } else {
                    self.tcx.sess.err(msg);
                }
            }
        }

        if let Some(unstable_attrs) = self.map.get(&KaniAttributeKind::Unstable) {
            for attr in unstable_attrs {
                let unstable_attr = UnstableAttribute::try_from(*attr).unwrap();
                if !enabled_features.contains(&unstable_attr.feature) {
                    // Reached an unstable attribute that was not enabled.
                    self.report_unstable_forbidden(&unstable_attr);
                } else {
                    debug!(enabled=?attr, def_id=?self.item, "check_unstable_features");
                }
            }
        }
    }

    /// Report misusage of an unstable feature that was not enabled.
    fn report_unstable_forbidden(&self, unstable_attr: &UnstableAttribute) -> ErrorGuaranteed {
        let fn_name = self.tcx.def_path_str(self.item);
        self.tcx
            .sess
            .struct_err(format!(
                "Use of unstable feature `{}`: {}",
                unstable_attr.feature, unstable_attr.reason
            ))
            .span_note(
                self.tcx.def_span(self.item),
                format!("the function `{fn_name}` is unstable:"),
            )
            .note(format!("see issue {} for more information", unstable_attr.issue))
            .help(format!("use `-Z {}` to enable using this function.", unstable_attr.feature))
            .emit()
    }

    /// Is this item a harness? (either `proof` or `proof_for_contract`
    /// attribute are present)
    fn is_harness(&self) -> bool {
        self.map.contains_key(&KaniAttributeKind::Proof)
            || self.map.contains_key(&KaniAttributeKind::ProofForContract)
    }

    /// Extract harness attributes for a given `def_id`.
    ///
    /// We only extract attributes for harnesses that are local to the current crate.
    /// Note that all attributes should be valid by now.
    pub fn harness_attributes(&self) -> HarnessAttributes {
        // Abort if not local.
        if !self.item.is_local() {
            panic!("Expected a local item, but got: {:?}", self.item);
        };
        trace!(?self, "extract_harness_attributes");
        assert!(self.is_harness());
        self.map.iter().fold(HarnessAttributes::default(), |mut harness, (kind, attributes)| {
            match kind {
                KaniAttributeKind::ShouldPanic => harness.should_panic = true,
                KaniAttributeKind::Solver => {
                    harness.solver = parse_solver(self.tcx, attributes[0]);
                }
                KaniAttributeKind::Stub => {
                    harness.stubs.extend_from_slice(&parse_stubs(self.tcx, self.item, attributes));
                }
                KaniAttributeKind::Unwind => {
                    harness.unwind_value = parse_unwind(self.tcx, attributes[0])
                }
                KaniAttributeKind::Proof => harness.proof = true,
                KaniAttributeKind::ProofForContract => self.handle_proof_for_contract(&mut harness),
                KaniAttributeKind::StubVerified => self.handle_stub_verified(&mut harness),
                KaniAttributeKind::Unstable => {
                    // Internal attribute which shouldn't exist here.
                    unreachable!()
                }
                KaniAttributeKind::CheckedWith
                | KaniAttributeKind::IsContractGenerated
                | KaniAttributeKind::Modifies
                | KaniAttributeKind::InnerCheck
                | KaniAttributeKind::ReplacedWith => {
                    self.tcx.sess.span_err(self.tcx.def_span(self.item), format!("Contracts are not supported on harnesses. (Found the kani-internal contract attribute `{}`)", kind.as_ref()));
                }
            };
            harness
        })
    }

    fn handle_proof_for_contract(&self, harness: &mut HarnessAttributes) {
        let sess = self.tcx.sess;
        let (name, id, span) = match self.interpret_the_for_contract_attribute() {
            None => unreachable!(
                "impossible, was asked to handle `proof_for_contract` but didn't find such an attribute."
            ),
            Some(Err(_)) => return, // This error was already emitted
            Some(Ok(values)) => values,
        };
        let Some(Ok(replacement_name)) = KaniAttributes::for_item(self.tcx, id).checked_with()
        else {
            sess.struct_span_err(
                span,
                format!(
                    "Failed to check contract: Function `{}` has no contract.",
                    self.item_name(),
                ),
            )
            .span_note(self.tcx.def_span(id), "Try adding a contract to this function.")
            .emit();
            return;
        };
        harness.stubs.push(self.stub_for_relative_item(name, replacement_name));
    }

    fn handle_stub_verified(&self, harness: &mut HarnessAttributes) {
        let sess = self.tcx.sess;
        for contract in self.interpret_stub_verified_attribute() {
            let Ok((name, def_id, span)) = contract else {
                // This error has already been emitted so we can ignore it now.
                // Later the session will fail anyway so we can just
                // optimistically forge on and try to find more errors.
                continue;
            };
            let replacement_name = match KaniAttributes::for_item(self.tcx, def_id).replaced_with()
            {
                None => {
                    sess.struct_span_err(
                        span,
                        format!(
                            "Failed to generate verified stub: Function `{}` has no contract.",
                            self.item_name(),
                        ),
                    )
                    .span_note(
                        self.tcx.def_span(def_id),
                        format!(
                            "Try adding a contract to this function or use the unsound `{}` attribute instead.",
                            KaniAttributeKind::Stub.as_ref(),
                        )
                    )
                    .emit();
                    continue;
                }
                Some(Ok(replacement_name)) => replacement_name,
                Some(Err(_)) => continue,
            };
            harness.stubs.push(self.stub_for_relative_item(name, replacement_name))
        }
    }

    fn item_name(&self) -> Symbol {
        self.tcx.item_name(self.item)
    }

    /// Check that if this item is tagged with a proof_attribute, it is a valid harness.
    fn check_proof_attribute(&self, proof_attribute: &Attribute) {
        let span = proof_attribute.span;
        let tcx = self.tcx;
        expect_no_args(tcx, KaniAttributeKind::Proof, proof_attribute);
        if tcx.def_kind(self.item) != DefKind::Fn {
            tcx.sess.span_err(span, "the `proof` attribute can only be applied to functions");
        } else if tcx.generics_of(self.item).requires_monomorphization(tcx) {
            tcx.sess.span_err(span, "the `proof` attribute cannot be applied to generic functions");
        } else {
            let instance = Instance::mono(tcx, self.item);
            if !super::fn_abi(tcx, instance).args.is_empty() {
                tcx.sess.span_err(span, "functions used as harnesses cannot have any arguments");
            }
        }
    }

    fn stub_for_relative_item(&self, anchor: Symbol, replacement: Symbol) -> Stub {
        let local_id = self.item.expect_local();
        let current_module = self.tcx.parent_module_from_def_id(local_id);
        let replace_str = replacement.as_str();
        let original_str = anchor.as_str();
        let replacement = original_str
            .rsplit_once("::")
            .map_or_else(|| replace_str.to_string(), |t| t.0.to_string() + "::" + replace_str);
        resolve::resolve_fn(self.tcx, current_module.to_local_def_id(), &replacement).unwrap();
        Stub { original: original_str.to_string(), replacement }
    }

    /// Parse and interpret the `kanitool::modifies(var1, var2, ...)` annotation into the vector
    /// `[var1, var2, ...]`.
    pub fn modifies_contract(&self) -> Option<Vec<Local>> {
        let local_def_id = self.item.expect_local();
        self.map.get(&KaniAttributeKind::Modifies).map(|attr| {
            attr.iter()
                .flat_map(|clause| match &clause.get_normal_item().args {
                    AttrArgs::Delimited(lvals) => {
                        parse_modify_values(self.tcx, local_def_id, &lvals.tokens)
                    }
                    _ => unreachable!(),
                })
                .collect()
        })
    }
}

/// Pattern macro for the comma token used in attributes.
macro_rules! comma_tok {
    () => {
        TokenTree::Token(Token { kind: TokenKind::Comma, .. }, _)
    };
}

/// Parse the a token stream inside an attribute (like `kanitool::modifies`) as a comma separated
/// sequence of function parameter names on `local_def_id` (must refer to a function). Then
/// translates the names into [`Local`]s.
fn parse_modify_values<'a>(
    tcx: TyCtxt<'a>,
    local_def_id: LocalDefId,
    t: &'a TokenStream,
) -> impl Iterator<Item = Local> + 'a {
    let mir = tcx.optimized_mir(local_def_id);
    let mut iter = t.trees();
    std::iter::from_fn(move || {
        let tree = iter.next()?;
        let wrong_token_err =
            || tcx.sess.span_err(tree.span(), "Unexpected token. Expected identifier.");
        let result = match tree {
            TokenTree::Token(token, _) => {
                if let TokenKind::Ident(id, _) = &token.kind {
                    let hir = tcx.hir();
                    let bid = hir.body_owned_by(local_def_id);
                    Some(
                        hir.body_param_names(bid)
                            .zip(mir.args_iter())
                            .find(|(name, _decl)| name.name == *id)
                            .unwrap()
                            .1
                            .as_usize(),
                    )
                } else {
                    wrong_token_err();
                    None
                }
            }
            _ => {
                wrong_token_err();
                None
            }
        };
        match iter.next() {
            None | Some(comma_tok!()) => (),
            Some(not_comma) => {
                tcx.sess.span_err(
                    not_comma.span(),
                    "Unexpected token, expected end of attribute or comma",
                );
                iter.by_ref().skip_while(|t| !matches!(t, comma_tok!())).count();
            }
        }
        result
    })
}

/// An efficient check for the existence for a particular [`KaniAttributeKind`].
/// Unlike querying [`KaniAttributes`] this method builds no new heap data
/// structures and has short circuiting.
fn has_kani_attribute<F: Fn(KaniAttributeKind) -> bool>(
    tcx: TyCtxt,
    def_id: DefId,
    predicate: F,
) -> bool {
    tcx.get_attrs_unchecked(def_id).iter().filter_map(|a| attr_kind(tcx, a)).any(predicate)
}

/// Test if this function was generated by expanding a contract attribute like
/// `requires` and `ensures`.
pub fn is_function_contract_generated(tcx: TyCtxt, def_id: DefId) -> bool {
    has_kani_attribute(tcx, def_id, KaniAttributeKind::is_function_contract)
}

/// Same as [`KaniAttributes::is_harness`] but more efficient because less
/// attribute parsing is performed.
pub fn is_proof_harness(tcx: TyCtxt, instance: InstanceStable) -> bool {
    let def_id = rustc_internal::internal(instance.def.def_id());
    has_kani_attribute(tcx, def_id, |a| {
        matches!(a, KaniAttributeKind::Proof | KaniAttributeKind::ProofForContract)
    })
}

/// Does this `def_id` have `#[rustc_test_marker]`?
pub fn is_test_harness_description(tcx: TyCtxt, item: impl CrateDef) -> bool {
    let def_id = rustc_internal::internal(item.def_id());
    let attrs = tcx.get_attrs_unchecked(def_id);
    attr::contains_name(attrs, rustc_span::symbol::sym::rustc_test_marker)
}

/// Extract the test harness name from the `#[rustc_test_maker]`
pub fn test_harness_name(tcx: TyCtxt, def: &impl CrateDef) -> String {
    let def_id = rustc_internal::internal(def.def_id());
    let attrs = tcx.get_attrs_unchecked(def_id);
    let marker = attr::find_by_name(attrs, rustc_span::symbol::sym::rustc_test_marker).unwrap();
    parse_str_value(&marker).unwrap()
}

/// Expect the contents of this attribute to be of the format #[attribute =
/// "value"] and return the `"value"`.
fn expect_key_string_value(
    sess: &Session,
    attr: &Attribute,
) -> Result<rustc_span::Symbol, ErrorGuaranteed> {
    let span = attr.span;
    let AttrArgs::Eq(_, it) = &attr.get_normal_item().args else {
        return Err(sess.span_err(span, "Expected attribute of the form #[attr = \"value\"]"));
    };
    let maybe_str = match it {
        AttrArgsEq::Ast(expr) => {
            if let ExprKind::Lit(tok) = expr.kind {
                match LitKind::from_token_lit(tok) {
                    Ok(l) => l.str(),
                    Err(err) => {
                        return Err(sess.span_err(
                            span,
                            format!("Invalid string literal on right hand side of `=` {err:?}"),
                        ));
                    }
                }
            } else {
                return Err(
                    sess.span_err(span, "Expected literal string as right hand side of `=`")
                );
            }
        }
        AttrArgsEq::Hir(lit) => lit.kind.str(),
    };
    if let Some(str) = maybe_str {
        Ok(str)
    } else {
        Err(sess.span_err(span, "Expected literal string as right hand side of `=`"))
    }
}

fn expect_single<'a>(
    tcx: TyCtxt,
    kind: KaniAttributeKind,
    attributes: &'a Vec<&'a Attribute>,
) -> &'a Attribute {
    let attr = attributes
        .first()
        .expect(&format!("expected at least one attribute {} in {attributes:?}", kind.as_ref()));
    if attributes.len() > 1 {
        tcx.sess.span_err(
            attr.span,
            format!("only one '#[kani::{}]' attribute is allowed per harness", kind.as_ref()),
        );
    }
    attr
}

/// Attribute used to mark a Kani lib API unstable.
#[derive(Debug)]
struct UnstableAttribute {
    /// The feature identifier.
    feature: String,
    /// A link to the stabilization tracking issue.
    issue: String,
    /// A user friendly message that describes the reason why this feature is marked as unstable.
    reason: String,
}

#[derive(Debug)]
struct UnstableAttrParseError<'a> {
    /// The reason why the parsing failed.
    reason: String,
    /// The attribute being parsed.
    attr: &'a Attribute,
}

impl<'a> UnstableAttrParseError<'a> {
    /// Report the error in a friendly format.
    fn report(&self, tcx: TyCtxt) -> ErrorGuaranteed {
        tcx.sess
            .struct_span_err(
                self.attr.span,
                format!("failed to parse `#[kani::unstable]`: {}", self.reason),
            )
            .note(format!(
                "expected format: #[kani::unstable({}, {}, {})]",
                r#"feature="<IDENTIFIER>""#, r#"issue="<ISSUE>""#, r#"reason="<DESCRIPTION>""#
            ))
            .emit()
    }
}

/// Try to parse an unstable attribute into an `UnstableAttribute`.
impl<'a> TryFrom<&'a Attribute> for UnstableAttribute {
    type Error = UnstableAttrParseError<'a>;
    fn try_from(attr: &'a Attribute) -> Result<Self, Self::Error> {
        let build_error = |reason: String| Self::Error { reason, attr };
        let args = parse_key_values(attr).map_err(build_error)?;
        let invalid_keys = args
            .iter()
            .filter_map(|(key, _)| {
                (!matches!(key.as_str(), "feature" | "issue" | "reason")).then_some(key)
            })
            .cloned()
            .collect::<Vec<_>>();

        if !invalid_keys.is_empty() {
            Err(build_error(format!("unexpected argument `{}`", invalid_keys.join("`, `"))))
        } else {
            let get_val = |name: &str| {
                args.get(name).cloned().ok_or(build_error(format!("missing `{name}` field")))
            };
            Ok(UnstableAttribute {
                feature: get_val("feature")?,
                issue: get_val("issue")?,
                reason: get_val("reason")?,
            })
        }
    }
}

fn expect_no_args(tcx: TyCtxt, kind: KaniAttributeKind, attr: &Attribute) {
    if !attr.is_word() {
        tcx.sess
            .struct_span_err(attr.span, format!("unexpected argument for `{}`", kind.as_ref()))
            .help("remove the extra argument")
            .emit();
    }
}

/// Return the unwind value from the given attribute.
fn parse_unwind(tcx: TyCtxt, attr: &Attribute) -> Option<u32> {
    // Get Attribute value and if it's not none, assign it to the metadata
    match parse_integer(attr) {
        None => {
            // There are no integers or too many arguments given to the attribute
            tcx.sess.span_err(
                attr.span,
                "invalid argument for `unwind` attribute, expected an integer",
            );
            None
        }
        Some(unwind_integer_value) => {
            if let Ok(val) = unwind_integer_value.try_into() {
                Some(val)
            } else {
                tcx.sess.span_err(attr.span, "value above maximum permitted value - u32::MAX");
                None
            }
        }
    }
}

fn parse_stubs(tcx: TyCtxt, harness: DefId, attributes: &[&Attribute]) -> Vec<Stub> {
    let current_module = tcx.parent_module_from_def_id(harness.expect_local());
    let check_resolve = |attr: &Attribute, name: &str| {
        let result = resolve::resolve_fn(tcx, current_module.to_local_def_id(), name);
        if let Err(err) = result {
            tcx.sess.span_err(attr.span, format!("failed to resolve `{name}`: {err}"));
        }
    };
    attributes
        .iter()
        .filter_map(|attr| match parse_paths(attr) {
            Ok(paths) => match paths.as_slice() {
                [orig, replace] => {
                    check_resolve(attr, orig);
                    check_resolve(attr, replace);
                    Some(Stub { original: orig.clone(), replacement: replace.clone() })
                }
                _ => {
                    tcx.sess.span_err(
                        attr.span,
                        format!(
                            "attribute `kani::stub` takes two path arguments; found {}",
                            paths.len()
                        ),
                    );
                    None
                }
            },
            Err(error_span) => {
                tcx.sess.span_err(
                    error_span,
                        "attribute `kani::stub` takes two path arguments; found argument that is not a path",
                );
                None
            }
        })
        .collect()
}

fn parse_solver(tcx: TyCtxt, attr: &Attribute) -> Option<CbmcSolver> {
    // TODO: Argument validation should be done as part of the `kani_macros` crate
    // <https://github.com/model-checking/kani/issues/2192>
    const ATTRIBUTE: &str = "#[kani::solver]";
    let invalid_arg_err = |attr: &Attribute| {
        tcx.sess.span_err(
                attr.span,
                format!("invalid argument for `{ATTRIBUTE}` attribute, expected one of the supported solvers (e.g. `kissat`) or a SAT solver binary (e.g. `bin=\"<SAT_SOLVER_BINARY>\"`)")
            )
    };

    let attr_args = attr.meta_item_list().unwrap();
    if attr_args.len() != 1 {
        tcx.sess.span_err(
            attr.span,
            format!(
                "the `{ATTRIBUTE}` attribute expects a single argument. Got {} arguments.",
                attr_args.len()
            ),
        );
        return None;
    }
    let attr_arg = &attr_args[0];
    let meta_item = attr_arg.meta_item();
    if meta_item.is_none() {
        invalid_arg_err(attr);
        return None;
    }
    let meta_item = meta_item.unwrap();
    let ident = meta_item.ident().unwrap();
    let ident_str = ident.as_str();
    match &meta_item.kind {
        MetaItemKind::Word => {
            let solver = CbmcSolver::from_str(ident_str);
            match solver {
                Ok(solver) => Some(solver),
                Err(_) => {
                    tcx.sess.span_err(attr.span, format!("unknown solver `{ident_str}`"));
                    None
                }
            }
        }
        MetaItemKind::NameValue(lit) if ident_str == "bin" && lit.kind.is_str() => {
            Some(CbmcSolver::Binary(lit.symbol.to_string()))
        }
        _ => {
            invalid_arg_err(attr);
            None
        }
    }
}

/// Extracts the integer value argument from the attribute provided
/// For example, `unwind(8)` return `Some(8)`
fn parse_integer(attr: &Attribute) -> Option<u128> {
    // Vector of meta items , that contain the arguments given the attribute
    let attr_args = attr.meta_item_list()?;
    // Only extracts one integer value as argument
    if attr_args.len() == 1 {
        let x = attr_args[0].lit()?;
        match x.kind {
            LitKind::Int(y, ..) => Some(y),
            _ => None,
        }
    }
    // Return none if there are no attributes or if there's too many attributes
    else {
        None
    }
}

/// Extracts a vector with the path arguments of an attribute.
/// Emits an error if it couldn't convert any of the arguments.
fn parse_paths(attr: &Attribute) -> Result<Vec<String>, Span> {
    let attr_args = attr.meta_item_list();
    attr_args
        .unwrap_or_default()
        .iter()
        .map(|arg| match arg {
            NestedMetaItem::Lit(item) => Err(item.span),
            NestedMetaItem::MetaItem(item) => parse_path(item).ok_or(item.span),
        })
        .collect()
}

/// Extracts a path from an attribute item, returning `None` if the item is not
/// syntactically a path.
fn parse_path(meta_item: &MetaItem) -> Option<String> {
    if meta_item.is_word() {
        Some(
            meta_item
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.as_str())
                .collect::<Vec<&str>>()
                .join("::"),
        )
    } else {
        None
    }
}

/// Parse the arguments of the attribute into a (key, value) map.
fn parse_key_values(attr: &Attribute) -> Result<BTreeMap<String, String>, String> {
    trace!(list=?attr.meta_item_list(), ?attr, "parse_key_values");
    let args = attr.meta_item_list().ok_or("malformed attribute input")?;
    args.iter()
        .map(|arg| match arg.meta_item() {
            Some(MetaItem { path: key, kind: MetaItemKind::NameValue(val), .. }) => {
                Ok((key.segments.first().unwrap().ident.to_string(), val.symbol.to_string()))
            }
            _ => Err(format!(
                r#"expected "key = value" pair, but found `{}`"#,
                rustc_ast_pretty::pprust::meta_list_item_to_string(arg)
            )),
        })
        .collect()
}

/// Extracts the string value argument from the attribute provided.
///
/// For attributes with the following format, this will return a string that represents "VALUE".
/// - `#[attribute = "VALUE"]`
fn parse_str_value(attr: &Attribute) -> Option<String> {
    // Vector of meta items , that contain the arguments given the attribute
    let value = attr.value_str();
    value.map(|sym| sym.to_string())
}

/// If the attribute is named `kanitool::name`, this extracts `name`
fn attr_kind(tcx: TyCtxt, attr: &Attribute) -> Option<KaniAttributeKind> {
    match &attr.kind {
        AttrKind::Normal(normal) => {
            let segments = &normal.item.path.segments;
            if (!segments.is_empty()) && segments[0].ident.as_str() == "kanitool" {
                let ident_str = segments[1..]
                    .iter()
                    .map(|segment| segment.ident.as_str())
                    .intersperse("::")
                    .collect::<String>();
                KaniAttributeKind::try_from(ident_str.as_str())
                    .map_err(|err| {
                        debug!(?err, "attr_kind_failed");
                        tcx.sess.span_err(attr.span, format!("unknown attribute `{ident_str}`"));
                        err
                    })
                    .ok()
            } else {
                None
            }
        }
        _ => None,
    }
}
