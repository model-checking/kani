// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functionality that makes RMC easier to debug

use crate::GotocCtx;
use rustc_middle::mir::Body;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::Instance;
use std::cell::RefCell;
use std::lazy::SyncLazy;
use std::panic;
use tracing::debug;

// Use a thread-local global variable to track the current codegen item for debugging.
// If RMC panics during codegen, we can grab this item to include the problematic
// codegen item in the panic trace.
thread_local!(static CURRENT_CODEGEN_ITEM: RefCell<Option<String>> = RefCell::new(None));

// Include RMC's bug reporting URL in our panics.
const BUG_REPORT_URL: &str =
    "https://github.com/model-checking/rmc/issues/new?labels=bug&template=bug_report.md";

pub fn init() {
    // Install panic hook
    SyncLazy::force(&DEFAULT_HOOK); // Install ice hook
}

// Custom panic hook.
static DEFAULT_HOOK: SyncLazy<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>> =
    SyncLazy::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Invoke the default handler, which prints the actual panic message and
            // optionally a backtrace. This also prints Rustc's "file a bug here" message:
            // it seems like the only way to remove that is to use rustc_driver::report_ice;
            // however, adding that dependency to this crate causes a circular dependency.
            // For now, just print our message after the Rust one and explicitly point to
            // our bug template form.
            (*DEFAULT_HOOK)(info);

            // Separate the output with an empty line
            eprintln!();

            // Print the current function if available
            CURRENT_CODEGEN_ITEM.with(|cell| {
                if let Some(current_item) = cell.borrow().clone() {
                    eprintln!("[RMC] current codegen item: {}", current_item);
                } else {
                    eprintln!("[RMC] no current codegen item.");
                }
            });

            // Separate the output with an empty line
            eprintln!();

            // Print the RMC message
            eprintln!("RMC unexpectedly panicked during code generation.\n");
            eprintln!(
                "If you are seeing this message, please file an issue here instead of on the Rust compiler: {}",
                BUG_REPORT_URL
            );
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
    ) {
        CURRENT_CODEGEN_ITEM.with(|odb_cell| {
            odb_cell.replace(Some(panic_debug));
            call(self);
            odb_cell.replace(None);
        });
    }

    pub fn print_instance(&self, instance: Instance<'_>, mir: &'tcx Body<'tcx>) {
        if cfg!(debug_assertions) {
            debug!(
                "handling {}, {}",
                instance,
                with_no_trimmed_paths(|| self.tcx.def_path_str(instance.def_id()))
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
