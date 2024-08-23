use super::{MirInstance, MirError};
use rustc_middle::ty::TyCtxt;
use stable_mir::ty::{GenericArgKind as GenericArg, GenericArgs};
use crate::kani_middle::find_fn_def;

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

impl Cache {
    /// Register the signature the to the cache
    /// in the given compilation context, ctx
    pub fn register(&mut self, ctx: &TyCtxt, sig: Signature) ->
        Result<&MirInstance, MirError> {
        let Cache { cache } = self;
        for i in 0..cache.len() {
            if sig == cache[i].signature {
                return Ok(&cache[i].instance);
            }
        }
        let fndef = find_fn_def(*ctx, &sig.diagnostic)
            .ok_or(MirError::new(format!("Not found: {}", &sig.diagnostic)))?;
        let instance = MirInstance::resolve(fndef, &GenericArgs(sig.args.clone()))?;
        cache.push(Instance::new(sig, instance));
        Ok(&cache[cache.len() - 1].instance)
    }

    /// Register the kani assertion function
    pub fn register_assert(&mut self, ctx: &TyCtxt) ->
        Result<&MirInstance, MirError> {
        let diagnostic = "KaniAssert".to_string();
        let args = vec![];
        let sig = Signature { diagnostic, args };
        let Cache { cache } = self;
        for i in 0..cache.len() {
            if sig == cache[i].signature {
                return Ok(&cache[i].instance);
            }
        }
        let fndef = find_fn_def(*ctx, &sig.diagnostic)
            .ok_or(MirError::new(format!("Not found: {}", &sig.diagnostic)))?;
        let instance = super::MirInstance::resolve(fndef, &GenericArgs(sig.args.clone()))?;
        cache.push(Instance::new(sig, instance));
        Ok(&cache[cache.len() - 1].instance)
    }
}
