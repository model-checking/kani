// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Compiler interface for Strata codegen backend

use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_middle::dep_graph::{WorkProduct, WorkProductId};
use rustc_middle::ty::TyCtxt;
use rustc_middle::util::Providers;
use rustc_session::Session;
use rustc_session::config::OutputFilenames;
use rustc_span::ErrorGuaranteed;
use std::any::Any;
use std::fs;
use std::path::Path;

use crate::codegen_strata::mir_to_strata::MirToStrata;

pub struct StrataCodegenBackend;

impl CodegenBackend for StrataCodegenBackend {
    fn locale_resource(&self) -> &'static str {
        ""
    }

    fn provide(&self, _providers: &mut Providers) {}

    fn codegen_crate<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        _metadata: rustc_metadata::EncodedMetadata,
        _need_metadata_module: bool,
    ) -> Box<dyn Any> {
        let mut translator = MirToStrata::new(tcx);

        // Translate all functions
        for def_id in tcx.mir_keys(()) {
            let body = tcx.optimized_mir(def_id);
            let fn_name = tcx.def_path_str(def_id);
            translator.translate_body(body, &fn_name);
        }

        let strata_ir = translator.finish();

        Box::new(StrataCodegenResults { strata_ir })
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        _sess: &Session,
        _outputs: &OutputFilenames,
    ) -> (Result<(), ErrorGuaranteed>, rustc_metadata::EncodedMetadata) {
        let results = ongoing_codegen.downcast::<StrataCodegenResults>().unwrap();

        // Write Strata IR to file
        let output_path = Path::new("output.core.st");
        fs::write(output_path, &results.strata_ir).expect("Failed to write Strata IR");

        println!("Strata IR written to: {}", output_path.display());

        (Ok(()), rustc_metadata::EncodedMetadata::new())
    }

    fn link(
        &self,
        _sess: &Session,
        _codegen_results: Box<dyn Any>,
        _outputs: &OutputFilenames,
    ) -> Result<(), ErrorGuaranteed> {
        Ok(())
    }
}

struct StrataCodegenResults {
    strata_ir: String,
}
