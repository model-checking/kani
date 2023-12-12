// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Stmt;
use rustc_middle::mir::{BasicBlock, Body as InternalBody};
use rustc_middle::ty::{Instance as InternalInstance, PolyFnSig};
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::Body;
use stable_mir::CrateDef;

/// This structure represents useful data about the function we are currently compiling.
#[derive(Debug)]
pub struct CurrentFnCtx<'tcx> {
    /// The GOTO block we are compiling into
    block: Vec<Stmt>,
    /// The codegen instance for the current function
    instance: Instance,
    /// The crate this function is from
    krate: String,
    /// The MIR for the current instance. This is using the internal representation.
    mir: &'tcx InternalBody<'tcx>,
    /// The MIR for the current instance.
    body: Body,
    /// The symbol name of the current function
    name: String,
    /// A human readable pretty name for the current function
    readable_name: String,
    /// The signature of the current function
    sig: PolyFnSig<'tcx>,
    /// A counter to enable creating temporary variables
    temp_var_counter: u64,
}

/// Constructor
impl<'tcx> CurrentFnCtx<'tcx> {
    pub fn new(instance: Instance, gcx: &GotocCtx<'tcx>) -> Self {
        let internal_instance = rustc_internal::internal(instance);
        let body = instance.body().unwrap();
        let readable_name = instance.name();
        let name =
            if &readable_name == "main" { readable_name.clone() } else { instance.mangled_name() };
        Self {
            block: vec![],
            instance,
            mir: gcx.tcx.instance_mir(internal_instance.def),
            krate: instance.def.krate().name,
            body,
            name,
            readable_name,
            sig: gcx.fn_sig_of_instance(internal_instance),
            temp_var_counter: 0,
        }
    }
}

/// Setters
impl<'tcx> CurrentFnCtx<'tcx> {
    /// Returns the current block, replacing it with an empty vector.
    pub fn extract_block(&mut self) -> Vec<Stmt> {
        std::mem::take(&mut self.block)
    }

    pub fn get_and_incr_counter(&mut self) -> u64 {
        let rval = self.temp_var_counter;
        self.temp_var_counter += 1;
        rval
    }

    pub fn push_onto_block(&mut self, s: Stmt) {
        self.block.push(s)
    }
}

/// Getters
impl<'tcx> CurrentFnCtx<'tcx> {
    /// The function we are currently compiling
    pub fn instance(&self) -> InternalInstance<'tcx> {
        rustc_internal::internal(self.instance)
    }

    /// The crate that function came from
    pub fn krate(&self) -> String {
        self.krate.to_string()
    }

    /// The internal MIR for the function we are currently compiling using internal APIs.
    pub fn body_internal(&self) -> &'tcx InternalBody<'tcx> {
        self.mir
    }

    /// The name of the function we are currently compiling
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The pretty name of the function we are currently compiling
    pub fn readable_name(&self) -> &str {
        &self.readable_name
    }

    /// The signature of the function we are currently compiling
    pub fn sig(&self) -> PolyFnSig<'tcx> {
        self.sig
    }

    /// The body of the function.
    pub fn body(&self) -> &Body {
        &self.body
    }
}

/// Utility functions
impl CurrentFnCtx<'_> {
    /// Is the current function from the `std` crate?
    pub fn is_std(&self) -> bool {
        self.krate == "std" || self.krate == "core"
    }

    pub fn find_label(&self, bb: &BasicBlock) -> String {
        format!("{bb:?}")
    }
}
