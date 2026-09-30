// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate clap;

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::PathBuf,
};
use strum_macros::{Display, EnumString};

pub use artifact::ArtifactType;
pub use cbmc_solver::CbmcSolver;
pub use harness::*;
pub use vtable::*;

pub mod artifact;
mod cbmc_solver;
mod harness;
pub mod unstable;
mod vtable;

pub use unstable::{EnabledUnstableFeatures, UnstableFeature};

/// The default maximum length for nondeterministic slices that automatic harnesses generate.
/// Verification results for functions taking `&[T]`/`&mut [T]` arguments are only valid up to
/// this bound. The default must stay below Kani's default unwinding bound (20), so that loops
/// iterating over such a slice can be fully unwound by default.
pub const AUTOHARNESS_SLICE_BOUND: u64 = 16;

/// The default maximum length (in bytes) for nondeterministic strings that automatic harnesses
/// generate. Strings use a much smaller bound than slices: the generated string is the longest
/// valid-UTF-8 prefix of nondeterministic bytes, and reasoning about UTF-8 validity is
/// expensive. On top of that, a harness that decodes every `char` (e.g. `s.chars().count()`)
/// unwinds the decoding loop up to the default bound (20) over symbolic bytes, so the cost grows
/// steeply with the number of bytes: on a typical machine 8 bytes already exceed Kani's default
/// 60s harness timeout for such harnesses, while 4 stay comfortably within it (though a
/// char-decoding harness can still approach the timeout on slow machines, so callers that must
/// not time out should raise `--harness-timeout`).
pub const AUTOHARNESS_STR_BOUND: u64 = 4;

/// The default bound for nondeterministic values of types that implement `BoundedArbitrary`
/// (rather than `Arbitrary`) that automatic harnesses generate, e.g. `Vec<T>` or `String`.
/// Verification results for functions with such arguments are only valid up to this bound.
/// This is smaller than the slice/str bounds since `BoundedArbitrary` values are heap
/// allocated, and for `String` additionally involve UTF-8 reasoning; a bound of 8 already
/// makes simple `String` harnesses exceed Kani's default 60s harness timeout.
pub const AUTOHARNESS_BOUNDED_ARBITRARY_BOUND: u64 = 4;

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaniMetadata {
    /// The crate name from which this metadata was extracted.
    pub crate_name: String,
    /// The proof harnesses (`#[kani::proof]`) found in this crate.
    pub proof_harnesses: Vec<HarnessMetadata>,
    /// The features found in this crate that Kani does not support.
    /// (These general translate to `assert(false)` so we can still attempt verification.)
    pub unsupported_features: Vec<UnsupportedFeature>,
    /// If crates are built in test-mode, then test harnesses will be recorded here.
    pub test_harnesses: Vec<HarnessMetadata>,
    /// The functions with contracts in this crate
    pub contracted_functions: Vec<ContractedFunction>,
    /// Metadata for the `autoharness` subcommand
    pub autoharness_md: Option<AutoHarnessMetadata>,
}

/// For the autoharness subcommand, all of the user-defined functions we found,
/// which are "chosen" if we generated an automatic harness for them, and "skipped" otherwise.
/// We use ordered data structures so that the metadata is in alphabetical order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoHarnessMetadata {
    /// Functions we generated automatic harnesses for.
    pub chosen: BTreeSet<String>,
    /// Map function names to the reason why we did not generate an automatic harness for that function.
    pub skipped: BTreeMap<String, AutoHarnessSkipReason>,
}

/// Reasons that Kani does not generate an automatic harness for a function.
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumString)]
pub enum AutoHarnessSkipReason {
    /// The function is generic and autoharness could not find a monomorphic instantiation to
    /// verify. The payload gives the specific reason (e.g. const generic parameters, or trait
    /// bounds that no candidate type satisfies).
    #[strum(serialize = "Generic Function")]
    GenericFn(String),
    /// A Kani-internal function: already a harness, implementation of a Kani associated item or Kani contract instrumentation functions).
    #[strum(serialize = "Kani implementation")]
    KaniImpl,
    /// At least one of the function's arguments does not implement kani::Arbitrary
    /// (The Vec<(String, String)> contains the list of (name, type) tuples for each argument that does not implement it
    #[strum(serialize = "Missing Arbitrary implementation for argument(s)")]
    MissingArbitraryImpl(Vec<(String, String)>),
    /// The function does not have a body.
    #[strum(serialize = "The function does not have a body")]
    NoBody,
    /// The function's arguments are only supported with bounded nondeterministic values, and
    /// the user did not pass --bounded-arguments.
    /// (The Vec<(String, String)> contains the list of (name, type) tuples for each such argument.)
    #[strum(serialize = "Requires --bounded-arguments for argument(s)")]
    RequiresBoundedArguments(Vec<(String, String)>),
    /// The function doesn't match the user's provided filters.
    #[strum(serialize = "Did not match provided filters")]
    UserFilter,
}
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
pub struct ContractedFunction {
    /// The fully qualified name the user gave to the function (i.e. includes the module path).
    pub function: String,
    /// The (currently full-) path to the file this function was declared within.
    pub file: String,
    /// The pretty names of the proof harnesses (`#[kani::proof_for_contract]`) for this function
    pub harnesses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedFeature {
    // We could replace this with an enum: https://github.com/model-checking/kani/issues/1765
    /// A string identifying the feature.
    pub feature: String,
    /// A list of locations (file, line) where this unsupported feature can be found.
    pub locations: HashSet<Location>,
}

/// The location in a file
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Location {
    pub filename: String,
    pub start_line: u64,
}

/// We stub artifacts with the path to a KaniMetadata file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerArtifactStub {
    pub metadata_path: PathBuf,
}
