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
    /// A `#[rustc_comptime]` function, which rustc only lets const items, statics and const
    /// blocks call; a harness calling it would not be a program rustc accepts.
    #[strum(serialize = "Can only be called at compile time")]
    Comptime,
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

/// Strip a crate's name from an absolute item path, restoring the crate-relative
/// form Kani uses in `kani list`, harness names, and stub targets. Since
/// rust-lang/rust#149401, `def_path_str` prefixes the local crate name at
/// *every* local path component, so it appears not just at the start
/// (`my_crate::f`) but also inside qualifiers and generic args
/// (`<my_crate::T as my_crate::Tr>::m`, `f::<my_crate::T>`). Remove `<crate>::`
/// at each path-component boundary (start of string or after a non-identifier
/// char) so these become `f`, `<T as Tr>::m`, `f::<T>`. Paths of other crates
/// are unaffected.
///
/// Every producer of item names that a consumer joins by name (the compiler's
/// metadata and the `scanner` tool's CSVs) must go through this one function,
/// so the two never disagree.
pub fn strip_crate_prefix(name: &str, krate: &str) -> String {
    let needle = format!("{krate}::");
    if !name.contains(&needle) {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    // The previous char in the input, used to decide whether a `<crate>::` here
    // is a crate-root *qualifier* (droppable) or a path *continuation* segment
    // (a module/item that happens to share the crate's name, which must be
    // kept). A qualifier appears at the start or after a type/path delimiter
    // (`<`, `,`, ` `, `&`, `*`, `(`, `[`, ...); a continuation appears after
    // `::`. So drop `<crate>::` only when the previous char is neither part of
    // an identifier nor `:`. After dropping, pretend the previous char is `:`
    // so an immediately following same-named segment is treated as a
    // continuation (e.g. `main::main::{closure#0}` -> `main::{closure#0}`).
    let mut prev: Option<char> = None;
    loop {
        let at_qualifier = match prev {
            None => true,
            Some(c) => !c.is_alphanumeric() && c != '_' && c != ':',
        };
        if at_qualifier && rest.starts_with(&needle) {
            rest = &rest[needle.len()..];
            prev = Some(':');
            continue;
        }
        let Some(ch) = rest.chars().next() else { break };
        out.push(ch);
        prev = Some(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_crate_prefix;

    /// The prefix is dropped only at a path-component boundary, including inside a
    /// `<T as Trait>` qualifier; a segment that merely shares the crate's name stays.
    #[test]
    fn strips_crate_root_qualifiers_only() {
        let cases = [
            (
                "core",
                "<char as core::ascii::AsciiExt>::is_ascii",
                "<char as ascii::AsciiExt>::is_ascii",
            ),
            ("core", "core::core_simd::vector::Simd", "core_simd::vector::Simd"),
            ("core", "ptr::align_offset", "ptr::align_offset"),
            ("core", "alloc::vec::Vec", "alloc::vec::Vec"),
            ("my_crate", "<my_crate::T as my_crate::Tr>::m", "<T as Tr>::m"),
            ("my_crate", "f::<my_crate::T>", "f::<T>"),
        ];
        for (krate, name, expected) in cases {
            assert_eq!(strip_crate_prefix(name, krate), expected, "{name}");
        }
    }

    /// Not idempotent, by design: a second application would eat a continuation
    /// segment. Callers must apply it exactly once, to a raw `def_path_str`.
    #[test]
    fn strips_one_qualifier_per_boundary() {
        assert_eq!(strip_crate_prefix("main::main::{closure#0}", "main"), "main::{closure#0}");
        assert_eq!(strip_crate_prefix("core::core::foo", "core"), "core::foo");
    }
}
