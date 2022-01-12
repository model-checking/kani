use crate::back::lto::ThinBuffer;
use crate::back::profiling::{
    selfprofile_after_pass_callback, selfprofile_before_pass_callback, LlvmSelfProfiler,
};
use crate::base;
use crate::common;
use crate::consts;
use crate::llvm::{self, DiagnosticInfo, PassManager, SMDiagnostic};
use crate::llvm_util;
use crate::type_::Type;
use crate::LlvmCodegenBackend;
use crate::ModuleLlvm;
use rustc_codegen_ssa::back::link::ensure_removed;
use rustc_codegen_ssa::back::write::{
    BitcodeSection, CodegenContext, EmitObj, ModuleConfig, TargetMachineFactoryConfig,
    TargetMachineFactoryFn,
};
use rustc_codegen_ssa::traits::*;
use rustc_codegen_ssa::{CompiledModule, ModuleCodegen};
use rustc_data_structures::profiling::SelfProfilerRef;
use rustc_data_structures::small_c_str::SmallCStr;
use rustc_errors::{FatalError, Handler, Level};
use rustc_fs_util::{link_or_copy, path_to_c_string};
use rustc_middle::bug;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::{self, Lto, OutputType, Passes, SplitDwarfKind, SwitchWithOptPath};
use rustc_session::Session;
use rustc_span::symbol::sym;
use rustc_span::InnerSpan;
use rustc_target::spec::{CodeModel, RelocModel, SanitizerSet, SplitDebuginfo};
use tracing::debug;

use libc::{c_char, c_int, c_uint, c_void, size_t};
use std::ffi::CString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::slice;
use std::str;
use std::sync::Arc;

pub fn llvm_err(handler: &rustc_errors::Handler, msg: &str) -> FatalError {
    match llvm::last_error() {
        Some(err) => handler.fatal(&format!("{}: {}", msg, err)),
        None => handler.fatal(msg),
    }
}

pub fn write_output_file<'ll>(
    handler: &rustc_errors::Handler,
    target: &'ll llvm::TargetMachine,
    pm: &llvm::PassManager<'ll>,
    m: &'ll llvm::Module,
    output: &Path,
    dwo_output: Option<&Path>,
    file_type: llvm::FileType,
    self_profiler_ref: &SelfProfilerRef,
) -> Result<(), FatalError> {
    unsafe {
        let output_c = path_to_c_string(output);
        let result = if let Some(dwo_output) = dwo_output {
            let dwo_output_c = path_to_c_string(dwo_output);
            llvm::LLVMRustWriteOutputFile(
                target,
                pm,
                m,
                output_c.as_ptr(),
                dwo_output_c.as_ptr(),
                file_type,
            )
        } else {
            llvm::LLVMRustWriteOutputFile(
                target,
                pm,
                m,
                output_c.as_ptr(),
                std::ptr::null(),
                file_type,
            )
        };

        // Record artifact sizes for self-profiling
        if result == llvm::LLVMRustResult::Success {
            let artifact_kind = match file_type {
                llvm::FileType::ObjectFile => "object_file",
                llvm::FileType::AssemblyFile => "assembly_file",
            };
            record_artifact_size(self_profiler_ref, artifact_kind, output);
            if let Some(dwo_file) = dwo_output {
                record_artifact_size(self_profiler_ref, "dwo_file", dwo_file);
            }
        }

        result.into_result().map_err(|()| {
            let msg = format!("could not write output to {}", output.display());
            llvm_err(handler, &msg)
        })
    }
}

pub fn create_informational_target_machine(sess: &Session) -> &'static mut llvm::TargetMachine {
    let config = TargetMachineFactoryConfig { split_dwarf_file: None };
    target_machine_factory(sess, config::OptLevel::No)(config)
        .unwrap_or_else(|err| llvm_err(sess.diagnostic(), &err).raise())
}

pub fn create_target_machine(tcx: TyCtxt<'_>, mod_name: &str) -> &'static mut llvm::TargetMachine {
    let split_dwarf_file = if tcx.sess.target_can_use_split_dwarf() {
        tcx.output_filenames(()).split_dwarf_path(
            tcx.sess.split_debuginfo(),
            tcx.sess.opts.debugging_opts.split_dwarf_kind,
            Some(mod_name),
        )
    } else {
        None
    };
    let config = TargetMachineFactoryConfig { split_dwarf_file };
    target_machine_factory(tcx.sess, tcx.backend_optimization_level(()))(config)
        .unwrap_or_else(|err| llvm_err(tcx.sess.diagnostic(), &err).raise())
}

pub fn to_llvm_opt_settings(
    cfg: config::OptLevel,
) -> (llvm::CodeGenOptLevel, llvm::CodeGenOptSize) {
    use self::config::OptLevel::*;
    match cfg {
        No => (llvm::CodeGenOptLevel::None, llvm::CodeGenOptSizeNone),
        Less => (llvm::CodeGenOptLevel::Less, llvm::CodeGenOptSizeNone),
        Default => (llvm::CodeGenOptLevel::Default, llvm::CodeGenOptSizeNone),
        Aggressive => (llvm::CodeGenOptLevel::Aggressive, llvm::CodeGenOptSizeNone),
        Size => (llvm::CodeGenOptLevel::Default, llvm::CodeGenOptSizeDefault),
        SizeMin => (llvm::CodeGenOptLevel::Default, llvm::CodeGenOptSizeAggressive),
    }
}

fn to_pass_builder_opt_level(cfg: config::OptLevel) -> llvm::PassBuilderOptLevel {
    use config::OptLevel::*;
    match cfg {
        No => llvm::PassBuilderOptLevel::O0,
        Less => llvm::PassBuilderOptLevel::O1,
        Default => llvm::PassBuilderOptLevel::O2,
        Aggressive => llvm::PassBuilderOptLevel::O3,
        Size => llvm::PassBuilderOptLevel::Os,
        SizeMin => llvm::PassBuilderOptLevel::Oz,
    }
}

fn to_llvm_relocation_model(relocation_model: RelocModel) -> llvm::RelocModel {
    match relocation_model {
        RelocModel::Static => llvm::RelocModel::Static,
        // LLVM doesn't have a PIE relocation model, it represents PIE as PIC with an extra attribute.
        RelocModel::Pic | RelocModel::Pie => llvm::RelocModel::PIC,
        RelocModel::DynamicNoPic => llvm::RelocModel::DynamicNoPic,
        RelocModel::Ropi => llvm::RelocModel::ROPI,
        RelocModel::Rwpi => llvm::RelocModel::RWPI,
        RelocModel::RopiRwpi => llvm::RelocModel::ROPI_RWPI,
    }
}

pub(crate) fn to_llvm_code_model(code_model: Option<CodeModel>) -> llvm::CodeModel {
    match code_model {
        Some(CodeModel::Tiny) => llvm::CodeModel::Tiny,
        Some(CodeModel::Small) => llvm::CodeModel::Small,
        Some(CodeModel::Kernel) => llvm::CodeModel::Kernel,
        Some(CodeModel::Medium) => llvm::CodeModel::Medium,
        Some(CodeModel::Large) => llvm::CodeModel::Large,
        None => llvm::CodeModel::None,
    }
}

pub fn target_machine_factory(
    sess: &Session,
    optlvl: config::OptLevel,
) -> TargetMachineFactoryFn<LlvmCodegenBackend> {
    let reloc_model = to_llvm_relocation_model(sess.relocation_model());

    let (opt_level, _) = to_llvm_opt_settings(optlvl);
    let use_softfp = sess.opts.cg.soft_float;

    let ffunction_sections =
        sess.opts.debugging_opts.function_sections.unwrap_or(sess.target.function_sections);
    let fdata_sections = ffunction_sections;
    let funique_section_names = !sess.opts.debugging_opts.no_unique_section_names;

    let code_model = to_llvm_code_model(sess.code_model());

    let mut singlethread = sess.target.singlethread;

    // On the wasm target once the `atomics` feature is enabled that means that
    // we're no longer single-threaded, or otherwise we don't want LLVM to
    // lower atomic operations to single-threaded operations.
    if singlethread && sess.target.is_like_wasm && sess.target_features.contains(&sym::atomics) {
        singlethread = false;
    }

    let triple = SmallCStr::new(&sess.target.llvm_target);
    let cpu = SmallCStr::new(llvm_util::target_cpu(sess));
    let features = llvm_util::llvm_global_features(sess).join(",");
    let features = CString::new(features).unwrap();
    let abi = SmallCStr::new(&sess.target.llvm_abiname);
    let trap_unreachable =
        sess.opts.debugging_opts.trap_unreachable.unwrap_or(sess.target.trap_unreachable);
    let emit_stack_size_section = sess.opts.debugging_opts.emit_stack_sizes;

    let asm_comments = sess.asm_comments();
    let relax_elf_relocations =
        sess.opts.debugging_opts.relax_elf_relocations.unwrap_or(sess.target.relax_elf_relocations);

    let use_init_array =
        !sess.opts.debugging_opts.use_ctors_section.unwrap_or(sess.target.use_ctors_section);

    let path_mapping = sess.source_map().path_mapping().clone();

    Arc::new(move |config: TargetMachineFactoryConfig| {
        let split_dwarf_file =
            path_mapping.map_prefix(config.split_dwarf_file.unwrap_or_default()).0;
        let split_dwarf_file = CString::new(split_dwarf_file.to_str().unwrap()).unwrap();

        let tm = unsafe {
            llvm::LLVMRustCreateTargetMachine(
                triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                abi.as_ptr(),
                code_model,
                reloc_model,
                opt_level,
                use_softfp,
                ffunction_sections,
                fdata_sections,
                funique_section_names,
                trap_unreachable,
                singlethread,
                asm_comments,
                emit_stack_size_section,
                relax_elf_relocations,
                use_init_array,
                split_dwarf_file.as_ptr(),
            )
        };

        tm.ok_or_else(|| {
            format!("Could not create LLVM TargetMachine for triple: {}", triple.to_str().unwrap())
        })
    })
}

pub(crate) fn save_temp_bitcode(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    module: &ModuleCodegen<ModuleLlvm>,
    name: &str,
) {
    if !cgcx.save_temps {
        return;
    }
    unsafe {
        let ext = format!("{}.bc", name);
        let cgu = Some(&module.name[..]);
        let path = cgcx.output_filenames.temp_path_ext(&ext, cgu);
        let cstr = path_to_c_string(&path);
        let llmod = module.module_llvm.llmod();
        llvm::LLVMWriteBitcodeToFile(llmod, cstr.as_ptr());
    }
}

pub struct DiagnosticHandlers<'a> {
    data: *mut (&'a CodegenContext<LlvmCodegenBackend>, &'a Handler),
    llcx: &'a llvm::Context,
    old_handler: Option<&'a llvm::DiagnosticHandler>,
}

impl<'a> DiagnosticHandlers<'a> {
    pub fn new(
        cgcx: &'a CodegenContext<LlvmCodegenBackend>,
        handler: &'a Handler,
        llcx: &'a llvm::Context,
    ) -> Self {
        let remark_passes_all: bool;
        let remark_passes: Vec<CString>;
        match &cgcx.remark {
            Passes::All => {
                remark_passes_all = true;
                remark_passes = Vec::new();
            }
            Passes::Some(passes) => {
                remark_passes_all = false;
                remark_passes =
                    passes.iter().map(|name| CString::new(name.as_str()).unwrap()).collect();
            }
        };
        let remark_passes: Vec<*const c_char> =
            remark_passes.iter().map(|name: &CString| name.as_ptr()).collect();
        let data = Box::into_raw(Box::new((cgcx, handler)));
        unsafe {
            let old_handler = llvm::LLVMRustContextGetDiagnosticHandler(llcx);
            llvm::LLVMRustContextConfigureDiagnosticHandler(
                llcx,
                diagnostic_handler,
                data.cast(),
                remark_passes_all,
                remark_passes.as_ptr(),
                remark_passes.len(),
            );
            llvm::LLVMRustSetInlineAsmDiagnosticHandler(llcx, inline_asm_handler, data.cast());
            DiagnosticHandlers { data, llcx, old_handler }
        }
    }
}

impl<'a> Drop for DiagnosticHandlers<'a> {
    fn drop(&mut self) {
        use std::ptr::null_mut;
        unsafe {
            llvm::LLVMRustSetInlineAsmDiagnosticHandler(self.llcx, inline_asm_handler, null_mut());
            llvm::LLVMRustContextSetDiagnosticHandler(self.llcx, self.old_handler);
            drop(Box::from_raw(self.data));
        }
    }
}

fn report_inline_asm(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    msg: String,
    level: llvm::DiagnosticLevel,
    mut cookie: c_uint,
    source: Option<(String, Vec<InnerSpan>)>,
) {
    // In LTO build we may get srcloc values from other crates which are invalid
    // since they use a different source map. To be safe we just suppress these
    // in LTO builds.
    if matches!(cgcx.lto, Lto::Fat | Lto::Thin) {
        cookie = 0;
    }
    let level = match level {
        llvm::DiagnosticLevel::Error => Level::Error { lint: false },
        llvm::DiagnosticLevel::Warning => Level::Warning,
        llvm::DiagnosticLevel::Note | llvm::DiagnosticLevel::Remark => Level::Note,
    };
    cgcx.diag_emitter.inline_asm_error(cookie as u32, msg, level, source);
}

unsafe extern "C" fn inline_asm_handler(diag: &SMDiagnostic, user: *const c_void, cookie: c_uint) {
    if user.is_null() {
        return;
    }
    let (cgcx, _) = *(user as *const (&CodegenContext<LlvmCodegenBackend>, &Handler));

    let smdiag = llvm::diagnostic::SrcMgrDiagnostic::unpack(diag);
    report_inline_asm(cgcx, smdiag.message, smdiag.level, cookie, smdiag.source);
}

unsafe extern "C" fn diagnostic_handler(info: &DiagnosticInfo, user: *mut c_void) {
    if user.is_null() {
        return;
    }
    let (cgcx, diag_handler) = *(user as *const (&CodegenContext<LlvmCodegenBackend>, &Handler));

    match llvm::diagnostic::Diagnostic::unpack(info) {
        llvm::diagnostic::InlineAsm(inline) => {
            report_inline_asm(cgcx, inline.message, inline.level, inline.cookie, inline.source);
        }

        llvm::diagnostic::Optimization(opt) => {
            let enabled = match cgcx.remark {
                Passes::All => true,
                Passes::Some(ref v) => v.iter().any(|s| *s == opt.pass_name),
            };

            if enabled {
                diag_handler.note_without_error(&format!(
                    "{}:{}:{}: {}: {}",
                    opt.filename, opt.line, opt.column, opt.pass_name, opt.message,
                ));
            }
        }
        llvm::diagnostic::PGO(diagnostic_ref) | llvm::diagnostic::Linker(diagnostic_ref) => {
            let msg = llvm::build_string(|s| {
                llvm::LLVMRustWriteDiagnosticInfoToString(diagnostic_ref, s)
            })
            .expect("non-UTF8 diagnostic");
            diag_handler.warn(&msg);
        }
        llvm::diagnostic::Unsupported(diagnostic_ref) => {
            let msg = llvm::build_string(|s| {
                llvm::LLVMRustWriteDiagnosticInfoToString(diagnostic_ref, s)
            })
            .expect("non-UTF8 diagnostic");
            diag_handler.err(&msg);
        }
        llvm::diagnostic::UnknownDiagnostic(..) => {}
    }
}

fn get_pgo_gen_path(config: &ModuleConfig) -> Option<CString> {
    match config.pgo_gen {
        SwitchWithOptPath::Enabled(ref opt_dir_path) => {
            let path = if let Some(dir_path) = opt_dir_path {
                dir_path.join("default_%m.profraw")
            } else {
                PathBuf::from("default_%m.profraw")
            };

            Some(CString::new(format!("{}", path.display())).unwrap())
        }
        SwitchWithOptPath::Disabled => None,
    }
}

fn get_pgo_use_path(config: &ModuleConfig) -> Option<CString> {
    config
        .pgo_use
        .as_ref()
        .map(|path_buf| CString::new(path_buf.to_string_lossy().as_bytes()).unwrap())
}

fn get_pgo_sample_use_path(config: &ModuleConfig) -> Option<CString> {
    config
        .pgo_sample_use
        .as_ref()
        .map(|path_buf| CString::new(path_buf.to_string_lossy().as_bytes()).unwrap())
}

pub(crate) unsafe fn optimize_with_new_llvm_pass_manager(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    diag_handler: &Handler,
    module: &ModuleCodegen<ModuleLlvm>,
    config: &ModuleConfig,
    opt_level: config::OptLevel,
    opt_stage: llvm::OptStage,
) -> Result<(), FatalError> {
    let unroll_loops =
        opt_level != config::OptLevel::Size && opt_level != config::OptLevel::SizeMin;
    let using_thin_buffers = opt_stage == llvm::OptStage::PreLinkThinLTO || config.bitcode_needed();
    let pgo_gen_path = get_pgo_gen_path(config);
    let pgo_use_path = get_pgo_use_path(config);
    let pgo_sample_use_path = get_pgo_sample_use_path(config);
    let is_lto = opt_stage == llvm::OptStage::ThinLTO || opt_stage == llvm::OptStage::FatLTO;
    // Sanitizer instrumentation is only inserted during the pre-link optimization stage.
    let sanitizer_options = if !is_lto {
        Some(llvm::SanitizerOptions {
            sanitize_address: config.sanitizer.contains(SanitizerSet::ADDRESS),
            sanitize_address_recover: config.sanitizer_recover.contains(SanitizerSet::ADDRESS),
            sanitize_memory: config.sanitizer.contains(SanitizerSet::MEMORY),
            sanitize_memory_recover: config.sanitizer_recover.contains(SanitizerSet::MEMORY),
            sanitize_memory_track_origins: config.sanitizer_memory_track_origins as c_int,
            sanitize_thread: config.sanitizer.contains(SanitizerSet::THREAD),
            sanitize_hwaddress: config.sanitizer.contains(SanitizerSet::HWADDRESS),
            sanitize_hwaddress_recover: config.sanitizer_recover.contains(SanitizerSet::HWADDRESS),
        })
    } else {
        None
    };

    let mut llvm_profiler = if cgcx.prof.llvm_recording_enabled() {
        Some(LlvmSelfProfiler::new(cgcx.prof.get_self_profiler().unwrap()))
    } else {
        None
    };

    let llvm_selfprofiler =
        llvm_profiler.as_mut().map(|s| s as *mut _ as *mut c_void).unwrap_or(std::ptr::null_mut());

    let extra_passes = config.passes.join(",");

    let llvm_plugins = config.llvm_plugins.join(",");

    // FIXME: NewPM doesn't provide a facility to pass custom InlineParams.
    // We would have to add upstream support for this first, before we can support
    // config.inline_threshold and our more aggressive default thresholds.
    let result = llvm::LLVMRustOptimizeWithNewPassManager(
        module.module_llvm.llmod(),
        &*module.module_llvm.tm,
        to_pass_builder_opt_level(opt_level),
        opt_stage,
        config.no_prepopulate_passes,
        config.verify_llvm_ir,
        using_thin_buffers,
        config.merge_functions,
        unroll_loops,
        config.vectorize_slp,
        config.vectorize_loop,
        config.no_builtins,
        config.emit_lifetime_markers,
        sanitizer_options.as_ref(),
        pgo_gen_path.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
        pgo_use_path.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
        config.instrument_coverage,
        config.instrument_gcov,
        pgo_sample_use_path.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
        config.debug_info_for_profiling,
        llvm_selfprofiler,
        selfprofile_before_pass_callback,
        selfprofile_after_pass_callback,
        extra_passes.as_ptr().cast(),
        extra_passes.len(),
        llvm_plugins.as_ptr().cast(),
        llvm_plugins.len(),
    );
    result.into_result().map_err(|()| llvm_err(diag_handler, "failed to run LLVM passes"))
}

// Unsafe due to LLVM calls.
pub(crate) unsafe fn optimize(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    diag_handler: &Handler,
    module: &ModuleCodegen<ModuleLlvm>,
    config: &ModuleConfig,
) -> Result<(), FatalError> {
    let _timer = cgcx.prof.generic_activity_with_arg("LLVM_module_optimize", &*module.name);

    let llmod = module.module_llvm.llmod();
    let llcx = &*module.module_llvm.llcx;
    let tm = &*module.module_llvm.tm;
    let _handlers = DiagnosticHandlers::new(cgcx, diag_handler, llcx);

    let module_name = module.name.clone();
    let module_name = Some(&module_name[..]);

    if config.emit_no_opt_bc {
        let out = cgcx.output_filenames.temp_path_ext("no-opt.bc", module_name);
        let out = path_to_c_string(&out);
        llvm::LLVMWriteBitcodeToFile(llmod, out.as_ptr());
    }

    if let Some(opt_level) = config.opt_level {
        if llvm_util::should_use_new_llvm_pass_manager(
            &config.new_llvm_pass_manager,
            &cgcx.target_arch,
        ) {
            let opt_stage = match cgcx.lto {
                Lto::Fat => llvm::OptStage::PreLinkFatLTO,
                Lto::Thin | Lto::ThinLocal => llvm::OptStage::PreLinkThinLTO,
                _ if cgcx.opts.cg.linker_plugin_lto.enabled() => llvm::OptStage::PreLinkThinLTO,
                _ => llvm::OptStage::PreLinkNoLTO,
            };
            return optimize_with_new_llvm_pass_manager(
                cgcx,
                diag_handler,
                module,
                config,
                opt_level,
                opt_stage,
            );
        }

        if cgcx.prof.llvm_recording_enabled() {
            diag_handler
                .warn("`-Z self-profile-events = llvm` requires `-Z new-llvm-pass-manager`");
        }

        // Create the two optimizing pass managers. These mirror what clang
        // does, and are by populated by LLVM's default PassManagerBuilder.
        // Each manager has a different set of passes, but they also share
        // some common passes.
        let fpm = llvm::LLVMCreateFunctionPassManagerForModule(llmod);
        let mpm = llvm::LLVMCreatePassManager();

        {
            let find_pass = |pass_name: &str| {
                let pass_name = SmallCStr::new(pass_name);
                llvm::LLVMRustFindAndCreatePass(pass_name.as_ptr())
            };

            if config.verify_llvm_ir {
                // Verification should run as the very first pass.
                llvm::LLVMRustAddPass(fpm, find_pass("verify").unwrap());
            }

            let mut extra_passes = Vec::new();
            let mut have_name_anon_globals_pass = false;

            for pass_name in &config.passes {
                if pass_name == "lint" {
                    // Linting should also be performed early, directly on the generated IR.
                    llvm::LLVMRustAddPass(fpm, find_pass("lint").unwrap());
                    continue;
                }

                if let Some(pass) = find_pass(pass_name) {
                    extra_passes.push(pass);
                } else {
                    diag_handler.warn(&format!("unknown pass `{}`, ignoring", pass_name));
                }

                if pass_name == "name-anon-globals" {
                    have_name_anon_globals_pass = true;
                }
            }

            // Instrumentation must be inserted before optimization,
            // otherwise LLVM may optimize some functions away which
            // breaks llvm-cov.
            //
            // This mirrors what Clang does in lib/CodeGen/BackendUtil.cpp.
            if config.instrument_gcov {
                llvm::LLVMRustAddPass(mpm, find_pass("insert-gcov-profiling").unwrap());
            }
            if config.instrument_coverage {
                llvm::LLVMRustAddPass(mpm, find_pass("instrprof").unwrap());
            }
            if config.debug_info_for_profiling {
                llvm::LLVMRustAddPass(mpm, find_pass("add-discriminators").unwrap());
            }

            add_sanitizer_passes(config, &mut extra_passes);

            // Some options cause LLVM bitcode to be emitted, which uses ThinLTOBuffers, so we need
            // to make sure we run LLVM's NameAnonGlobals pass when emitting bitcode; otherwise
            // we'll get errors in LLVM.
            let using_thin_buffers = config.bitcode_needed();
            if !config.no_prepopulate_passes {
                llvm::LLVMAddAnalysisPasses(tm, fpm);
                llvm::LLVMAddAnalysisPasses(tm, mpm);
                let opt_level = to_llvm_opt_settings(opt_level).0;
                let prepare_for_thin_lto = cgcx.lto == Lto::Thin
                    || cgcx.lto == Lto::ThinLocal
                    || (cgcx.lto != Lto::Fat && cgcx.opts.cg.linker_plugin_lto.enabled());
                with_llvm_pmb(llmod, config, opt_level, prepare_for_thin_lto, &mut |b| {
                    llvm::LLVMRustAddLastExtensionPasses(
                        b,
                        extra_passes.as_ptr(),
                        extra_passes.len() as size_t,
                    );
                    llvm::LLVMPassManagerBuilderPopulateFunctionPassManager(b, fpm);
                    llvm::LLVMPassManagerBuilderPopulateModulePassManager(b, mpm);
                });

                have_name_anon_globals_pass = have_name_anon_globals_pass || prepare_for_thin_lto;
                if using_thin_buffers && !prepare_for_thin_lto {
                    llvm::LLVMRustAddPass(mpm, find_pass("name-anon-globals").unwrap());
                    have_name_anon_globals_pass = true;
                }
            } else {
                // If we don't use the standard pipeline, directly populate the MPM
                // with the extra passes.
                for pass in extra_passes {
                    llvm::LLVMRustAddPass(mpm, pass);
                }
            }

            if using_thin_buffers && !have_name_anon_globals_pass {
                // As described above, this will probably cause an error in LLVM
                if config.no_prepopulate_passes {
                    diag_handler.err(
                        "The current compilation is going to use thin LTO buffers \
                                      without running LLVM's NameAnonGlobals pass. \
                                      This will likely cause errors in LLVM. Consider adding \
                                      -C passes=name-anon-globals to the compiler command line.",
                    );
                } else {
                    bug!(
                        "We are using thin LTO buffers without running the NameAnonGlobals pass. \
                          This will likely cause errors in LLVM and should never happen."
                    );
                }
            }
        }

        diag_handler.abort_if_errors();

        // Finally, run the actual optimization passes
        {
            let _timer = cgcx.prof.extra_verbose_generic_activity(
                "LLVM_module_optimize_function_passes",
                &*module.name,
            );
            llvm::LLVMRustRunFunctionPassManager(fpm, llmod);
        }
        {
            let _timer = cgcx.prof.extra_verbose_generic_activity(
                "LLVM_module_optimize_module_passes",
                &*module.name,
            );
            llvm::LLVMRunPassManager(mpm, llmod);
        }

        // Deallocate managers that we're now done with
        llvm::LLVMDisposePassManager(fpm);
        llvm::LLVMDisposePassManager(mpm);
    }
    Ok(())
}

unsafe fn add_sanitizer_passes(config: &ModuleConfig, passes: &mut Vec<&'static mut llvm::Pass>) {
    if config.sanitizer.contains(SanitizerSet::ADDRESS) {
        let recover = config.sanitizer_recover.contains(SanitizerSet::ADDRESS);
        passes.push(llvm::LLVMRustCreateAddressSanitizerFunctionPass(recover));
        passes.push(llvm::LLVMRustCreateModuleAddressSanitizerPass(recover));
    }
    if config.sanitizer.contains(SanitizerSet::MEMORY) {
        let track_origins = config.sanitizer_memory_track_origins as c_int;
        let recover = config.sanitizer_recover.contains(SanitizerSet::MEMORY);
        passes.push(llvm::LLVMRustCreateMemorySanitizerPass(track_origins, recover));
    }
    if config.sanitizer.contains(SanitizerSet::THREAD) {
        passes.push(llvm::LLVMRustCreateThreadSanitizerPass());
    }
    if config.sanitizer.contains(SanitizerSet::HWADDRESS) {
        let recover = config.sanitizer_recover.contains(SanitizerSet::HWADDRESS);
        passes.push(llvm::LLVMRustCreateHWAddressSanitizerPass(recover));
    }
}

pub(crate) fn link(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    diag_handler: &Handler,
    mut modules: Vec<ModuleCodegen<ModuleLlvm>>,
) -> Result<ModuleCodegen<ModuleLlvm>, FatalError> {
    use super::lto::{Linker, ModuleBuffer};
    // Sort the modules by name to ensure to ensure deterministic behavior.
    modules.sort_by(|a, b| a.name.cmp(&b.name));
    let (first, elements) =
        modules.split_first().expect("Bug! modules must contain at least one module.");

    let mut linker = Linker::new(first.module_llvm.llmod());
    for module in elements {
        let _timer =
            cgcx.prof.generic_activity_with_arg("LLVM_link_module", format!("{:?}", module.name));
        let buffer = ModuleBuffer::new(module.module_llvm.llmod());
        linker.add(buffer.data()).map_err(|()| {
            let msg = format!("failed to serialize module {:?}", module.name);
            llvm_err(diag_handler, &msg)
        })?;
    }
    drop(linker);
    Ok(modules.remove(0))
}

pub(crate) unsafe fn codegen(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    diag_handler: &Handler,
    module: ModuleCodegen<ModuleLlvm>,
    config: &ModuleConfig,
) -> Result<CompiledModule, FatalError> {
    let _timer = cgcx.prof.generic_activity_with_arg("LLVM_module_codegen", &*module.name);
    {
        let llmod = module.module_llvm.llmod();
        let llcx = &*module.module_llvm.llcx;
        let tm = &*module.module_llvm.tm;
        let module_name = module.name.clone();
        let module_name = Some(&module_name[..]);
        let handlers = DiagnosticHandlers::new(cgcx, diag_handler, llcx);

        if cgcx.msvc_imps_needed {
            create_msvc_imps(cgcx, llcx, llmod);
        }

        // A codegen-specific pass manager is used to generate object
        // files for an LLVM module.
        //
        // Apparently each of these pass managers is a one-shot kind of
        // thing, so we create a new one for each type of output. The
        // pass manager passed to the closure should be ensured to not
        // escape the closure itself, and the manager should only be
        // used once.
        unsafe fn with_codegen<'ll, F, R>(
            tm: &'ll llvm::TargetMachine,
            llmod: &'ll llvm::Module,
            no_builtins: bool,
            f: F,
        ) -> R
        where
            F: FnOnce(&'ll mut PassManager<'ll>) -> R,
        {
            let cpm = llvm::LLVMCreatePassManager();
            llvm::LLVMAddAnalysisPasses(tm, cpm);
            llvm::LLVMRustAddLibraryInfo(cpm, llmod, no_builtins);
            f(cpm)
        }

        // Two things to note:
        // - If object files are just LLVM bitcode we write bitcode, copy it to
        //   the .o file, and delete the bitcode if it wasn't otherwise
        //   requested.
        // - If we don't have the integrated assembler then we need to emit
        //   asm from LLVM and use `gcc` to create the object file.

        let bc_out = cgcx.output_filenames.temp_path(OutputType::Bitcode, module_name);
        let obj_out = cgcx.output_filenames.temp_path(OutputType::Object, module_name);

        if config.bitcode_needed() {
            let _timer = cgcx
                .prof
                .generic_activity_with_arg("LLVM_module_codegen_make_bitcode", &*module.name);
            let thin = ThinBuffer::new(llmod);
            let data = thin.data();

            if let Some(bitcode_filename) = bc_out.file_name() {
                cgcx.prof.artifact_size(
                    "llvm_bitcode",
                    bitcode_filename.to_string_lossy(),
                    data.len() as u64,
                );
            }

            if config.emit_bc || config.emit_obj == EmitObj::Bitcode {
                let _timer = cgcx
                    .prof
                    .generic_activity_with_arg("LLVM_module_codegen_emit_bitcode", &*module.name);
                if let Err(e) = fs::write(&bc_out, data) {
                    let msg = format!("failed to write bytecode to {}: {}", bc_out.display(), e);
                    diag_handler.err(&msg);
                }
            }

            if config.emit_obj == EmitObj::ObjectCode(BitcodeSection::Full) {
                let _timer = cgcx
                    .prof
                    .generic_activity_with_arg("LLVM_module_codegen_embed_bitcode", &*module.name);
                embed_bitcode(cgcx, llcx, llmod, &config.bc_cmdline, data);
            }
        }

        if config.emit_ir {
            let _timer =
                cgcx.prof.generic_activity_with_arg("LLVM_module_codegen_emit_ir", &*module.name);
            let out = cgcx.output_filenames.temp_path(OutputType::LlvmAssembly, module_name);
            let out_c = path_to_c_string(&out);

            extern "C" fn demangle_callback(
                input_ptr: *const c_char,
                input_len: size_t,
                output_ptr: *mut c_char,
                output_len: size_t,
            ) -> size_t {
                let input =
                    unsafe { slice::from_raw_parts(input_ptr as *const u8, input_len as usize) };

                let input = match str::from_utf8(input) {
                    Ok(s) => s,
                    Err(_) => return 0,
                };

                let output = unsafe {
                    slice::from_raw_parts_mut(output_ptr as *mut u8, output_len as usize)
                };
                let mut cursor = io::Cursor::new(output);

                let demangled = match rustc_demangle::try_demangle(input) {
                    Ok(d) => d,
                    Err(_) => return 0,
                };

                if write!(cursor, "{:#}", demangled).is_err() {
                    // Possible only if provided buffer is not big enough
                    return 0;
                }

                cursor.position() as size_t
            }

            let result = llvm::LLVMRustPrintModule(llmod, out_c.as_ptr(), demangle_callback);

            if result == llvm::LLVMRustResult::Success {
                record_artifact_size(&cgcx.prof, "llvm_ir", &out);
            }

            result.into_result().map_err(|()| {
                let msg = format!("failed to write LLVM IR to {}", out.display());
                llvm_err(diag_handler, &msg)
            })?;
        }

        if config.emit_asm {
            let _timer =
                cgcx.prof.generic_activity_with_arg("LLVM_module_codegen_emit_asm", &*module.name);
            let path = cgcx.output_filenames.temp_path(OutputType::Assembly, module_name);

            // We can't use the same module for asm and object code output,
            // because that triggers various errors like invalid IR or broken
            // binaries. So we must clone the module to produce the asm output
            // if we are also producing object code.
            let llmod = if let EmitObj::ObjectCode(_) = config.emit_obj {
                llvm::LLVMCloneModule(llmod)
            } else {
                llmod
            };
            with_codegen(tm, llmod, config.no_builtins, |cpm| {
                write_output_file(
                    diag_handler,
                    tm,
                    cpm,
                    llmod,
                    &path,
                    None,
                    llvm::FileType::AssemblyFile,
                    &cgcx.prof,
                )
            })?;
        }

        match config.emit_obj {
            EmitObj::ObjectCode(_) => {
                let _timer = cgcx
                    .prof
                    .generic_activity_with_arg("LLVM_module_codegen_emit_obj", &*module.name);

                let dwo_out = cgcx.output_filenames.temp_path_dwo(module_name);
                let dwo_out = match (cgcx.split_debuginfo, cgcx.split_dwarf_kind) {
                    // Don't change how DWARF is emitted when disabled.
                    (SplitDebuginfo::Off, _) => None,
                    // Don't provide a DWARF object path if split debuginfo is enabled but this is
                    // a platform that doesn't support Split DWARF.
                    _ if !cgcx.target_can_use_split_dwarf => None,
                    // Don't provide a DWARF object path in single mode, sections will be written
                    // into the object as normal but ignored by linker.
                    (_, SplitDwarfKind::Single) => None,
                    // Emit (a subset of the) DWARF into a separate dwarf object file in split
                    // mode.
                    (_, SplitDwarfKind::Split) => Some(dwo_out.as_path()),
                };

                with_codegen(tm, llmod, config.no_builtins, |cpm| {
                    write_output_file(
                        diag_handler,
                        tm,
                        cpm,
                        llmod,
                        &obj_out,
                        dwo_out,
                        llvm::FileType::ObjectFile,
                        &cgcx.prof,
                    )
                })?;
            }

            EmitObj::Bitcode => {
                debug!("copying bitcode {:?} to obj {:?}", bc_out, obj_out);
                if let Err(e) = link_or_copy(&bc_out, &obj_out) {
                    diag_handler.err(&format!("failed to copy bitcode to object file: {}", e));
                }

                if !config.emit_bc {
                    debug!("removing_bitcode {:?}", bc_out);
                    ensure_removed(diag_handler, &bc_out);
                }
            }

            EmitObj::None => {}
        }

        drop(handlers);
    }

    Ok(module.into_compiled_module(
        config.emit_obj != EmitObj::None,
        cgcx.target_can_use_split_dwarf
            && cgcx.split_debuginfo != SplitDebuginfo::Off
            && cgcx.split_dwarf_kind == SplitDwarfKind::Split,
        config.emit_bc,
        &cgcx.output_filenames,
    ))
}

fn create_section_with_flags_asm(section_name: &str, section_flags: &str, data: &[u8]) -> Vec<u8> {
    let mut asm = format!(".section {},\"{}\"\n", section_name, section_flags).into_bytes();
    asm.extend_from_slice(b".ascii \"");
    asm.reserve(data.len());
    for &byte in data {
        if byte == b'\\' || byte == b'"' {
            asm.push(b'\\');
            asm.push(byte);
        } else if byte < 0x20 || byte >= 0x80 {
            // Avoid non UTF-8 inline assembly. Use octal escape sequence, because it is fixed
            // width, while hex escapes will consume following characters.
            asm.push(b'\\');
            asm.push(b'0' + ((byte >> 6) & 0x7));
            asm.push(b'0' + ((byte >> 3) & 0x7));
            asm.push(b'0' + ((byte >> 0) & 0x7));
        } else {
            asm.push(byte);
        }
    }
    asm.extend_from_slice(b"\"\n");
    asm
}

/// Embed the bitcode of an LLVM module in the LLVM module itself.
///
/// This is done primarily for iOS where it appears to be standard to compile C
/// code at least with `-fembed-bitcode` which creates two sections in the
/// executable:
///
/// * __LLVM,__bitcode
/// * __LLVM,__cmdline
///
/// It appears *both* of these sections are necessary to get the linker to
/// recognize what's going on. A suitable cmdline value is taken from the
/// target spec.
///
/// Furthermore debug/O1 builds don't actually embed bitcode but rather just
/// embed an empty section.
///
/// Basically all of this is us attempting to follow in the footsteps of clang
/// on iOS. See #35968 for lots more info.
unsafe fn embed_bitcode(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    llcx: &llvm::Context,
    llmod: &llvm::Module,
    cmdline: &str,
    bitcode: &[u8],
) {
    // We're adding custom sections to the output object file, but we definitely
    // do not want these custom sections to make their way into the final linked
    // executable. The purpose of these custom sections is for tooling
    // surrounding object files to work with the LLVM IR, if necessary. For
    // example rustc's own LTO will look for LLVM IR inside of the object file
    // in these sections by default.
    //
    // To handle this is a bit different depending on the object file format
    // used by the backend, broken down into a few different categories:
    //
    // * Mach-O - this is for macOS. Inspecting the source code for the native
    //   linker here shows that the `.llvmbc` and `.llvmcmd` sections are
    //   automatically skipped by the linker. In that case there's nothing extra
    //   that we need to do here.
    //
    // * Wasm - the native LLD linker is hard-coded to skip `.llvmbc` and
    //   `.llvmcmd` sections, so there's nothing extra we need to do.
    //
    // * COFF - if we don't do anything the linker will by default copy all
    //   these sections to the output artifact, not what we want! To subvert
    //   this we want to flag the sections we inserted here as
    //   `IMAGE_SCN_LNK_REMOVE`.
    //
    // * ELF - this is very similar to COFF above. One difference is that these
    //   sections are removed from the output linked artifact when
    //   `--gc-sections` is passed, which we pass by default. If that flag isn't
    //   passed though then these sections will show up in the final output.
    //   Additionally the flag that we need to set here is `SHF_EXCLUDE`.
    //
    // Unfortunately, LLVM provides no way to set custom section flags. For ELF
    // and COFF we emit the sections using module level inline assembly for that
    // reason (see issue #90326 for historical background).
    let is_apple = cgcx.opts.target_triple.triple().contains("-ios")
        || cgcx.opts.target_triple.triple().contains("-darwin")
        || cgcx.opts.target_triple.triple().contains("-tvos");
    if is_apple
        || cgcx.opts.target_triple.triple().starts_with("wasm")
        || cgcx.opts.target_triple.triple().starts_with("asmjs")
    {
        // We don't need custom section flags, create LLVM globals.
        let llconst = common::bytes_in_context(llcx, bitcode);
        let llglobal = llvm::LLVMAddGlobal(
            llmod,
            common::val_ty(llconst),
            "rustc.embedded.module\0".as_ptr().cast(),
        );
        llvm::LLVMSetInitializer(llglobal, llconst);

        let section = if is_apple { "__LLVM,__bitcode\0" } else { ".llvmbc\0" };
        llvm::LLVMSetSection(llglobal, section.as_ptr().cast());
        llvm::LLVMRustSetLinkage(llglobal, llvm::Linkage::PrivateLinkage);
        llvm::LLVMSetGlobalConstant(llglobal, llvm::True);

        let llconst = common::bytes_in_context(llcx, cmdline.as_bytes());
        let llglobal = llvm::LLVMAddGlobal(
            llmod,
            common::val_ty(llconst),
            "rustc.embedded.cmdline\0".as_ptr().cast(),
        );
        llvm::LLVMSetInitializer(llglobal, llconst);
        let section = if is_apple { "__LLVM,__cmdline\0" } else { ".llvmcmd\0" };
        llvm::LLVMSetSection(llglobal, section.as_ptr().cast());
        llvm::LLVMRustSetLinkage(llglobal, llvm::Linkage::PrivateLinkage);
    } else {
        // We need custom section flags, so emit module-level inline assembly.
        let section_flags = if cgcx.is_pe_coff { "n" } else { "e" };
        let asm = create_section_with_flags_asm(".llvmbc", section_flags, bitcode);
        llvm::LLVMRustAppendModuleInlineAsm(llmod, asm.as_ptr().cast(), asm.len());
        let asm = create_section_with_flags_asm(".llvmcmd", section_flags, cmdline.as_bytes());
        llvm::LLVMRustAppendModuleInlineAsm(llmod, asm.as_ptr().cast(), asm.len());
    }
}

pub unsafe fn with_llvm_pmb(
    llmod: &llvm::Module,
    config: &ModuleConfig,
    opt_level: llvm::CodeGenOptLevel,
    prepare_for_thin_lto: bool,
    f: &mut dyn FnMut(&llvm::PassManagerBuilder),
) {
    use std::ptr;

    // Create the PassManagerBuilder for LLVM. We configure it with
    // reasonable defaults and prepare it to actually populate the pass
    // manager.
    let builder = llvm::LLVMPassManagerBuilderCreate();
    let opt_size = config.opt_size.map_or(llvm::CodeGenOptSizeNone, |x| to_llvm_opt_settings(x).1);
    let inline_threshold = config.inline_threshold;
    let pgo_gen_path = get_pgo_gen_path(config);
    let pgo_use_path = get_pgo_use_path(config);
    let pgo_sample_use_path = get_pgo_sample_use_path(config);

    llvm::LLVMRustConfigurePassManagerBuilder(
        builder,
        opt_level,
        config.merge_functions,
        config.vectorize_slp,
        config.vectorize_loop,
        prepare_for_thin_lto,
        pgo_gen_path.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
        pgo_use_path.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
        pgo_sample_use_path.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
    );

    llvm::LLVMPassManagerBuilderSetSizeLevel(builder, opt_size as u32);

    if opt_size != llvm::CodeGenOptSizeNone {
        llvm::LLVMPassManagerBuilderSetDisableUnrollLoops(builder, 1);
    }

    llvm::LLVMRustAddBuilderLibraryInfo(builder, llmod, config.no_builtins);

    // Here we match what clang does (kinda). For O0 we only inline
    // always-inline functions (but don't add lifetime intrinsics), at O1 we
    // inline with lifetime intrinsics, and O2+ we add an inliner with a
    // thresholds copied from clang.
    match (opt_level, opt_size, inline_threshold) {
        (.., Some(t)) => {
            llvm::LLVMPassManagerBuilderUseInlinerWithThreshold(builder, t);
        }
        (llvm::CodeGenOptLevel::Aggressive, ..) => {
            llvm::LLVMPassManagerBuilderUseInlinerWithThreshold(builder, 275);
        }
        (_, llvm::CodeGenOptSizeDefault, _) => {
            llvm::LLVMPassManagerBuilderUseInlinerWithThreshold(builder, 75);
        }
        (_, llvm::CodeGenOptSizeAggressive, _) => {
            llvm::LLVMPassManagerBuilderUseInlinerWithThreshold(builder, 25);
        }
        (llvm::CodeGenOptLevel::None, ..) => {
            llvm::LLVMRustAddAlwaysInlinePass(builder, config.emit_lifetime_markers);
        }
        (llvm::CodeGenOptLevel::Less, ..) => {
            llvm::LLVMRustAddAlwaysInlinePass(builder, config.emit_lifetime_markers);
        }
        (llvm::CodeGenOptLevel::Default, ..) => {
            llvm::LLVMPassManagerBuilderUseInlinerWithThreshold(builder, 225);
        }
    }

    f(builder);
    llvm::LLVMPassManagerBuilderDispose(builder);
}

// Create a `__imp_<symbol> = &symbol` global for every public static `symbol`.
// This is required to satisfy `dllimport` references to static data in .rlibs
// when using MSVC linker.  We do this only for data, as linker can fix up
// code references on its own.
// See #26591, #27438
fn create_msvc_imps(
    cgcx: &CodegenContext<LlvmCodegenBackend>,
    llcx: &llvm::Context,
    llmod: &llvm::Module,
) {
    if !cgcx.msvc_imps_needed {
        return;
    }
    // The x86 ABI seems to require that leading underscores are added to symbol
    // names, so we need an extra underscore on x86. There's also a leading
    // '\x01' here which disables LLVM's symbol mangling (e.g., no extra
    // underscores added in front).
    let prefix = if cgcx.target_arch == "x86" { "\x01__imp__" } else { "\x01__imp_" };

    unsafe {
        let i8p_ty = Type::i8p_llcx(llcx);
        let globals = base::iter_globals(llmod)
            .filter(|&val| {
                llvm::LLVMRustGetLinkage(val) == llvm::Linkage::ExternalLinkage
                    && llvm::LLVMIsDeclaration(val) == 0
            })
            .filter_map(|val| {
                // Exclude some symbols that we know are not Rust symbols.
                let name = llvm::get_value_name(val);
                if ignored(name) { None } else { Some((val, name)) }
            })
            .map(move |(val, name)| {
                let mut imp_name = prefix.as_bytes().to_vec();
                imp_name.extend(name);
                let imp_name = CString::new(imp_name).unwrap();
                (imp_name, val)
            })
            .collect::<Vec<_>>();

        for (imp_name, val) in globals {
            let imp = llvm::LLVMAddGlobal(llmod, i8p_ty, imp_name.as_ptr().cast());
            llvm::LLVMSetInitializer(imp, consts::ptrcast(val, i8p_ty));
            llvm::LLVMRustSetLinkage(imp, llvm::Linkage::ExternalLinkage);
        }
    }

    // Use this function to exclude certain symbols from `__imp` generation.
    fn ignored(symbol_name: &[u8]) -> bool {
        // These are symbols generated by LLVM's profiling instrumentation
        symbol_name.starts_with(b"__llvm_profile_")
    }
}

fn record_artifact_size(
    self_profiler_ref: &SelfProfilerRef,
    artifact_kind: &'static str,
    path: &Path,
) {
    // Don't stat the file if we are not going to record its size.
    if !self_profiler_ref.enabled() {
        return;
    }

    if let Some(artifact_name) = path.file_name() {
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        self_profiler_ref.artifact_size(artifact_kind, artifact_name.to_string_lossy(), file_size);
    }
}
