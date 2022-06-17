// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Data structures to represent vtable trait function pointer restrictions

// TODO: We currently use `InternedString`, but possibly should only use `String`
pub use cbmc::InternedString;
use serde::{Deserialize, Serialize};

/// A "trait-defined method"  (`Trait::method`) represents the abstract function.
/// For example, `Into::into` identifies a trait and a function within this trait, but
/// does not identify a concrete function (because it is not applied to a concrete type.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraitDefinedMethod {
    /// The canonical trait name (see function `normalized_trait_name` in the Kani compiler)
    pub trait_name: InternedString,
    /// Use the index into this vtable, instead of the function name.
    pub vtable_idx: usize,
}

/// A call-site is a location in the code that invokes a particular `TraitDefinedMethod`.
/// This is identified by:
///   1. The (mangled) name of the function this code is a part of
///   2. The (unique) label we applied
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// The "Trait::method" being invoked at this location
    pub trait_method: TraitDefinedMethod,
    /// The (mangled symbol name of the) function this code is within
    pub function_name: InternedString,
    /// The unique label we applied to this function invocation.
    /// Because of how MIR works, the code being emitted here will always look like this:
    ///   `label: tmp_n = vtable->fn(tmp_1, tmp_2, ...)`
    /// This label we apply is the means by which we identify the function pointer `vtable->fn` as
    /// having only certain possible values.
    pub label: InternedString,
}

/// A set of possible targets for a vtable entry's function pointer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PossibleMethodEntry {
    /// The `Trait::method` entry we have new possibilities for.
    pub trait_method: TraitDefinedMethod,
    /// The (mangled symbol name of the) function this trait-defined method might pointer to.
    /// (This is a `Vec` purely for representation efficiency reasons. It could be a single
    /// possibility, but with more entries in `possible_method` below.)
    pub possibilities: Vec<InternedString>,
}

/// Represents the full set of vtable restrictions visible in this crate.
/// Currently corresponds to a `*.restrictions.json` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtableCtxResults {
    /// Each call site that is visible in this crate: a call site can have restrictions applied to it.
    pub call_sites: Vec<CallSite>,
    /// A set of entries to the map from `TraitDefinedMethod` to function symbol.
    /// When all of these are aggregated together from all linked crates, these collectively represent
    /// the only function pointers that might exist in this vtable entry.
    pub possible_methods: Vec<PossibleMethodEntry>,
}
