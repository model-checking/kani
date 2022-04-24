// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functionality that makes Kani easier to debug

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Location;
use rustc_middle::mir::Body;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::Instance;
use rustc_span::def_id::DefId;
use std::cell::RefCell;
use std::lazy::SyncLazy;
use std::panic;
use tracing::debug;

// Use a thread-local global variable to track the current codegen item for debugging.
// If Kani panics during codegen, we can grab this item to include the problematic
// codegen item in the panic trace.
thread_local!(static CURRENT_CODEGEN_ITEM: RefCell<(Option<String>, Option<Location>)> = RefCell::new((None, None)));

pub fn init() {
    // Install panic hook
    SyncLazy::force(&DEFAULT_HOOK); // Install ice hook
}

// Custom panic hook to add more information when panic occurs during goto-c codegen.
static DEFAULT_HOOK: SyncLazy<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>> =
    SyncLazy::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Invoke the default handler, which prints the actual panic message and
            // optionally a backtrace.
            (*DEFAULT_HOOK)(info);
            eprintln!();

            // Print the current function if available
            CURRENT_CODEGEN_ITEM.with(|cell| {
                let t = cell.borrow().clone();
                if let Some(current_item) = t.0 {
                    eprintln!("[Kani] current codegen item: {}", current_item);
                } else {
                    eprintln!("[Kani] no current codegen item.");
                }
                if let Some(current_loc) = t.1 {
                    eprintln!("[Kani] current codegen location: {:?}", current_loc);
                } else {
                    eprintln!("[Kani] no current codegen location.");
                }
            });
        }));
        hook
    });

impl<'tcx> GotocCtx<'tcx> {
    // Calls the closure while updating the tracked global variable marking the
    // codegen item for panic debugging.
    pub fn call_with_panic_debug_info<F: FnOnce(&mut GotocCtx<'tcx>) -> ()>(
        &mut self,
        call: F,
        panic_debug: String,
        def_id: DefId,
    ) {
        CURRENT_CODEGEN_ITEM.with(|odb_cell| {
            odb_cell
                .replace((Some(panic_debug), Some(self.codegen_span(&self.tcx.def_span(def_id)))));
            call(self);
            odb_cell.replace((None, None));
        });
    }

    pub fn print_instance(&self, instance: Instance<'_>, mir: &'tcx Body<'tcx>) {
        if cfg!(debug_assertions) {
            debug!(
                "handling {}, {}",
                instance,
                with_no_trimmed_paths!(self.tcx.def_path_str(instance.def_id()))
            );
            debug!("variables: ");
            for l in mir.args_iter().chain(mir.vars_and_temps_iter()) {
                debug!("let {:?}: {:?}", l, self.local_ty(l));
            }
            for (bb, bbd) in mir.basic_blocks().iter_enumerated() {
                debug!("block {:?}", bb);
                for stmt in &bbd.statements {
                    debug!("{:?}", stmt);
                }
                debug!("{:?}", bbd.terminator().kind);
            }
        }
    }
}
