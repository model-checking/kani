// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use crate::args::Arguments;
use crate::kani_middle::kani_functions::{
    KaniFunction, find_kani_functions, validate_kani_functions,
};
use stable_mir::ty::FnDef;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// This structure should only be used behind a synchronized reference or a snapshot.
///
/// TODO: Merge this with arguments
#[derive(Debug, Default, Clone)]
pub struct QueryDb {
    args: Option<Arguments>,
    kani_functions: OnceCell<HashMap<KaniFunction, FnDef>>,
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

    /// Return a map with model and intrinsic functions defined in Kani's library.
    ///
    /// For `kani_core`, those definitions live in the `core` library.
    ///
    /// We cache these definitions to avoid doing the lookup every time it is needed.
    /// The cache should not be used after the `stable_mir` context ends.
    /// For example, in the goto backend, we run the entire crate codegen under the same StableMIR
    /// context, which is defined by the scope of the StableMIR `run` callback.
    /// See the `codegen_crate` function in [crate::codegen_cprover_gotoc::compiler_interface].
    /// It is OK to set the cache and use it inside the callback scope, however, the cache should
    /// not be accessible after that.
    ///
    /// For that, users should create a new QueryDb that does not outlive the callback scope.
    ///
    /// To ensure we don't accidentally use the cache outside of the callback context, we run a
    /// sanity check if we are reusing the cache.
    pub fn kani_functions(&self) -> &HashMap<KaniFunction, FnDef> {
        if let Some(kani_functions) = self.kani_functions.get() {
            // Sanity check the values stored to ensure the cache is being within the StableMIR
            // context used to populate the cache.
            validate_kani_functions(kani_functions);
            kani_functions
        } else {
            self.kani_functions.get_or_init(|| {
                // Find all kani functions
                find_kani_functions()
            })
        }
    }
}
