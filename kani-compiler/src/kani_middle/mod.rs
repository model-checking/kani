// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code that are backend agnostic. For example, MIR analysis
//! and transformations.

use std::collections::HashSet;
use std::path::Path;

use crate::kani_middle::transform::BodyTransformation;
use crate::kani_queries::QueryDb;
use rustc_hir::{def::DefKind, def_id::LOCAL_CRATE};
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    FnAbiError, FnAbiOf, FnAbiOfHelpers, FnAbiRequest, HasParamEnv, HasTyCtxt, LayoutError,
    LayoutOfHelpers, TyAndLayout,
};
use rustc_middle::ty::{self, Instance as InstanceInternal, Ty as TyInternal, TyCtxt};
use rustc_session::config::OutputType;
use rustc_smir::rustc_internal;
use rustc_span::source_map::respan;
use rustc_span::Span;
use rustc_target::abi::call::FnAbi;
use rustc_target::abi::{HasDataLayout, TargetDataLayout};
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::ty::{FnDef, RigidTy, Span as SpanStable, TyKind};
use stable_mir::CrateDef;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use self::attributes::KaniAttributes;

pub mod analysis;
pub mod attributes;
pub mod codegen_units;
pub mod coercion;
mod intrinsics;
pub mod metadata;
pub mod provide;
pub mod reachability;
pub mod resolve;
pub mod stubbing;
pub mod transform;

/// Check that all crate items are supported and there's no misconfiguration.
/// This method will exhaustively print any error / warning and it will abort at the end if any
/// error was found.
pub fn check_crate_items(tcx: TyCtxt, ignore_asm: bool) {
    let krate = tcx.crate_name(LOCAL_CRATE);
    for item in tcx.hir().items() {
        let def_id = item.owner_id.def_id.to_def_id();
        KaniAttributes::for_item(tcx, def_id).check_attributes();
        if tcx.def_kind(def_id) == DefKind::GlobalAsm {
            if !ignore_asm {
                let error_msg = format!(
                    "Crate {krate} contains global ASM, which is not supported by Kani. Rerun with \
                    `--enable-unstable --ignore-global-asm` to suppress this error \
                    (**Verification results may be impacted**).",
                );
                tcx.dcx().err(error_msg);
            } else {
                tcx.dcx().warn(format!(
                    "Ignoring global ASM in crate {krate}. Verification results may be impacted.",
                ));
            }
        }
    }
    tcx.dcx().abort_if_errors();
}

/// Check that all given items are supported and there's no misconfiguration.
/// This method will exhaustively print any error / warning and it will abort at the end if any
/// error was found.
pub fn check_reachable_items(tcx: TyCtxt, queries: &QueryDb, items: &[MonoItem]) {
    // Avoid printing the same error multiple times for different instantiations of the same item.
    let mut def_ids = HashSet::new();
    for item in items.iter().filter(|i| matches!(i, MonoItem::Fn(..) | MonoItem::Static(..))) {
        let def_id = match item {
            MonoItem::Fn(instance) => instance.def.def_id(),
            MonoItem::Static(def) => def.def_id(),
            MonoItem::GlobalAsm(_) => {
                unreachable!()
            }
        };
        if !def_ids.contains(&def_id) {
            // Check if any unstable attribute was reached.
            KaniAttributes::for_def_id(tcx, def_id)
                .check_unstable_features(&queries.args().unstable_features);
            def_ids.insert(def_id);
        }
    }
    tcx.dcx().abort_if_errors();
}

/// Print MIR for the reachable items if the `--emit mir` option was provided to rustc.
pub fn dump_mir_items(
    tcx: TyCtxt,
    transformer: &mut BodyTransformation,
    items: &[MonoItem],
    output: &Path,
) {
    /// Convert MonoItem into a DefId.
    /// Skip stuff that we cannot generate the MIR items.
    fn get_instance(item: &MonoItem) -> Option<Instance> {
        match item {
            // Exclude FnShims and others that cannot be dumped.
            MonoItem::Fn(instance) => Some(*instance),
            MonoItem::Static(def) => {
                let instance: Instance = (*def).into();
                instance.has_body().then_some(instance)
            }
            MonoItem::GlobalAsm(_) => None,
        }
    }

    if tcx.sess.opts.output_types.contains_key(&OutputType::Mir) {
        // Create output buffer.
        let out_file = File::create(output).unwrap();
        let mut writer = BufWriter::new(out_file);

        // For each def_id, dump their MIR
        for instance in items.iter().filter_map(get_instance) {
            writeln!(writer, "// Item: {} ({})", instance.name(), instance.mangled_name()).unwrap();
            let body = transformer.body(tcx, instance);
            let _ = body.dump(&mut writer, &instance.name());
        }
    }
}

/// Structure that represents the source location of a definition.
/// TODO: Use `InternedString` once we move it out of the cprover_bindings.
/// <https://github.com/model-checking/kani/issues/2435>
pub struct SourceLocation {
    pub filename: String,
    pub start_line: usize,
    #[allow(dead_code)]
    pub start_col: usize, // set, but not currently used in Goto output
    pub end_line: usize,
    #[allow(dead_code)]
    pub end_col: usize, // set, but not currently used in Goto output
}

impl SourceLocation {
    pub fn new(span: SpanStable) -> Self {
        let loc = span.get_lines();
        let filename = span.get_filename().to_string();
        let start_line = loc.start_line;
        let start_col = loc.start_col;
        let end_line = loc.end_line;
        let end_col = loc.end_col;
        SourceLocation { filename, start_line, start_col, end_line, end_col }
    }
}

/// Get the FnAbi of a given instance with no extra variadic arguments.
/// TODO: Get rid of this. Use instance.fn_abi() instead.
/// <https://github.com/model-checking/kani/issues/1365>
pub fn fn_abi<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: InstanceInternal<'tcx>,
) -> &'tcx FnAbi<'tcx, TyInternal<'tcx>> {
    let helper = CompilerHelpers { tcx };
    helper.fn_abi_of_instance(instance, ty::List::empty())
}

struct CompilerHelpers<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> HasParamEnv<'tcx> for CompilerHelpers<'tcx> {
    fn param_env(&self) -> ty::ParamEnv<'tcx> {
        ty::ParamEnv::reveal_all()
    }
}

impl<'tcx> HasTyCtxt<'tcx> for CompilerHelpers<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
}

impl<'tcx> HasDataLayout for CompilerHelpers<'tcx> {
    fn data_layout(&self) -> &TargetDataLayout {
        self.tcx.data_layout()
    }
}

impl<'tcx> LayoutOfHelpers<'tcx> for CompilerHelpers<'tcx> {
    type LayoutOfResult = TyAndLayout<'tcx>;

    #[inline]
    fn handle_layout_err(&self, err: LayoutError<'tcx>, span: Span, ty: TyInternal<'tcx>) -> ! {
        span_bug!(span, "failed to get layout for `{}`: {}", ty, err)
    }
}

/// Implement error handling for extracting function ABI information.
impl<'tcx> FnAbiOfHelpers<'tcx> for CompilerHelpers<'tcx> {
    type FnAbiOfResult = &'tcx FnAbi<'tcx, TyInternal<'tcx>>;

    #[inline]
    fn handle_fn_abi_err(
        &self,
        err: FnAbiError<'tcx>,
        span: Span,
        fn_abi_request: FnAbiRequest<'tcx>,
    ) -> ! {
        if let FnAbiError::Layout(LayoutError::SizeOverflow(_)) = err {
            self.tcx.dcx().emit_fatal(respan(span, err))
        } else {
            match fn_abi_request {
                FnAbiRequest::OfFnPtr { sig, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err:?}\n while running `fn_abi_of_fn_ptr. ({sig}, {extra_args:?})`",
                    );
                }
                FnAbiRequest::OfInstance { instance, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err:?}\n while running `fn_abi_of_instance. ({instance}, {extra_args:?})`",
                    );
                }
            }
        }
    }
}

/// Find an instance of a function from the given crate that has been annotated with `diagnostic`
/// item.
fn find_fn_def(tcx: TyCtxt, diagnostic: &str) -> Option<FnDef> {
    let attr_id = tcx
        .all_diagnostic_items(())
        .name_to_id
        .get(&rustc_span::symbol::Symbol::intern(diagnostic))?;
    let TyKind::RigidTy(RigidTy::FnDef(def, _)) =
        rustc_internal::stable(tcx.type_of(attr_id)).value.kind()
    else {
        return None;
    };
    Some(def)
}
