// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use rustc_middle::ty::TyCtxt;

use crate::clean;
use crate::config::RenderOptions;
use crate::error::Error;
use crate::formats::cache::Cache;

/// Allows for different backends to rustdoc to be used with the `run_format()` function. Each
/// backend renderer has hooks for initialization, documenting an item, entering and exiting a
/// module, and cleanup/finalizing output.
crate trait FormatRenderer<'tcx>: Sized {
    /// Gives a description of the renderer. Used for performance profiling.
    fn descr() -> &'static str;

    /// Whether to call `item` recursivly for modules
    ///
    /// This is true for html, and false for json. See #80664
    const RUN_ON_MODULE: bool;

    /// Sets up any state required for the renderer. When this is called the cache has already been
    /// populated.
    fn init(
        krate: clean::Crate,
        options: RenderOptions,
        cache: Cache,
        tcx: TyCtxt<'tcx>,
    ) -> Result<(Self, clean::Crate), Error>;

    /// Make a new renderer to render a child of the item currently being rendered.
    fn make_child_renderer(&self) -> Self;

    /// Renders a single non-module item. This means no recursive sub-item rendering is required.
    fn item(&mut self, item: clean::Item) -> Result<(), Error>;

    /// Renders a module (should not handle recursing into children).
    fn mod_item_in(&mut self, item: &clean::Item) -> Result<(), Error>;

    /// Runs after recursively rendering all sub-items of a module.
    fn mod_item_out(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Post processing hook for cleanup and dumping output to files.
    fn after_krate(&mut self) -> Result<(), Error>;

    fn cache(&self) -> &Cache;
}
