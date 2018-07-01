// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides `UseTracker` as well as `UseMarkable` which is used to
//! track uses of type variables that need `Arbitrary` bounds in our
//! impls.

// Perhaps ordermap would be better, but our maps are so small that we care
// much more about the increased compile times incured by including ordermap.
// We need to preserve insertion order in any case, so HashMap is not useful.
use std::collections::BTreeMap;
use std::mem;
use std::borrow::Borrow;

use syn;

use attr;
use util;
use error::{DeriveResult, Ctx};

//==============================================================================
// API: Type variable use tracking
//==============================================================================

/// `UseTracker` tracks what type variables that have used in
/// `any_with::<Type>` or similar and thus needs an `Arbitrary<'a>`
/// bound added to them.
pub struct UseTracker {
    /// Tracks 'usage' of a type variable name.
    /// Allocation of this map will happen at once and no further
    /// allocation will happen after that. Only potential updates
    /// will happen after initial allocation.
    used_map: BTreeMap<syn::Ident, bool>,
    /// The generics that we are doing this for.
    /// This what we will modify later once we're done.
    generics: syn::Generics,
    /// If set to `true`, then `mark_used` has no effect.
    track: bool,
}

/// Models a thing that may have type variables in it that
/// can be marked as 'used' as defined by `UseTracker`.
pub trait UseMarkable {
    fn mark_uses(&self, tracker: &mut UseTracker);
}

impl UseTracker {
    /// Constructs the tracker for the given `generics`.
    pub fn new(generics: syn::Generics) -> Self {
        // Construct the map by setting all type variables as being unused
        // initially. This is the only time we will allocate for the map.
        let used_map = generics.type_params()
            .map(|v| (v.ident.clone(), false))
            .collect();
        Self { generics, used_map, track: true }
    }

    /// Stop tracking. `.mark_used` will have no effect.
    pub fn no_track(&mut self) {
        self.track = false;
    }

    /// Mark the _potential_ type variable `ty_var` as used.
    /// If the tracker does not know about the name, it is not
    /// a type variable and this call has no effect.
    pub fn mark_used(&mut self, ty_var: impl Borrow<syn::Ident>) {
        if self.track {
            self.used_map
                .get_mut(ty_var.borrow())
                .map(|used| { *used = true; });
        }
    }

    /// Adds the bound in `for_used` on used type variables and
    /// the bound in `for_not` (`if .is_some()`) on unused type variables.
    pub fn add_bounds(&mut self, ctx: Ctx,
        for_used: syn::TypeParamBound, for_not: Option<syn::TypeParamBound>)
        -> DeriveResult<()>
    {
        let mut iter = self.used_map.values().zip(self.generics.type_params_mut());
        if let Some(for_not) = for_not {
            iter.try_for_each(|(&used, tv)| {
                // Steal the attributes:
                let attrs = mem::replace(&mut tv.attrs, vec![]);
                let no_bound = attr::has_no_bound(ctx, attrs)?;

                let bound = if used && !no_bound { &for_used } else { &for_not };
                tv.bounds.push(bound.clone());
                Ok(())
            })?;
        } else {
            iter.for_each(|(&used, tv)|
                if used { tv.bounds.push(for_used.clone()) }
            )
        }
        Ok(())
    }

    /// Consumes the (potentially) modified generics that the
    /// tracker was originally constructed with and returns it.
    pub fn consume(self) -> syn::Generics {
        self.generics
    }
}

//==============================================================================
// Impls
//==============================================================================

// It would be really nice to use SYB programming here, but these are not our
// types, wherefore using scrapmetal would result in orphan impls.

impl UseMarkable for syn::Type {
    fn mark_uses(&self, ut: &mut UseTracker) {
        use syn::visit;

        visit::visit_type(&mut PathVisitor(ut), self);

        struct PathVisitor<'a>(&'a mut UseTracker);

        impl<'a, 'ast> visit::Visit<'ast> for PathVisitor<'a> {
            fn visit_macro(&mut self, _: &syn::Macro) {}

            fn visit_path(&mut self, path: &syn::Path) {
                // If path is PhantomData do not mark innards.
                if util::is_phantom_data(path) { return; }
                
                if let Some(ident) = util::extract_simple_path(path) {
                    self.0.mark_used(ident);
                }

                visit::visit_path(self, path);
            }

            // TODO: Consider BareFnTy and ParenthesizedParameterData wrt.
            // CoArbitrary.
        }
    }
}
