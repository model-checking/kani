// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Data structures to represent vtable trait function pointer restrictions
use cbmc::InternedString;
use serde::{Deserialize, Serialize};

/// Trait-defined method: the trait type and the vtable index of the method.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraitDefinedMethod {
    // Needs to be a string to handle both the MIR and Gotoc types
    pub trait_name: InternedString,
    pub vtable_idx: usize,
}

// Call sites are identified by the trait-defined method and the function location
// - we use a wrapper function to unambigiously point to call site locations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallSite {
    pub trait_method: TraitDefinedMethod,
    pub function_name: InternedString,
    pub label: InternedString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PossibleMethodEntry {
    pub trait_method: TraitDefinedMethod,
    pub possibilities: Vec<InternedString>,
}

// The full result of vtable restriction analysis is a list of call sides and a map
// of trait defined methods to list of possible trait method targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtableCtxResults {
    pub call_sites: Vec<CallSite>,
    pub possible_methods: Vec<PossibleMethodEntry>,
}
