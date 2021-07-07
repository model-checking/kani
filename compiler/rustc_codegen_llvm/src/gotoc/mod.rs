// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use bitflags::_core::any::Any;
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
use rustc_middle::mir::{BasicBlock, BasicBlockData, Body, HasLocalDecls};
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
pub mod cbmc;
mod current_fn;
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

// Use a thread-local global variable to track the current codegen item for debugging.
// If RMC panics during codegen, we can grab this item to include the problematic
// codegen item in the panic trace.
thread_local!(static CURRENT_CODEGEN_ITEM: RefCell<Option<String>> = RefCell::new(None));

// Updates the tracking global variable for panic debugging.
fn set_debug_item(s: String) {
    CURRENT_CODEGEN_ITEM.with(|odb_cell| {
        odb_cell.replace(Some(s));
    });
}

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
    pub fn codegen_block(&mut self, bb: BasicBlock, bbd: &BasicBlockData<'tcx>) {
        self.current_fn_mut().set_current_bb(bb);
        let label: String = self.current_fn().find_label(&bb);
        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        match bbd.statements.len() {
            0 => {
                let term = bbd.terminator();
                let tcode = self.codegen_terminator(term);
                self.current_fn_mut().push_onto_block(tcode.with_label(label));
            }
            _ => {
                let stmt = &bbd.statements[0];
                let scode = self.codegen_statement(stmt);
                self.current_fn_mut().push_onto_block(scode.with_label(label));

                for s in &bbd.statements[1..] {
                    let stmt = self.codegen_statement(s);
                    self.current_fn_mut().push_onto_block(stmt);
                }
                let term = self.codegen_terminator(bbd.terminator());
                self.current_fn_mut().push_onto_block(term);
            }
        }
        self.current_fn_mut().reset_current_bb();
    }

    fn codegen_declare_variables(&mut self) {
        let mir = self.current_fn().mir();
        let ldecls = mir.local_decls();
        ldecls.indices().for_each(|lc| {
            let base_name = match self.find_debug_info(&lc) {
                None => format!("var_{}", lc.index()),
                Some(info) => format!("{}", info.name),
            };
            let name = self.codegen_var_name(&lc);
            let ldata = &ldecls[lc];
            let t = self.monomorphize(ldata.ty);
            let t = self.codegen_ty(t);
            let loc = self.codegen_span2(&ldata.source_info.span);
            let sym =
                Symbol::variable(name, base_name, t, self.codegen_span2(&ldata.source_info.span));
            let sym_e = sym.to_expr();
            self.symbol_table.insert(sym);

            // declare local variables. 0 represents return value
            if lc.index() < 1 || lc.index() > mir.arg_count {
                self.current_fn_mut().push_onto_block(Stmt::decl(sym_e, None, loc));
            }
        });
    }

    /// collect all labels for goto
    fn codegen_prepare_blocks(&self) -> Vec<String> {
        self.current_fn().mir().basic_blocks().indices().map(|bb| format!("{:?}", bb)).collect()
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

    pub fn codegen_function(&mut self, instance: Instance<'tcx>) {
        self.set_current_fn(instance);
        let name = self.current_fn().name();
        let old_sym = self.symbol_table.lookup(&name).unwrap();
        assert!(old_sym.is_function());
        if old_sym.is_function_definition() {
            warn!("Double codegen of {:?}", old_sym);
        } else if self.should_skip_current_fn() {
            debug!("Skipping function {}", self.current_fn().readable_name());
            let loc = self.codegen_span2(&self.current_fn().mir().span);
            let body = Stmt::assert_false(
                &format!(
                    "The function {} is not currently supported by RMC",
                    self.current_fn().readable_name()
                ),
                loc,
            );
            self.symbol_table.update_fn_declaration_with_definition(&name, body);
        } else {
            let mir = self.current_fn().mir();
            self.print_instance(instance, mir);
            let labels = self.codegen_prepare_blocks();
            self.current_fn_mut().set_labels(labels);
            self.codegen_declare_variables();

            mir.basic_blocks().iter_enumerated().for_each(|(bb, bbd)| self.codegen_block(bb, bbd));

            let loc = self.codegen_span2(&mir.span);
            let stmts = self.current_fn_mut().extract_block();
            let body = Stmt::block(stmts, loc);
            self.symbol_table.update_fn_declaration_with_definition(&name, body);
        }
        self.reset_current_fn();
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
        let symbol = Symbol::variable(symbol_name.to_string(), symbol_name, typ, location)
            .with_is_thread_local(false)
            .with_is_static_lifetime(true);
        self.symbol_table.insert(symbol);
    }

    fn declare_function(&mut self, instance: Instance<'tcx>) {
        debug!("declaring {}; {:?}", instance, instance);
        self.set_current_fn(instance);
        self.ensure(&self.current_fn().name(), |ctx, fname| {
            let mir = ctx.current_fn().mir();
            Symbol::function(
                fname,
                ctx.fn_typ(),
                None,
                Some(ctx.current_fn().readable_name().to_string()),
                ctx.codegen_span2(&mir.span),
            )
        });
        self.reset_current_fn();
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
                        set_debug_item(format!(
                            "declare_function: {}",
                            c.readable_instance_name(instance)
                        ));
                        c.declare_function(instance)
                    }
                    MonoItem::Static(def_id) => {
                        set_debug_item(format!("declare_static: {:?}", def_id));
                        c.declare_static(def_id, item)
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
                        set_debug_item(format!(
                            "codegen_function: {}",
                            c.readable_instance_name(instance)
                        ));
                        c.codegen_function(instance)
                    }
                    MonoItem::Static(def_id) => {
                        set_debug_item(format!("codegen_static: {:?}", def_id));
                        c.codegen_static(def_id, item)
                    }
                    MonoItem::GlobalAsm(_) => {} // We have already warned above
                }
            }
        }

        Box::new(GotocCodegenResult {
            symtab: c.symbol_table,
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
