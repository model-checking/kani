// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains a cache of resolved generic functions

use super::{MirError, MirInstance};
use crate::kani_middle::find_fn_def;
use rustc_middle::ty::TyCtxt;
use stable_mir::ty::{GenericArgKind as GenericArg, GenericArgs};

/// FunctionSignature encapsulates the data
/// for rust functions with generic arguments
/// to ensure that it can be cached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    /// The diagnostic string associated with the function
    diagnostic: String,
    /// The generic arguments applied
    args: Vec<GenericArg>,
}

impl Signature {
    /// Create a new signature from the name and args
    pub fn new(name: &str, args: &[GenericArg]) -> Signature {
        Signature { diagnostic: name.to_string(), args: args.to_vec() }
    }
}

/// FunctionInstance encapsulates the
/// data for a resolved rust function with
/// generic arguments to ensure that it can be cached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    /// The "key" with which the function instance
    /// is created, and with which the function instance
    /// can be looked up
    signature: Signature,
    /// The "value", the resolved function instance itself
    instance: MirInstance,
}

impl Instance {
    /// Create a new cacheable instance with the given signature and
    /// instance
    pub fn new(signature: Signature, instance: MirInstance) -> Instance {
        Instance { signature, instance }
    }
}

/// Caches function instances for later lookups.
#[derive(Default, Debug)]
pub struct Cache {
    /// The cache
    cache: Vec<Instance>,
}

fn try_get_or_insert<T, P, F, E>(vec: &mut Vec<T>, p: P, f: F) -> Result<&mut T, E>
where
    F: FnOnce() -> Result<T, E>,
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
    ) -> Result<&MirInstance, MirError> {
        let test_sig = signature.clone();
        let Cache { cache } = self;
        try_get_or_insert(
            cache,
            |item| item.signature == test_sig,
            || {
                let fndef = find_fn_def(*ctx, &signature.diagnostic)
                    .ok_or(MirError::new(format!("Not found: {}", &signature.diagnostic)))?;
                let instance = MirInstance::resolve(fndef, &GenericArgs(signature.args.clone()))?;
                Ok(Instance::new(signature, instance))
            },
        )
        .map(|entry| &entry.instance)
    }
}
