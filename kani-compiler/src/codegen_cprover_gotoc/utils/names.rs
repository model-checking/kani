// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions that make names for things

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::InternedString;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_middle::mir::mono::CodegenUnitNameBuilder;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::Local;

impl<'tcx> GotocCtx<'tcx> {
    /// The full crate name including versioning info
    pub fn full_crate_name(&self) -> &str {
        &self.full_crate_name
    }

    pub fn codegen_var_base_name(&self, l: &Local) -> String {
        match self.current_fn().local_name(*l) {
            None => format!("var_{l}"),
            Some(name) => name.to_string(),
        }
    }

    pub fn codegen_var_name(&self, l: &Local) -> String {
        let fname = self.current_fn().name();
        match self.current_fn().local_name(*l) {
            Some(name) => format!("{fname}::1::var_{l}::{name}"),
            None => format!("{fname}::1::var_{l}"),
        }
    }

    pub fn is_user_variable(&self, var: &Local) -> bool {
        self.current_fn().local_name(*var).is_some()
    }

    // Special naming conventions for parameters that are spread from a tuple
    // into its individual components at the LLVM level, see comment at
    // compiler/rustc_codegen_llvm/src/gotoc/mod.rs:codegen_function_prelude
    pub fn codegen_spread_arg_name(&self, l: &Local) -> (String, String) {
        let fname = self.current_fn().name();
        let base_name = format!("spread{l:?}");
        let name = format!("{fname}::1::{base_name}");
        (name, base_name)
    }

    pub fn initializer_fn_name(var_name: &str) -> String {
        format!("{var_name}_init")
    }

    /// Return the mangled name to be used in the symbol table.
    #[inline(always)]
    pub fn symbol_name_stable(&self, instance: Instance) -> String {
        instance.mangled_name()
    }

    /// The name for a tuple field
    pub fn tuple_fld_name(n: usize) -> String {
        format!("{n}")
    }

    /// The name for the struct field on a vtable for a given function. Because generic
    /// functions can share the same name, we need to use the index of the entry in the
    /// vtable. This is the same index that will be passed in virtual function calls as
    /// InstanceDef::Virtual(def_id, idx).
    pub fn vtable_field_name(&self, idx: usize) -> InternedString {
        idx.to_string().into()
    }

    /// Add a prefix of the form:
    /// \[`<prefix>`\]
    /// to the provided message
    pub fn add_prefix_to_msg(msg: &str, prefix: &str) -> String {
        format!("[{prefix}] {msg}")
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
