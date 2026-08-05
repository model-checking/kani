// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpmc::{Receiver, Sender, channel};
use std::sync::mpsc::TryRecvError;
use std::thread::JoinHandle;

use cbmc::irep::goto_binary_serde::write_goto_binary_file;
use cbmc::{InternedString, InternerSpecific, WithInterner};
use kani_metadata::ArtifactType;

use crate::codegen_cprover_gotoc::compiler_interface::write_file;

unsafe impl InternerSpecific for FileDataToWrite {}

/// A struct that contains all the data needed to export a Goto binary.
pub(crate) struct FileDataToWrite {
    pub symtab_goto: PathBuf,
    pub symbol_table: cbmc::goto_program::SymbolTable,
    pub vtable_restrictions: Option<kani_metadata::VtableCtxResults>,
    pub type_map: BTreeMap<InternedString, InternedString>,
    pub pretty_name_map: BTreeMap<InternedString, Option<InternedString>>,
    pub pretty: bool,
}

/// A thread pool of `N` worker threads specifically for writing Goto files in parallel.
///
/// File data can be sent to the `work_queue`. This will wake a worker thread which will then serialize and write
/// it to disk in parallel, allowing the main compiler thread to continue codegen.
pub struct ThreadPool {
    pub(crate) work_queue: Sender<WorkToSend>,
    work_queue_recv: Receiver<WorkToSend>,
    join_handles: Vec<JoinHandle<WorkerReturn>>,
}

type WorkerReturn = ();

type WorkToSend = WithInterner<FileDataToWrite>;
impl ThreadPool {
    pub fn empty() -> Self {
        let (work_queue_send, work_queue_recv) = channel();
        ThreadPool { work_queue: work_queue_send, work_queue_recv, join_handles: Vec::new() }
    }

    fn new_worker(work_queue: &Receiver<WorkToSend>) -> JoinHandle<()> {
        let new_work_queue = work_queue.clone();
        std::thread::spawn(move || {
            worker_loop(new_work_queue);
        })
    }

    pub fn add_workers(&mut self, count: usize) {
        self.join_handles.extend((0..count).map(|_| Self::new_worker(&self.work_queue_recv)));
    }

    /// Try to send work to the work queue, or do it yourself if there's no worker threads.
    /// Will only fail if all recievers have disconnected.
    pub fn send_work(&self, work: WorkToSend) -> Result<(), &str> {
        // If we don't have any workers, just synchronously handle the work ourselves.
        if self.join_handles.is_empty() {
            handle_file(work.into_inner());
            return Ok(());
        }

        // Otherwise send it to the queue.
        self.work_queue.send(work).map_err(|_| "all worker threads must have disconnected")
    }

    /// Wait for all worker threads to finish and join.
    pub fn join_all(self) {
        // Since this structure maintains a reference to the work queue,
        // we have to close it so the channel will close and workers will know to exit.
        drop(self.work_queue);

        for handle in self.join_handles {
            handle.join().unwrap();
        }
    }
}

fn worker_loop(work_queue: Receiver<WithInterner<FileDataToWrite>>) -> WorkerReturn {
    while let Ok(new_work) = work_queue.recv() {
        // This call to into_inner implicitly updates our thread local interner.
        handle_file(new_work.into_inner());
    }

    // Double check that the work queue has been closed by the sender.
    debug_assert!(matches!(work_queue.try_recv(), Err(TryRecvError::Disconnected)));
}

fn handle_file(
    FileDataToWrite {
        symtab_goto,
        symbol_table,
        vtable_restrictions,
        type_map,
        pretty_name_map,
        pretty,
    }: FileDataToWrite,
) {
    write_file(&symtab_goto, ArtifactType::PrettyNameMap, &pretty_name_map, pretty);
    write_goto_binary_file(&symtab_goto, &symbol_table);
    write_file(&symtab_goto, ArtifactType::TypeMap, &type_map, pretty);
    // If they exist, write out vtable virtual call function pointer restrictions
    if let Some(restrictions) = vtable_restrictions {
        write_file(&symtab_goto, ArtifactType::VTableRestriction, &restrictions, pretty);
    }
}
