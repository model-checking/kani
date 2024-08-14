pub use stable_mir::ty::GenericArgKind as GenericArg;
pub use stable_mir::ty::GenericArgs;
use super::{MirInstance, MirError, TyCtxt, super::super::find_fn_def};
/// FunctionSignature encapsulates the data
/// for rust functions with generic arguments
/// to ensure that it can be cached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    name: String,
    args: Vec<GenericArg>,
}

impl Signature {
    pub fn new(name: &str, args: &[GenericArg]) -> Signature {
        Signature { name: name.to_string(), args: args.to_vec() }
    }
}

/// FunctionInstance encapsulates the
/// data for a resolved rust function with
/// generic arguments to ensure that it can be cached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    signature: Signature,
    instance: MirInstance,
}

impl Instance {
    pub fn new(signature: Signature, instance: super::MirInstance) -> Instance {
        Instance { signature, instance }
    }
}

/// Caches function instances for later lookups.
#[derive(Default, Debug)]
pub struct Cache {
    cache: Vec<Instance>,
}

impl Cache {
    pub fn register(&mut self, ctx: &TyCtxt, sig: Signature) ->
        Result<&MirInstance, MirError> {
        let Cache { cache } = self;
        for i in 0..cache.len() {
            if sig == cache[i].signature {
                return Ok(&cache[i].instance);
            }
        }
        let fndef = find_fn_def(*ctx, &sig.name)
            .ok_or(MirError::new(format!("Not found: {}", &sig.name)))?;
        let instance = super::MirInstance::resolve(fndef, &GenericArgs(sig.args.clone()))?;
        cache.push(Instance::new(sig, instance));
        Ok(&cache[cache.len() - 1].instance)
    }

    #[allow(unused)]
    fn get(&self, sig: &Signature) -> Result<&MirInstance, MirError> {
        let Cache { cache } = self;
        for Instance { signature, instance } in cache {
            if *sig == *signature {
                return Ok(instance);
            }
        }
        Err(MirError::new(format!("Not found: {:?}", sig)))
    }
}
