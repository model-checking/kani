// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use std::sync::{Arc, Mutex};

use crate::args::Arguments;

/// This structure should only be used behind a synchronized reference or a snapshot.
///
/// TODO: Merge this with arguments
#[derive(Debug, Default, Clone)]
pub struct QueryDb {
    args: Option<Arguments>,
}

impl QueryDb {
    pub fn new() -> Arc<Mutex<QueryDb>> {
        Arc::new(Mutex::new(QueryDb::default()))
    }

    pub fn set_args(&mut self, args: Arguments) {
        self.args = Some(args);
    }

    pub fn args(&self) -> &Arguments {
        self.args.as_ref().expect("Arguments have not been initialized")
    }
}
