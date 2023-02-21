// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code that are backend agnostic. For example, MIR analysis
//! and transformations.

use rustc_hir::{def::DefKind, def_id::LOCAL_CRATE};
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    FnAbiError, FnAbiOfHelpers, FnAbiRequest, HasParamEnv, HasTyCtxt, LayoutError, LayoutOfHelpers,
    TyAndLayout,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::source_map::respan;
use rustc_span::Span;
use rustc_target::abi::call::FnAbi;
use rustc_target::abi::{HasDataLayout, TargetDataLayout};

use self::attributes::check_attributes;

pub mod attributes;
pub mod coercion;
pub mod provide;
pub mod reachability;
pub mod resolve;
pub mod stubbing;

/// Check that all crate items are supported and there's no misconfiguration.
/// This method will exhaustively print any error / warning and it will abort at the end if any
/// error was found.
pub fn check_crate_items(tcx: TyCtxt, ignore_asm: bool) {
    let krate = tcx.crate_name(LOCAL_CRATE);
    for item in tcx.hir_crate_items(()).items() {
        let def_id = item.owner_id.def_id.to_def_id();
        check_attributes(tcx, def_id);
        if tcx.def_kind(def_id) == DefKind::GlobalAsm {
            if !ignore_asm {
                let error_msg = format!(
                    "Crate {krate} contains global ASM, which is not supported by Kani. Rerun with \
                    `--enable-unstable --ignore-global-asm` to suppress this error \
                    (**Verification results may be impacted**).",
                );
                tcx.sess.err(&error_msg);
            } else {
                tcx.sess.warn(format!(
                    "Ignoring global ASM in crate {krate}. Verification results may be impacted.",
                ));
            }
        }
    }
    tcx.sess.abort_if_errors();
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
    fn handle_layout_err(&self, err: LayoutError<'tcx>, span: Span, ty: Ty<'tcx>) -> ! {
        span_bug!(span, "failed to get layout for `{}`: {}", ty, err)
    }
}

/// Implement error handling for extracting function ABI information.
impl<'tcx> FnAbiOfHelpers<'tcx> for CompilerHelpers<'tcx> {
    type FnAbiOfResult = &'tcx FnAbi<'tcx, Ty<'tcx>>;

    #[inline]
    fn handle_fn_abi_err(
        &self,
        err: FnAbiError<'tcx>,
        span: Span,
        fn_abi_request: FnAbiRequest<'tcx>,
    ) -> ! {
        if let FnAbiError::Layout(LayoutError::SizeOverflow(_)) = err {
            self.tcx.sess.emit_fatal(respan(span, err))
        } else {
            match fn_abi_request {
                FnAbiRequest::OfFnPtr { sig, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err}\n while running `fn_abi_of_fn_ptr. ({sig}, {extra_args:?})`",
                    );
                }
                FnAbiRequest::OfInstance { instance, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err}\n while running `fn_abi_of_instance. ({instance}, {extra_args:?})`",
                    );
                }
            }
        }
    }
}
