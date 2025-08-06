// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functionality that makes Kani easier to debug

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Location;
use rustc_public::CrateDef;
use rustc_public::mir::Body;
use rustc_public::mir::mono::Instance;
use std::cell::RefCell;
use std::panic;
use std::sync::LazyLock;
use tracing::debug;

// Use a thread-local global variable to track the current codegen item for debugging.
// If Kani panics during codegen, we can grab this item to include the problematic
// codegen item in the panic trace.
type FindCurrentItemFn = Box<dyn Fn() -> String>;
thread_local!(static CURRENT_CODEGEN_ITEM: RefCell<(Option<FindCurrentItemFn>, Option<Location>)> = const { RefCell::new((None, None)) });

pub fn init() {
    // Install panic hook
    LazyLock::force(&DEFAULT_HOOK); // Install ice hook
}

/// Custom panic hook to add more information when panic occurs during goto-c codegen.
#[allow(clippy::type_complexity)]
static DEFAULT_HOOK: LazyLock<Box<dyn Fn(&panic::PanicHookInfo<'_>) + Sync + Send + 'static>> =
    LazyLock::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Invoke the default handler, which prints the actual panic message and
            // optionally a backtrace.
            (*DEFAULT_HOOK)(info);
            eprintln!();

            // Print the current function if available
            CURRENT_CODEGEN_ITEM.with(|cell| {
                let t = cell.borrow();
                if let Some(get_current_item) = &t.0 {
                    eprintln!("[Kani] current codegen item: {}", get_current_item());
                } else {
                    eprintln!("[Kani] no current codegen item.");
                }
                if let Some(current_loc) = t.1 {
                    eprintln!("[Kani] current codegen location: {current_loc:?}");
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
    pub fn call_with_panic_debug_info<
        D: CrateDef,
        F: FnOnce(&mut GotocCtx<'tcx>),
        S: Fn() -> String + 'static,
    >(
        &mut self,
        call: F,
        debug_str_generator: S,
        def: D,
    ) {
        CURRENT_CODEGEN_ITEM.with(|odb_cell| {
            odb_cell.replace((
                Some(Box::new(debug_str_generator)),
                Some(self.codegen_span_stable(def.span())),
            ));
            call(self);
            odb_cell.replace((None, None));
        });
    }

    pub fn print_instance(&self, instance: Instance, body: &Body) {
        if cfg!(debug_assertions) {
            debug!("handling {}", instance.name(),);
            debug!("variables: ");
            for (idx, decl) in body.locals().iter().enumerate() {
                debug!("let _{idx}: {:?}", decl.ty);
            }
            for (bb, bbd) in body.blocks.iter().enumerate() {
                debug!("block {:?}", bb);
                for stmt in &bbd.statements {
                    debug!("{:?}", stmt);
                }
                debug!("{:?}", bbd.terminator.kind);
            }
        }
    }
}
