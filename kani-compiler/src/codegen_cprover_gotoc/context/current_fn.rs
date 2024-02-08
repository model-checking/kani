// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Stmt;
use cbmc::InternedString;
use rustc_middle::mir::Body as BodyInternal;
use rustc_middle::ty::Instance as InstanceInternal;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Body, Local, LocalDecl};
use stable_mir::CrateDef;
use std::collections::HashMap;

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
    mir: &'tcx BodyInternal<'tcx>,
    /// The current instance. This is using the internal representation.
    instance_internal: InstanceInternal<'tcx>,
    /// A list of local declarations used to retrieve MIR component types.
    locals: Vec<LocalDecl>,
    /// A list of pretty names for locals that corrspond to user variables.
    local_names: HashMap<Local, InternedString>,
    /// The symbol name of the current function
    name: String,
    /// A human readable pretty name for the current function
    readable_name: String,
    /// A counter to enable creating temporary variables
    temp_var_counter: u64,
}

/// Constructor
impl<'tcx> CurrentFnCtx<'tcx> {
    pub fn new(instance: Instance, gcx: &GotocCtx<'tcx>, body: &Body) -> Self {
        let instance_internal = rustc_internal::internal(gcx.tcx, instance);
        let readable_name = instance.name();
        let name =
            if &readable_name == "main" { readable_name.clone() } else { instance.mangled_name() };
        let locals = body.locals().to_vec();
        let local_names = body
            .var_debug_info
            .iter()
            .filter_map(|info| info.local().map(|local| (local, (&info.name).into())))
            .collect::<HashMap<_, _>>();
        Self {
            block: vec![],
            instance,
            mir: gcx.tcx.instance_mir(instance_internal.def),
            instance_internal,
            krate: instance.def.krate().name,
            locals,
            local_names,
            name,
            readable_name,
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
    pub fn instance(&self) -> InstanceInternal<'tcx> {
        self.instance_internal
    }

    pub fn instance_stable(&self) -> Instance {
        self.instance
    }

    /// The internal MIR for the function we are currently compiling using internal APIs.
    pub fn body_internal(&self) -> &'tcx BodyInternal<'tcx> {
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

    pub fn locals(&self) -> &[LocalDecl] {
        &self.locals
    }

    pub fn local_name(&self, local: Local) -> Option<InternedString> {
        self.local_names.get(&local).copied()
    }
}

/// Utility functions
impl CurrentFnCtx<'_> {
    /// Is the current function from the `std` crate?
    pub fn is_std(&self) -> bool {
        self.krate == "std" || self.krate == "core"
    }
}
