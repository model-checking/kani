// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
/// We can improve verification performance by conveying semantic information
/// about function pointer calls down to the underlying solver. In particular,
/// method calls through Rust dynamic trait objects use a vtable (virtual method
/// table) that stores method pointers for the concrete object type rather than
/// the shared abstract trait type.
///
/// This file build a map of virtual call sites to all of the possible trait
/// implementations that match that method and trait. We then can write out
/// this information as function pointer restrictions, improving verification
/// performance compared to heuristics that consider a wider set of possible
/// function pointer targets.
///
/// For the current CBMC implementation of function restrictions, see:
///     http://cprover.diffblue.com/md__home_travis_build_diffblue_cbmc_doc_architectural_restrict-function-pointer.html
use crate::codegen_cprover_gotoc::codegen::typ::pointee_type;
use cbmc::InternedString;
use cbmc::goto_program::{Stmt, Type};
use kani_metadata::{CallSite, PossibleMethodEntry, TraitDefinedMethod, VtableCtxResults};
use rustc_data_structures::fx::FxHashMap;
use tracing::debug;

/// This structure represents data about the vtable that we construct
/// Per trait, per method, which functions could virtual call sites
/// possibly refer to?
pub struct VtableCtx {
    // Option to actually enable restrictions
    pub emit_vtable_restrictions: bool,

    // Map: (normalized trait name, method index) -> possible implementations
    possible_methods: FxHashMap<TraitDefinedMethod, Vec<InternedString>>,

    // All sites where a virtual call takes place
    call_sites: Vec<CallSite>,

    // Internal tracing of index needed for call site wrappers
    call_site_global_idx: usize,
}

/// Constructor
impl VtableCtx {
    pub fn new(emit_vtable_restrictions: bool) -> Self {
        debug!("Restricting vtable function pointers? {:?}", emit_vtable_restrictions);
        Self {
            emit_vtable_restrictions,
            possible_methods: FxHashMap::default(),
            call_sites: Vec::new(),
            call_site_global_idx: 0,
        }
    }
}

/// Interface for codegen to add possible methods
impl VtableCtx {
    /// Add a possible implementation for a virtual method call.
    pub fn add_possible_method(
        &mut self,
        trait_name: InternedString,
        method: usize,
        imp: InternedString,
    ) {
        assert!(self.emit_vtable_restrictions);
        let key = TraitDefinedMethod { trait_name, vtable_idx: method };

        if let Some(possibilities) = self.possible_methods.get_mut(&key) {
            possibilities.push(imp);
        } else {
            self.possible_methods.insert(key, vec![imp]);
        }
    }

    /// The vtable index for drop
    pub fn drop_index() -> usize {
        rustc_middle::ty::COMMON_VTABLE_ENTRIES_DROPINPLACE
    }
}

/// Internal tracking helpers
impl VtableCtx {
    fn get_call_site_global_idx(&mut self) -> usize {
        assert!(self.emit_vtable_restrictions);
        self.call_site_global_idx += 1;
        self.call_site_global_idx
    }

    /// Add a given call site for a virtual function
    fn add_call_site(
        &mut self,
        trait_name: InternedString,
        method: usize,
        function_name: InternedString,
        label: InternedString,
    ) {
        assert!(self.emit_vtable_restrictions);
        let site = CallSite {
            trait_method: TraitDefinedMethod { trait_name, vtable_idx: method },
            function_name,
            label,
        };
        self.call_sites.push(site);
    }
}

impl<'tcx> GotocCtx<'tcx> {
    /// Create a label to the virtual call site
    pub fn virtual_call_with_restricted_fn_ptr(
        &mut self,
        trait_ref: Type,
        vtable_idx: usize,
        body: Stmt,
    ) -> Stmt {
        assert!(self.vtable_ctx.emit_vtable_restrictions);

        let label: InternedString =
            format!("restricted_call_label_{}", self.vtable_ctx.get_call_site_global_idx()).into();

        // We only have the Gotoc type, we need to normalize to match the MIR type.
        // Retrieve the MIR for `&dyn T` and normalize the name.
        assert!(trait_ref.is_struct_tag());
        let trait_ref_mir_type = self.type_map.get(&trait_ref.tag().unwrap()).unwrap();
        let trait_name = self.normalized_trait_name(pointee_type(*trait_ref_mir_type).unwrap());

        // Label
        self.vtable_ctx.add_call_site(
            trait_name.into(),
            vtable_idx,
            self.current_fn().name().into(),
            label,
        );
        body.with_label(label)
    }
}

/// Write out information per crate. We need to later aggregate the information
/// for the final combined executable (virtual calls can be across dependencies).
impl VtableCtx {
    /// Write out (1) all call sites and (2) possible concrete methods to JSON.
    pub fn get_virtual_function_restrictions(&mut self) -> VtableCtxResults {
        assert!(self.emit_vtable_restrictions);

        VtableCtxResults {
            call_sites: self.call_sites.clone(),
            possible_methods: self
                .possible_methods
                .drain()
                .map(|(k, v)| PossibleMethodEntry { trait_method: k, possibilities: v })
                .collect(),
        }
    }
}
