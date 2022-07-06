// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions that make names for things

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::InternedString;
use rustc_hir::def_id::DefId;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::definitions::DefPathDataName;
use rustc_middle::mir::mono::CodegenUnitNameBuilder;
use rustc_middle::mir::Local;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{Instance, TyCtxt};
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    /// The short crate name without versioning information.
    pub fn short_crate_name(&self) -> String {
        self.tcx.crate_name(LOCAL_CRATE).to_string()
    }

    /// The full crate name including versioning info
    pub fn full_crate_name(&self) -> &str {
        &self.full_crate_name
    }

    pub fn codegen_var_base_name(&self, l: &Local) -> String {
        match self.find_debug_info(l) {
            None => format!("var_{}", l.index()),
            Some(info) => format!("{}", info.name),
        }
    }

    pub fn codegen_var_name(&self, l: &Local) -> String {
        let fname = self.current_fn().name();
        match self.find_debug_info(l) {
            Some(info) => format!("{}::1::var{:?}::{}", fname, l, info.name),
            None => format!("{}::1::var{:?}", fname, l),
        }
    }

    // Special naming conventions for parameters that are spread from a tuple
    // into its individual components at the LLVM level, see comment at
    // compiler/rustc_codegen_llvm/src/gotoc/mod.rs:codegen_function_prelude
    pub fn codegen_spread_arg_name(&self, l: &Local) -> (String, String) {
        let fname = self.current_fn().name();
        let base_name = format!("spread{:?}", l);
        let name = format!("{}::1::{}", fname, base_name);
        (name, base_name)
    }

    pub fn initializer_fn_name(var_name: &str) -> String {
        format!("{}_init", var_name)
    }

    /// A human readable name in Rust for reference, should not be used as a key.
    pub fn readable_instance_name(&self, instance: Instance<'tcx>) -> String {
        with_no_trimmed_paths!(self.tcx.def_path_str(instance.def_id()))
    }

    /// The actual function name used in the symbol table
    pub fn symbol_name(&self, instance: Instance<'tcx>) -> String {
        let llvm_mangled = self.tcx.symbol_name(instance).name.to_string();
        debug!(
            "finding function name for instance: {}, debug: {:?}, name: {}, symbol: {}, demangle: {}",
            instance,
            instance,
            self.readable_instance_name(instance),
            llvm_mangled,
            rustc_demangle::demangle(&llvm_mangled).to_string()
        );

        let pretty = self.readable_instance_name(instance);

        // Make main function a special case for easy CBMC entry
        // TODO: probably need to edit for https://github.com/model-checking/kani/issues/169
        if pretty == "main" {
            "main".to_string()
        } else {
            // TODO: llvm mangled string is not very readable. one way to tackle this is to
            // demangle it. but the demangled string has no generic info.
            // the best scenario is to use v0 mangler, but this is not default at this moment.
            // this is the kind of tiny but annoying issue.
            // c.f. https://github.com/rust-lang/rust/issues/60705
            //
            // the following solution won't work pretty:
            // match self.tcx.sess.opts.debugging_opts.symbol_mangling_version {
            //     SymbolManglingVersion::Legacy => llvm_mangled,
            //     SymbolManglingVersion::V0 => rustc_demangle::demangle(llvm_mangled.as_str()).to_string(),
            // }
            llvm_mangled
        }
    }

    /// The name for a tuple field
    pub fn tuple_fld_name(n: usize) -> String {
        format!("{}", n)
    }

    /// The name for the struct field on a vtable for a given function. Because generic
    /// functions can share the same name, we need to use the index of the entry in the
    /// vtable. This is the same index that will be passed in virtual function calls as
    /// InstanceDef::Virtual(def_id, idx). We could use solely the index as a key into
    /// the vtable struct, but we add the method name for debugging readability.
    ///     Example: 3_vol
    pub fn vtable_field_name(&self, _def_id: DefId, idx: usize) -> InternedString {
        // format!("{}_{}", idx, with_no_trimmed_paths!(|| self.tcx.item_name(def_id)))
        // TODO: use def_id https://github.com/model-checking/kani/issues/364
        idx.to_string().into()
    }

    /// Add a prefix of the form:
    /// \[<prefix>\]
    /// to the provided message
    pub fn add_prefix_to_msg(msg: &str, prefix: &str) -> String {
        format!("[{}] {}", prefix, msg)
    }

    /// Generate a message for the reachability check of an assert with ID
    /// `check_id`. The message is of the form:
    /// \[KANI_REACHABILITY_CHECK\] <ID of assert>
    /// The check_id is generated using the GotocCtx::next_check_id method and
    /// is a unique string identifier for that check.
    pub fn reachability_check_message(check_id: &str) -> String {
        format!("[KANI_REACHABILITY_CHECK] {}", check_id)
    }
}

/// The full crate name should use the Codegen Unit builder to include full name resolution,
/// for example, the versioning information if a build requires two different versions
/// of the same crate.
pub fn full_crate_name(tcx: TyCtxt) -> String {
    format!(
        "{}::{}",
        CodegenUnitNameBuilder::new(tcx).build_cgu_name(
            LOCAL_CRATE,
            &[] as &[String; 0],
            None as Option<String>
        ),
        tcx.crate_name(LOCAL_CRATE)
    )
}

//TODO: These were moved from hooks.rs, where they didn't have a goto context. Normalize them.

/// Helper function to determine if the function name starts with `expected`
// TODO: rationalize how we match the hooks https://github.com/model-checking/kani/issues/130
pub fn instance_name_starts_with(tcx: TyCtxt, instance: Instance, expected: &str) -> bool {
    let def_path = tcx.def_path(instance.def.def_id());
    if let Some(data) = def_path.data.last() {
        match data.data.name() {
            DefPathDataName::Named(name) => return name.to_string().starts_with(expected),
            DefPathDataName::Anon { .. } => (),
        }
    }
    false
}
