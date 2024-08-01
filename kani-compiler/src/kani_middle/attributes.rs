// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for processing Rust attributes (like `kani::proof`).

use std::collections::BTreeMap;

use kani_metadata::{CbmcSolver, HarnessAttributes, HarnessKind, Stub};
use rustc_ast::{
    attr, AttrArgs, AttrArgsEq, AttrKind, Attribute, ExprKind, LitKind, MetaItem, MetaItemKind,
    NestedMetaItem,
};
use rustc_errors::ErrorGuaranteed;
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::ty::{Instance, TyCtxt, TyKind};
use rustc_session::Session;
use rustc_smir::rustc_internal;
use rustc_span::{Span, Symbol};
use stable_mir::mir::mono::Instance as InstanceStable;
use stable_mir::{CrateDef, DefId as StableDefId};
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
    /// Attribute on a function with a contract that identifies the code
    /// implementing the recursive check for the harness.
    RecursionCheck,
    /// Attribute on a function that was auto-generated from expanding a
    /// function contract.
    IsContractGenerated,
    /// A function with contract expanded to include the write set as arguments.
    ///
    /// Contains the original body of the contracted function. The signature is
    /// expanded with additional pointer arguments that are not used in the function
    /// but referenced by the `modifies` annotation.
    ModifiesWrapper,
    /// Attribute used to mark contracts for functions with recursion.
    /// We use this attribute to properly instantiate `kani::any_modifies` in
    /// cases when recursion is present given our contracts instrumentation.
    Recursion,
    /// Attribute used to mark the static variable used for tracking recursion check.
    RecursionTracker,
    /// Generic marker that can be used to mark functions so this list doesn't have to keep growing.
    /// This takes a key which is the marker.
    FnMarker,
    /// Used to mark functions where generating automatic pointer checks should be disabled. This is
    /// used later to automatically attach pragma statements to locations.
    DisableChecks,
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
            | KaniAttributeKind::FnMarker
            | KaniAttributeKind::Recursion
            | KaniAttributeKind::RecursionTracker
            | KaniAttributeKind::ReplacedWith
            | KaniAttributeKind::RecursionCheck
            | KaniAttributeKind::CheckedWith
            | KaniAttributeKind::ModifiesWrapper
            | KaniAttributeKind::IsContractGenerated
            | KaniAttributeKind::DisableChecks => false,
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

#[derive(Clone, Debug)]
/// Bundle contract attributes for a function annotated with contracts.
pub struct ContractAttributes {
    /// Whether the contract was marked with #[recursion] attribute.
    pub has_recursion: bool,
    /// The name of the contract recursion check.
    pub recursion_check: Symbol,
    /// The name of the contract check.
    pub checked_with: Symbol,
    /// The name of the contract replacement.
    pub replaced_with: Symbol,
    /// The name of the inner check used to modify clauses.
    pub modifies_wrapper: Symbol,
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
        KaniAttributes::for_def_id(tcx, instance.def.def_id())
    }

    /// Look up the attributes by a stable MIR DefID
    pub fn for_def_id(tcx: TyCtxt<'tcx>, def_id: StableDefId) -> Self {
        KaniAttributes::for_item(tcx, rustc_internal::internal(tcx, def_id))
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
                self.tcx.dcx().err(format!(
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
    ///
    /// Any error is emitted and the attribute is filtered out.
    pub fn interpret_stub_verified_attribute(&self) -> Vec<(Symbol, DefId, Span)> {
        self.map
            .get(&KaniAttributeKind::StubVerified)
            .map_or([].as_slice(), Vec::as_slice)
            .iter()
            .filter_map(|attr| {
                let name = expect_key_string_value(self.tcx.sess, attr).ok()?;
                let def = self
                    .resolve_from_mod(name.as_str())
                    .map_err(|e| {
                        self.tcx.dcx().span_err(
                            attr.span,
                            format!(
                                "Failed to resolve replacement function {}: {e}",
                                name.as_str()
                            ),
                        )
                    })
                    .ok()?;
                Some((name, def, attr.span))
            })
            .collect()
    }

    pub(crate) fn has_recursion(&self) -> bool {
        self.map.contains_key(&KaniAttributeKind::Recursion)
    }

    /// Parse and extract the `proof_for_contract(TARGET)` attribute. The
    /// returned symbol and DefId are respectively the name and id of `TARGET`,
    /// the span in the span for the attribute (contents).
    ///
    /// In the case of an error, this function will emit the error and return `None`.
    pub(crate) fn interpret_for_contract_attribute(&self) -> Option<(Symbol, DefId, Span)> {
        self.expect_maybe_one(KaniAttributeKind::ProofForContract).and_then(|target| {
            let name = expect_key_string_value(self.tcx.sess, target).ok()?;
            self.resolve_from_mod(name.as_str())
                .map(|ok| (name, ok, target.span))
                .map_err(|resolve_err| {
                    self.tcx.dcx().span_err(
                        target.span,
                        format!(
                            "Failed to resolve checking function {} because {resolve_err}",
                            name.as_str()
                        ),
                    )
                })
                .ok()
        })
    }

    pub fn proof_for_contract(&self) -> Option<Result<Symbol, ErrorGuaranteed>> {
        self.expect_maybe_one(KaniAttributeKind::ProofForContract)
            .map(|target| expect_key_string_value(self.tcx.sess, target))
    }

    /// Extract the name of the local that represents this function's contract is
    /// checked with (if any).
    ///
    /// `None` indicates this function does not use a contract, or an error was found.
    /// Note that the error will already be emitted, so we don't return an error.
    pub fn contract_attributes(&self) -> Option<ContractAttributes> {
        let has_recursion = self.has_recursion();
        let recursion_check = self.attribute_value(KaniAttributeKind::RecursionCheck);
        let checked_with = self.attribute_value(KaniAttributeKind::CheckedWith);
        let replace_with = self.attribute_value(KaniAttributeKind::ReplacedWith);
        let modifies_wrapper = self.attribute_value(KaniAttributeKind::ModifiesWrapper);

        let total = recursion_check
            .iter()
            .chain(&checked_with)
            .chain(&replace_with)
            .chain(&modifies_wrapper)
            .count();
        if total != 0 && total != 4 {
            self.tcx.sess.dcx().err(format!(
                "Failed to parse contract instrumentation tags in function `{}`.\
                Expected `4` attributes, but was only able to process `{total}`",
                self.tcx.def_path_str(self.item)
            ));
        }
        Some(ContractAttributes {
            has_recursion,
            recursion_check: recursion_check?,
            checked_with: checked_with?,
            replaced_with: replace_with?,
            modifies_wrapper: modifies_wrapper?,
        })
    }

    /// Return a function marker if any.
    pub fn fn_marker(&self) -> Option<Symbol> {
        self.attribute_value(KaniAttributeKind::FnMarker)
    }

    /// Check if function is annotated with any contract attribute.
    pub fn has_contract(&self) -> bool {
        self.map.contains_key(&KaniAttributeKind::CheckedWith)
    }

    /// Resolve a path starting from this item's module context.
    fn resolve_from_mod(&self, path_str: &str) -> Result<DefId, ResolveError<'tcx>> {
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
        let is_harness = self.is_proof_harness();
        for (&kind, attrs) in self.map.iter() {
            let local_error = |msg| self.tcx.dcx().span_err(attrs[0].span, msg);

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
                KaniAttributeKind::Recursion => {
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
                KaniAttributeKind::FnMarker
                | KaniAttributeKind::CheckedWith
                | KaniAttributeKind::ModifiesWrapper
                | KaniAttributeKind::RecursionCheck
                | KaniAttributeKind::ReplacedWith => {
                    self.attribute_value(kind);
                }
                KaniAttributeKind::IsContractGenerated => {
                    // Ignored here because this is only used by the proc macros
                    // to communicate with one another. So by the time it gets
                    // here we don't care if it's valid or not.
                }
                KaniAttributeKind::RecursionTracker => {
                    // Nothing to do here. This is used by contract instrumentation.
                }
                KaniAttributeKind::DisableChecks => {
                    // Ignored here, because it should be an internal attribute. Actual validation
                    // happens when pragmas are generated.
                }
            }
        }
    }

    /// Get the value of an attribute if one exists.
    ///
    /// This expects up to one attribute with format `#[kanitool::<name>("<value>")]`.
    ///
    /// Any format or expectation error is emitted already, and does not need to be handled
    /// upstream.
    fn attribute_value(&self, kind: KaniAttributeKind) -> Option<Symbol> {
        self.expect_maybe_one(kind)
            .and_then(|target| expect_key_string_value(self.tcx.sess, target).ok())
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
                    self.tcx.dcx().span_err(attr.span, msg);
                } else {
                    self.tcx.dcx().err(msg);
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
            .dcx()
            .struct_err(format!(
                "Use of unstable feature `{}`: {}",
                unstable_attr.feature, unstable_attr.reason
            ))
            .with_span_note(
                self.tcx.def_span(self.item),
                format!("the function `{fn_name}` is unstable:"),
            )
            .with_note(format!("see issue {} for more information", unstable_attr.issue))
            .with_help(format!("use `-Z {}` to enable using this function.", unstable_attr.feature))
            .emit()
    }

    /// Is this item a harness? (either `proof` or `proof_for_contract`
    /// attribute are present)
    fn is_proof_harness(&self) -> bool {
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
        assert!(self.is_proof_harness());
        let harness_attrs = if let Some(Ok(harness)) = self.proof_for_contract() {
            HarnessAttributes::new(HarnessKind::ProofForContract { target_fn: harness.to_string() })
        } else {
            HarnessAttributes::new(HarnessKind::Proof)
        };
        self.map.iter().fold(harness_attrs, |mut harness, (kind, attributes)| {
            match kind {
                KaniAttributeKind::ShouldPanic => harness.should_panic = true,
                KaniAttributeKind::Recursion => {
                    self.tcx.dcx().span_err(self.tcx.def_span(self.item), "The attribute `kani::recursion` should only be used in combination with function contracts.");
                }
                KaniAttributeKind::Solver => {
                    harness.solver = parse_solver(self.tcx, attributes[0]);
                }
                KaniAttributeKind::Stub => {
                    harness.stubs.extend_from_slice(&parse_stubs(self.tcx, self.item, attributes));
                }
                KaniAttributeKind::Unwind => {
                    harness.unwind_value = parse_unwind(self.tcx, attributes[0])
                }
                KaniAttributeKind::Proof => { /* no-op */ }
                KaniAttributeKind::ProofForContract => self.handle_proof_for_contract(&mut harness),
                KaniAttributeKind::StubVerified => self.handle_stub_verified(&mut harness),
                KaniAttributeKind::Unstable => {
                    // Internal attribute which shouldn't exist here.
                    unreachable!()
                }
                KaniAttributeKind::CheckedWith
                | KaniAttributeKind::IsContractGenerated
                | KaniAttributeKind::ModifiesWrapper
                | KaniAttributeKind::RecursionCheck
                | KaniAttributeKind::RecursionTracker
                | KaniAttributeKind::ReplacedWith => {
                    self.tcx.dcx().span_err(self.tcx.def_span(self.item), format!("Contracts are not supported on harnesses. (Found the kani-internal contract attribute `{}`)", kind.as_ref()));
                }
                KaniAttributeKind::DisableChecks => {
                    // Internal attribute which shouldn't exist here.
                    unreachable!()
                }
                KaniAttributeKind::FnMarker => {
                    /* no-op */
                }
            };
            harness
        })
    }

    fn handle_proof_for_contract(&self, harness: &mut HarnessAttributes) {
        let dcx = self.tcx.dcx();
        let (name, id, span) = match self.interpret_for_contract_attribute() {
            None => return, // This error was already emitted
            Some(values) => values,
        };
        assert!(matches!(
                &harness.kind, HarnessKind::ProofForContract { target_fn }
                if *target_fn == name.to_string()));
        if KaniAttributes::for_item(self.tcx, id).contract_attributes().is_none() {
            dcx.struct_span_err(
                span,
                format!(
                    "Failed to check contract: Function `{}` has no contract.",
                    self.item_name(),
                ),
            )
            .with_span_note(self.tcx.def_span(id), "Try adding a contract to this function.")
            .emit();
        }
    }

    fn handle_stub_verified(&self, harness: &mut HarnessAttributes) {
        let dcx = self.tcx.dcx();
        for (name, def_id, span) in self.interpret_stub_verified_attribute() {
            if KaniAttributes::for_item(self.tcx, def_id).contract_attributes().is_none() {
                dcx.struct_span_err(
                        span,
                        format!(
                            "Failed to generate verified stub: Function `{}` has no contract.",
                            self.item_name(),
                        ),
                    )
                        .with_span_note(
                            self.tcx.def_span(def_id),
                            format!(
                                "Try adding a contract to this function or use the unsound `{}` attribute instead.",
                                KaniAttributeKind::Stub.as_ref(),
                            ),
                        )
                    .emit();
                return;
            }
            harness.verified_stubs.push(name.to_string())
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
            tcx.dcx().span_err(span, "the `proof` attribute can only be applied to functions");
        } else if tcx.generics_of(self.item).requires_monomorphization(tcx) {
            tcx.dcx()
                .span_err(span, "the `proof` attribute cannot be applied to generic functions");
        } else {
            let instance = Instance::mono(tcx, self.item);
            if !super::fn_abi(tcx, instance).args.is_empty() {
                tcx.dcx().span_err(span, "functions used as harnesses cannot have any arguments");
            }
        }
    }
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

/// Same as [`KaniAttributes::is_proof_harness`] but more efficient because less
/// attribute parsing is performed.
pub fn is_proof_harness(tcx: TyCtxt, instance: InstanceStable) -> bool {
    let def_id = rustc_internal::internal(tcx, instance.def.def_id());
    has_kani_attribute(tcx, def_id, |a| {
        matches!(a, KaniAttributeKind::Proof | KaniAttributeKind::ProofForContract)
    })
}

/// Does this `def_id` have `#[rustc_test_marker]`?
pub fn is_test_harness_description(tcx: TyCtxt, item: impl CrateDef) -> bool {
    let def_id = rustc_internal::internal(tcx, item.def_id());
    let attrs = tcx.get_attrs_unchecked(def_id);
    attr::contains_name(attrs, rustc_span::symbol::sym::rustc_test_marker)
}

/// Extract the test harness name from the `#[rustc_test_maker]`
pub fn test_harness_name(tcx: TyCtxt, def: &impl CrateDef) -> String {
    let def_id = rustc_internal::internal(tcx, def.def_id());
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
        return Err(sess
            .dcx()
            .span_err(span, "Expected attribute of the form #[attr = \"value\"]"));
    };
    let maybe_str = match it {
        AttrArgsEq::Ast(expr) => {
            if let ExprKind::Lit(tok) = expr.kind {
                match LitKind::from_token_lit(tok) {
                    Ok(l) => l.str(),
                    Err(err) => {
                        return Err(sess.dcx().span_err(
                            span,
                            format!("Invalid string literal on right hand side of `=` {err:?}"),
                        ));
                    }
                }
            } else {
                return Err(sess
                    .dcx()
                    .span_err(span, "Expected literal string as right hand side of `=`"));
            }
        }
        AttrArgsEq::Hir(lit) => lit.kind.str(),
    };
    if let Some(str) = maybe_str {
        Ok(str)
    } else {
        Err(sess.dcx().span_err(span, "Expected literal string as right hand side of `=`"))
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
        tcx.dcx().span_err(
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
        tcx.dcx()
            .struct_span_err(
                self.attr.span,
                format!("failed to parse `#[kani::unstable_feature]`: {}", self.reason),
            )
            .with_note(format!(
                "expected format: #[kani::unstable_feature({}, {}, {})]",
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
        tcx.dcx()
            .struct_span_err(attr.span, format!("unexpected argument for `{}`", kind.as_ref()))
            .with_help("remove the extra argument")
            .emit();
    }
}

/// Return the unwind value from the given attribute.
fn parse_unwind(tcx: TyCtxt, attr: &Attribute) -> Option<u32> {
    // Get Attribute value and if it's not none, assign it to the metadata
    match parse_integer(attr) {
        None => {
            // There are no integers or too many arguments given to the attribute
            tcx.dcx().span_err(
                attr.span,
                "invalid argument for `unwind` attribute, expected an integer",
            );
            None
        }
        Some(unwind_integer_value) => {
            if let Ok(val) = unwind_integer_value.try_into() {
                Some(val)
            } else {
                tcx.dcx().span_err(attr.span, "value above maximum permitted value - u32::MAX");
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
            tcx.dcx().span_err(attr.span, format!("failed to resolve `{name}`: {err}"));
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
                    tcx.dcx().span_err(
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
                tcx.dcx().span_err(
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
        tcx.dcx().span_err(
            attr.span,
            format!("invalid argument for `{ATTRIBUTE}` attribute, expected one of the supported solvers (e.g. `kissat`) or a SAT solver binary (e.g. `bin=\"<SAT_SOLVER_BINARY>\"`)"),
        )
    };

    let attr_args = attr.meta_item_list().unwrap();
    if attr_args.len() != 1 {
        tcx.dcx().span_err(
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
                    tcx.dcx().span_err(attr.span, format!("unknown solver `{ident_str}`"));
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
            LitKind::Int(y, ..) => Some(y.get()),
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
                    .inspect_err(|&err| {
                        debug!(?err, "attr_kind_failed");
                        tcx.dcx().span_err(attr.span, format!("unknown attribute `{ident_str}`"));
                    })
                    .ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn matches_diagnostic<T: CrateDef>(tcx: TyCtxt, def: T, attr_name: &str) -> bool {
    let attr_sym = rustc_span::symbol::Symbol::intern(attr_name);
    if let Some(attr_id) = tcx.all_diagnostic_items(()).name_to_id.get(&attr_sym) {
        if rustc_internal::internal(tcx, def.def_id()) == *attr_id {
            debug!("matched: {:?} {:?}", attr_id, attr_sym);
            return true;
        }
    }
    false
}
