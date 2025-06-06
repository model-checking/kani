// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Unified handling of unstable-feature flags across Kani.
//!
//! The central types are [`UnstableFeature`], which describes a single feature
//! and is intended as a small, cheap enum that is [`Copy`].
//! [`EnabledUnstableFeatures`] then describes which [`UnstableFeature`]s are
//! enabled.
//!
//! To check if a feature is enabled use [`EnabledUnstableFeatures::contains`].
//!
//! ### Parsing
//!
//! [`EnabledUnstableFeatures`] is intended to be used with the [`clap`]
//! "derive" API. You can directly drop it into a command line arguments struct
//! like so:
//!
//! ```
//! # use kani_metadata::unstable::*;
//! use clap::Parser;
//!
//! #[derive(Parser)]
//! struct MyCmdArgs {
//!     // ...
//!     #[clap(flatten)]
//!     unstable: EnabledUnstableFeatures,
//! }
//! ```
//!
//! Which will add the long form `--unstable feature-name` and short form `-Z
//! feature-name` options to your argument parser.
//!
//! **Note:** [`clap`] internally uses a unique name (string) to refer to each
//! argument or group, which is usually derived from the field name.
//! [`EnabledUnstableFeatures`] uses the internal name
//! `"enabled_unstable_features"` which may therefore not be used (as a field
//! name) in the embedding argument struct, e.g. `MyCmdArgs`.
//!
//! ### Reusing
//!
//! You can turn an [`UnstableFeature`] back into its command line
//! representation. This should be done with
//! [`EnabledUnstableFeatures::as_arguments`], which returns an iterator that,
//! when passed to e.g. [`std::process::Command::args`] will enable those
//! features in the subsequent call.
//!
//! You can also serialize a single feature with
//! [`UnstableFeature::as_argument`].
//!
//! Both of these methods return values that are ready for direct usage on the
//! command line, e.g one or more `-Z feature-name`. If you need only the
//! serialized name of the feature use [`UnstableFeature::as_ref`].

/// A single unstable feature. This is where you should register your own if you
/// are adding new ones, e.g. as part of the RFC process.
///
/// For usage see the [module level documentation][self].
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    clap::ValueEnum,
    strum_macros::Display,
    strum_macros::AsRefStr
)]
#[strum(serialize_all = "kebab-case")]
pub enum UnstableFeature {
    /// Enable Kani's unstable async library.
    AsyncLib,
    /// Enable the autoharness subcommand.
    Autoharness,
    /// Enable concrete playback flow.
    ConcretePlayback,
    /// Allow Kani to link against C code.
    CFfi,
    /// Kani APIs related to floating-point operations (e.g. `float_to_int_in_range`)
    FloatLib,
    /// Enable function contracts [RFC 9](https://model-checking.github.io/kani/rfc/rfcs/0009-function-contracts.html)
    FunctionContracts,
    /// Generate a C-like file equivalent to input program used for debugging purpose.
    GenC,
    /// Ghost state and shadow memory APIs.
    GhostState,
    /// Enabled Lean backend (Aeneas/LLBC)
    Lean,
    /// The list subcommand [RFC 13](https://model-checking.github.io/kani/rfc/rfcs/0013-list.html)
    List,
    /// Enable loop contracts [RFC 12](https://model-checking.github.io/kani/rfc/rfcs/0012-loop-contracts.html)
    LoopContracts,
    /// Memory predicate APIs.
    MemPredicates,
    /// Enable vtable restriction.
    RestrictVtable,
    /// Enable source-based code coverage workflow.
    /// See [RFC-0011](https://model-checking.github.io/kani/rfc/rfcs/0011-source-coverage.html)
    SourceCoverage,
    /// Allow replacing certain items with stubs (mocks).
    /// See [RFC-0002](https://model-checking.github.io/kani/rfc/rfcs/0002-function-stubbing.html)
    Stubbing,
    /// Enable quantifiers [RFC 10](https://model-checking.github.io/kani/rfc/rfcs/0010-quantifiers.html)
    Quantifiers,
    /// Automatically check that uninitialized memory is not used.
    UninitChecks,
    /// Enable an unstable option or subcommand.
    UnstableOptions,
    /// Automatically check that no invalid value is produced which is considered UB in Rust.
    /// Note that this does not include checking uninitialized value.
    ValidValueChecks,
}

impl UnstableFeature {
    /// Serialize this feature into a format in which it can be passed on the
    /// command line. Note that this already includes the `-Z` prefix, if you
    /// require only the serialized feature name use [`Self::as_ref`].
    pub fn as_argument(&self) -> [&str; 2] {
        ["-Z", self.as_ref()]
    }

    /// Serialize this feature into a format ideal for error messages.
    pub fn as_argument_string(&self) -> String {
        self.as_argument().join(" ")
    }

    /// If this unstable feature has been stabilized, return the version it was stabilized in.
    /// Use this function to produce warnings that the unstable flag is no longer necessary.
    pub fn stabilization_version(&self) -> Option<String> {
        match self {
            UnstableFeature::List => Some("0.63.0".to_string()),
            _ => None,
        }
    }
}

/// An opaque collection of unstable features that is enabled on this session.
/// Used to unify handling of unstable command line arguments across the
/// compiler and the driver.
///
/// For usage see the [module level documentation][self].
#[derive(clap::Args, Debug)]
pub struct EnabledUnstableFeatures {
    #[clap(short = 'Z', long = "unstable", num_args(1), value_name = "UNSTABLE_FEATURE")]
    enabled_unstable_features: Vec<UnstableFeature>,
}

impl EnabledUnstableFeatures {
    /// The preferred way to serialize these unstable features back into a
    /// format that can be used as command line arguments fo an invocation of
    /// e.g. the compiler.
    ///
    /// See also the [module level documentation][self].
    pub fn as_arguments(&self) -> impl Iterator<Item = &str> {
        self.enabled_unstable_features.iter().flat_map(|f| f.as_argument())
    }

    /// Is this feature enabled?
    pub fn contains(&self, feature: UnstableFeature) -> bool {
        self.enabled_unstable_features.contains(&feature)
    }

    pub fn iter(&self) -> impl Iterator<Item = &UnstableFeature> {
        self.enabled_unstable_features.iter()
    }

    /// Enable an additional unstable feature.
    /// Note that this enables an unstable feature that the user did not pass on the command line, so this function should be called with caution.
    /// At time of writing, the only use is to enable -Z function-contracts and -Z loop-contracts when the autoharness subcommand is running.
    pub fn enable_feature(&mut self, feature: UnstableFeature) {
        if !self.contains(feature) {
            self.enabled_unstable_features.push(feature);
        }
    }
}
