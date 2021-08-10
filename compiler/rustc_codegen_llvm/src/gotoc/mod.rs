// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use bitflags::_core::any::Any;
use cbmc::goto_program::symtab_transformer;
use cbmc::goto_program::{Stmt, Symbol, SymbolTable};
use cbmc::{MachineModel, RoundingMode};
use metadata::*;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::ErrorReported;
use rustc_hir::def_id::DefId;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::middle::cstore::{EncodedMetadata, MetadataLoaderDyn};
use rustc_middle::mir::mono::{CodegenUnit, MonoItem};
use rustc_middle::mir::Body;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::{self, Instance, TyCtxt};
use rustc_serialize::json::ToJson;
use rustc_session::config::{OutputFilenames, OutputType};
use rustc_session::Session;
use rustc_target::abi::Endian;
use std::cell::RefCell;
use std::lazy::SyncLazy;
use std::panic;
use tracing::{debug, warn};

mod assumptions;
mod backend;
mod block;
pub mod cbmc;
mod current_fn;
mod function;
mod hooks;
mod intrinsic;
mod metadata;
mod monomorphize;
mod operand;
mod place;
mod rvalue;
mod statement;
pub mod stubs;
mod typ;
mod utils;

// #[derive(RustcEncodable, RustcDecodable)]
pub struct GotocCodegenResult {
    pub symtab: SymbolTable,
    pub crate_name: rustc_span::Symbol,
}

// Use a thread-local global variable to track the current codegen item for debugging.
// If RMC panics during codegen, we can grab this item to include the problematic
// codegen item in the panic trace.
thread_local!(static CURRENT_CODEGEN_ITEM: RefCell<Option<String>> = RefCell::new(None));

// Include RMC's bug reporting URL in our panics.
const BUG_REPORT_URL: &str =
    "https://github.com/model-checking/rmc/issues/new?labels=bug&template=bug_report.md";

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

#[derive(Clone)]
pub struct GotocCodegenBackend();

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

    fn should_skip_current_fn(&self) -> bool {
        match self.current_fn().readable_name() {
            // https://github.com/model-checking/rmc/issues/202
            "fmt::ArgumentV1::<'a>::as_usize" => true,
            // https://github.com/model-checking/rmc/issues/204
            name if name.ends_with("__getit") => true,
            // https://github.com/model-checking/rmc/issues/205
            "panic::Location::<'a>::caller" => true,
            // https://github.com/model-checking/rmc/issues/207
            "core::slice::<impl [T]>::split_first" => true,
            // https://github.com/model-checking/rmc/issues/281
            name if name.starts_with("bridge::client") => true,
            // https://github.com/model-checking/rmc/issues/282
            "bridge::closure::Closure::<'a, A, R>::call" => true,
            _ => false,
        }
    }

    pub fn codegen_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        debug!("codegen_static");
        let alloc = self.tcx.eval_static_initializer(def_id).unwrap();
        let symbol_name = item.symbol_name(self.tcx).to_string();
        self.codegen_allocation(alloc, |_| symbol_name.clone(), Some(symbol_name.clone()));
    }

    fn print_instance(&self, instance: Instance<'_>, mir: &'tcx Body<'tcx>) {
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

    fn declare_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        debug!("declare_static {:?}", def_id);
        let symbol_name = item.symbol_name(self.tcx).to_string();
        let typ = self.codegen_ty(self.tcx.type_of(def_id));
        let span = self.tcx.def_span(def_id);
        let location = self.codegen_span2(&span);
        let symbol = Symbol::static_variable(symbol_name.to_string(), symbol_name, typ, location);
        self.symbol_table.insert(symbol);
    }
}

impl GotocCodegenBackend {
    pub fn new() -> Box<dyn CodegenBackend> {
        Box::new(GotocCodegenBackend())
    }
}

fn machine_model_from_session(sess: &Session) -> MachineModel {
    // TODO: Hardcoded values from from the ones currently used in env.rs
    // We may wish to get more of them from the session.
    let alignment = sess.target.options.min_global_align.unwrap_or(1);
    let architecture = &sess.target.arch;
    let bool_width = 8;
    let char_is_unsigned = false;
    let char_width = 8;
    let double_width = 64;
    let float_width = 32;
    let int_width = 32;
    let is_big_endian = match sess.target.options.endian {
        Endian::Little => false,
        Endian::Big => true,
    };
    let long_double_width = 128;
    let long_int_width = 64;
    let long_long_int_width = 64;
    let memory_operand_size = 4;
    let null_is_zero = true;
    let pointer_width = sess.target.pointer_width.into();
    let short_int_width = 16;
    let single_width = 32;
    let wchar_t_is_unsigned = false;
    let wchar_t_width = 32;
    let word_size = 32;
    let rounding_mode = RoundingMode::ToNearest;

    MachineModel::new(
        alignment,
        architecture,
        bool_width,
        char_is_unsigned,
        char_width,
        double_width,
        float_width,
        int_width,
        is_big_endian,
        long_double_width,
        long_int_width,
        long_long_int_width,
        memory_operand_size,
        null_is_zero,
        pointer_width,
        rounding_mode,
        short_int_width,
        single_width,
        wchar_t_is_unsigned,
        wchar_t_width,
        word_size,
    )
}

impl CodegenBackend for GotocCodegenBackend {
    fn metadata_loader(&self) -> Box<MetadataLoaderDyn> {
        Box::new(rustc_codegen_ssa::back::metadata::DefaultMetadataLoader)
    }

    fn provide(&self, providers: &mut Providers) {
        monomorphize::partitioning::provide(providers);
    }

    fn provide_extern(&self, _providers: &mut ty::query::Providers) {}

    fn codegen_crate<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        _metadata: EncodedMetadata,
        _need_metadata_module: bool,
    ) -> Box<dyn Any> {
        use rustc_hir::def_id::LOCAL_CRATE;

        // Install panic hook
        SyncLazy::force(&DEFAULT_HOOK); // Install ice hook

        let codegen_units: &'tcx [CodegenUnit<'_>] = tcx.collect_and_partition_mono_items(()).1;
        let mm = machine_model_from_session(&tcx.sess);
        let mut c = GotocCtx::new(tcx, SymbolTable::new(mm));

        // we first declare all functions
        for cgu in codegen_units {
            let items = cgu.items_in_deterministic_order(tcx);
            for (item, _) in items {
                match item {
                    MonoItem::Fn(instance) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.declare_function(instance),
                            format!("declare_function: {}", c.readable_instance_name(instance)),
                        );
                    }
                    MonoItem::Static(def_id) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.declare_static(def_id, item),
                            format!("declare_static: {:?}", def_id),
                        );
                    }
                    MonoItem::GlobalAsm(_) => {
                        warn!(
                            "Crate {} contains global ASM, which is not handled by RMC",
                            c.crate_name()
                        );
                    }
                }
            }
        }

        // then we move on to codegen
        for cgu in codegen_units {
            let items = cgu.items_in_deterministic_order(tcx);
            for (item, _) in items {
                match item {
                    MonoItem::Fn(instance) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.codegen_function(instance),
                            format!("codegen_function: {}", c.readable_instance_name(instance)),
                        );
                    }
                    MonoItem::Static(def_id) => {
                        c.call_with_panic_debug_info(
                            |ctx| ctx.codegen_static(def_id, item),
                            format!("codegen_static: {:?}", def_id),
                        );
                    }
                    MonoItem::GlobalAsm(_) => {} // We have already warned above
                }
            }
        }

        // perform post-processing symbol table passes
        let symbol_table = symtab_transformer::do_passes(
            c.symbol_table,
            &tcx.sess.opts.debugging_opts.symbol_table_passes,
        );

        Box::new(GotocCodegenResult {
            symtab: symbol_table,
            crate_name: tcx.crate_name(LOCAL_CRATE) as rustc_span::Symbol,
        })
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        _sess: &Session,
    ) -> Result<(Box<dyn Any>, FxHashMap<WorkProductId, WorkProduct>), ErrorReported> {
        Ok((ongoing_codegen, FxHashMap::default()))
    }

    fn link(
        &self,
        _sess: &Session,
        codegen_results: Box<dyn Any>,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorReported> {
        use std::io::Write;

        let result = codegen_results
            .downcast::<GotocCodegenResult>()
            .expect("in link: codegen_results is not a GotocCodegenResult");
        let symtab = result.symtab;
        let irep_symtab = symtab.to_irep();
        let json = irep_symtab.to_json();
        let pretty_json = json.pretty();

        let output_name = outputs.path(OutputType::Object).with_extension("json");
        debug!("output to {:?}", output_name);
        let mut out_file = ::std::fs::File::create(output_name).unwrap();
        write!(out_file, "{}", pretty_json.to_string()).unwrap();

        Ok(())
    }
}
