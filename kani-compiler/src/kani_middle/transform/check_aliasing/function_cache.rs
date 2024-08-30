// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains a cache of resolved generic functions

use crate::kani_middle::find_fn_def;
use rustc_middle::ty::TyCtxt;

type Result<T> = std::result::Result<T, super::MirError>;


/// Caches function instances for later lookups.
#[derive(Default, Debug)]
pub struct Cache {
    /// The cache
    cache: Vec<NamedFnDef>,
}

fn try_get_or_insert<T, P, F>(vec: &mut Vec<T>, p: P, f: F) -> Result<&mut T>
where
    F: FnOnce() -> Result<T>,
    P: Fn(&T) -> bool,
    T: PartialEq,
{
    if let Some(i) = vec.iter().position(p) {
        Ok(&mut vec[i])
    } else {
        vec.push(f()?);
        Ok(vec.last_mut().unwrap())
    }
}

impl Cache {
    /// Register the signature the to the cache
    /// in the given compilation context, ctx
    pub fn register(
        &mut self,
        ctx: &TyCtxt,
        signature: Signature,
    ) -> Result<&> {
        let test_sig = signature.clone();
        let Cache { cache } = self;
        try_get_or_insert(
            cache,
            |item| item.signature == test_sig,
            || {
                let fndef = find_fn_def(*ctx, &signature.diagnostic)
                    .ok_or(MirError::new(format!("Not found: {}", &signature.diagnostic)))?;
                let instance = MirInstance::resolve(fndef, &GenericArgs(signature.args.clone()))?;
                Ok(NamedFnDef::new(signature, instance))
            },
        )
        .map(|entry| &entry.instance)
    }
}
