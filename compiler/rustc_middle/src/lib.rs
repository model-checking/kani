//! The "main crate" of the Rust compiler. This crate contains common
//! type definitions that are used by the other crates in the rustc
//! "family". Some prominent examples (note that each of these modules
//! has their own README with further details).
//!
//! - **HIR.** The "high-level (H) intermediate representation (IR)" is
//!   defined in the `hir` module.
//! - **MIR.** The "mid-level (M) intermediate representation (IR)" is
//!   defined in the `mir` module. This module contains only the
//!   *definition* of the MIR; the passes that transform and operate
//!   on MIR are found in `rustc_const_eval` crate.
//! - **Types.** The internal representation of types used in rustc is
//!   defined in the `ty` module. This includes the **type context**
//!   (or `tcx`), which is the central context during most of
//!   compilation, containing the interners and other things.
//!
//! For more information about how rustc works, see the [rustc dev guide].
//!
//! [rustc dev guide]: https://rustc-dev-guide.rust-lang.org/
//!
//! # Note
//!
//! This API is completely unstable and subject to change.

#![doc(html_root_url = "https://doc.rust-lang.org/nightly/nightly-rustc/")]
#![feature(allocator_api)]
#![feature(array_windows)]
#![feature(assert_matches)]
#![feature(backtrace)]
#![feature(bool_to_option)]
#![feature(box_patterns)]
#![feature(core_intrinsics)]
#![feature(derive_default_enum)]
#![feature(discriminant_kind)]
#![feature(exhaustive_patterns)]
#![feature(get_mut_unchecked)]
#![feature(if_let_guard)]
#![feature(map_first_last)]
#![feature(never_type)]
#![feature(extern_types)]
#![feature(new_uninit)]
#![feature(nll)]
#![feature(once_cell)]
#![feature(let_else)]
#![feature(min_specialization)]
#![feature(trusted_len)]
#![feature(crate_visibility_modifier)]
#![feature(associated_type_bounds)]
#![feature(rustc_attrs)]
#![feature(half_open_range_patterns)]
#![feature(control_flow_enum)]
#![feature(associated_type_defaults)]
#![feature(trusted_step)]
#![feature(try_blocks)]
#![feature(try_reserve_kind)]
#![feature(nonzero_ops)]
#![feature(unwrap_infallible)]
#![recursion_limit = "512"]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate rustc_macros;
#[macro_use]
extern crate rustc_data_structures;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate smallvec;

#[cfg(test)]
mod tests;

#[macro_use]
mod macros;

#[macro_use]
pub mod query;

#[macro_use]
pub mod arena;
#[macro_use]
pub mod dep_graph;
pub mod hir;
pub mod infer;
pub mod lint;
pub mod metadata;
pub mod middle;
pub mod mir;
pub mod thir;
pub mod traits;
pub mod ty;

pub mod util {
    pub mod bug;
    pub mod common;
}

// Allows macros to refer to this crate as `::rustc_middle`
extern crate self as rustc_middle;
