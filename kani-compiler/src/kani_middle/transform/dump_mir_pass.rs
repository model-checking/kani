// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Global transformation pass, which does not modify bodies but dumps MIR whenever the appropriate debug flag is passed.

use crate::kani_middle::reachability::CallGraph;
use crate::kani_middle::transform::GlobalPass;
use crate::kani_queries::QueryDb;
use kani_metadata::ArtifactType;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::OutputType;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::Body;
use std::fmt::{Debug, Formatter, Result};
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

/// Dump all MIR bodies.
pub struct DumpMirPass {
    enabled: bool,
}

impl DumpMirPass {
    pub fn new(tcx: TyCtxt) -> Self {
        Self { enabled: tcx.sess.opts.output_types.contains_key(&OutputType::Mir) }
    }
}

impl Debug for DumpMirPass {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("DumpMirPass").field("enabled", &self.enabled).finish()
    }
}

impl GlobalPass for DumpMirPass {
    fn is_enabled(&self, _query_db: &QueryDb) -> bool {
        self.enabled
    }

    fn transform(
        &mut self,
        tcx: TyCtxt,
        starting_items: &[MonoItem],
        bodies: Vec<(Body, Instance)>,
        _call_graph: &CallGraph,
    ) -> Vec<(bool, Body, Instance)> {
        // Create output buffer.
        let file_path = {
            let base_path = tcx.output_filenames(()).path(OutputType::Object);
            let base_name = base_path.as_path();
            let entry_point = (starting_items.len() == 1).then_some(starting_items[0].clone());
            if let Some(MonoItem::Fn(starting_instance)) = entry_point {
                let mangled_name = starting_instance.mangled_name();
                let file_stem =
                    format!("{}_{mangled_name}", base_name.file_stem().unwrap().to_str().unwrap());
                base_name.with_file_name(file_stem).with_extension(ArtifactType::SymTabGoto)
            } else {
                base_name.with_extension(ArtifactType::SymTabGoto)
            }
        };
        let out_file = File::create(file_path.with_extension("kani.mir")).unwrap();
        let mut writer = BufWriter::new(out_file);

        // For each def_id, dump their MIR
        for (body, instance) in bodies.iter() {
            writeln!(writer, "// Item: {} ({})", instance.name(), instance.mangled_name()).unwrap();
            let _ = body.dump(&mut writer, &instance.name());
        }

        bodies.into_iter().map(|(body, instance)| (false, body, instance)).collect()
    }
}
