- **Feature Name:** Ghost State (`ghost-state`)
- **Feature Request Issue:** [#3184](https://github.com/model-checking/kani/issues/3184)
- **RFC PR:**
- **Status:** Unstable
- **Version:** 0
- **Proof-of-concept:** N/A

-------------------

## Summary

Add support to ghost state to allow users to track metadata useful for verification purpose without extra
runtime overhead.

## User Impact

TODO

## User Experience

We propose that ghost state to be implemented using a trait which is extended by Kani.

```rust
/// Trait that indicates a type has an associated ghost code of type `S`.
///
/// A type can have many multiple implementations, each will be mapped to a different ghost state.
pub trait GhostState<S: Default + GhostValue>: Sized + GhostStateExt<S> {}

/// Trait that specifies types that are safe to be used as a ghost value.
///
/// # Safety
///
/// A ghost value has to conform to the following existing restrictions:
/// - Type size is less or equal to 8 bits (this may become a compilation error in the future).
/// - The byte `0` is a valid representation of this type.
/// - A ghost state is always initialized with the byte `0`.
pub unsafe trait GhostValue: Sized {}

/// These functions are automatically implemented  
pub trait GhostStateExt<S> {
    /// Set the value of a ghost state.
    fn set_ghost_state(&self, value: S);

    /// Get the value of the ghost state.
    fn ghost_state(&self) -> S;

    /// Add a dummy function with a private structure so this trait cannot be overwritten.
    #[doc(hidden)]
    fn private(&self, _: Internal);
}
```

For example, let's say a user wants to check their union have been initialized, together with
which variant.

```rust
union Size {
    unsigned: usize,
    signed: isize,
}

/// Declare ghost state to track initialization.
impl kani::GhostState<IsInit> for Size {}

/// Declare ghost state to track which variant is used.
impl kani::GhostState<IsSigned> for Size {}

/// Declare the ghost memory type and declare that it can be used as `GhostValue`
struct IsInit(bool);

unsafe impl kani::GhostValue for IsInit {}

struct IsSigned(bool);

unsafe impl kani::GhostValue for IsSigned {}

impl Size {
    /// Specify safety contract.
    #[kani::requires(self.ghost_state::< IsInit > () && ! self.ghost_state::< IsSigned > ())]
    pub unsafe fn unsigned(&self) -> usize {
        unsafe { self.unsigned }
    }

    /// Show how ghost_state can also be used outside of contracts.
    pub fn set_unsigned(&self, val: usize) {
        self.unsigned = val;
        self.set_ghost_state::<IsInit>(true);
        self.set_ghost_state::<IsSigned>(false);
    }
}
```

One limitation today is that ghost state cannot be implemented for ZST types.
Kani compiler will generate an unsupported feature for any reachable occurrence.

## Software Design

The trait `GhostStateExt` will contain an automatic implementation for any type `T` that implements `GhostState`
that cannot be overridden.
For that, we added the `private` function, and we will include the following implementation:

```rust
impl<T, S> GhostStateExt<S> for T {
    #[rustc_diagnostic_item = "KaniSetGhost"]
    fn set_ghost_state(&self, value: S) {
        kani_intrinsic()
    }

    #[rustc_diagnostic_item = "KaniGetGhost"]
    fn ghost_state(&self) -> S {
        kani_intrinsic()
    }

    fn private(&self, _: Internal) {}
}
```

Since CBMC ghost memory supports multiple maps which are indexed by a name, we are planning to instantiate one
map per reachable GhostState implementation.
This would avoid value collision.
We could eventually simplify CBMC to avoid using the "OR" logic over the members of a structure.

### Changes to Kani compiler

We are creating two new intrinsics to Kani: `KaniSetGhost` and `KaniGetGhost`.
Those intrinsics will directly map to CBMC's

## Rationale and alternatives

The main motivation to implement this in Kani today is to allow more sophisticated safety contracts.
Not doing this will likely limit the expressiveness of UB detection to things that only Kani implements.

### User experience

We propose using the trait based implementation to allow us to implement ghost state for built-in types such as
reference, arrays and pointers.

Another approach we explored was to use a ZST structure, that kani-compiler is aware of.
The compiler would encode dereference and assignments to object of this type by mapping to the ghost memory.

Something like:

```rust
struct GhostState<T>(PhantomData<T>) where T: Default + GhostValue;

impl<T> GhostState<T> {
    fn new(val: T) -> Self { /* ... */ }
    fn set(&self, val: T) { /* ... */ }
    fn get(&self) -> T { /* ... */ }
}

struct Dummy {
    field1: u8,
    field2: u8,
    ghost_state: GhostState<bool>,
}

impl Dummy {
    fn new() -> Dummy {
        Dummy { field1: 0, field2: 0, ghost: GhostState::new(false) }
    }
}
```

Even though this approach is more ergonomic, it cannot be applied to builtin types.

A third approach would be to expose the intrinsics and let users use them freely, something like:

```rust
pub fn ghost_state<T, S: GhostState>(obj: &T) -> S {
    kani_intrinsic()
}

pub fn set_ghost_state<T, S: GhostState>(obj: &T, val: S) {
    kani_intrinsic()
}
```

The major advantage of this approach is that a crate can define a ghost state for any type,
i.e., orphan rule does not apply.

The major downside is that

1. We either restrict `T` to be sized, or it may not be clear to the user that the shadow memory for the same address
   changes with coercion. See follow-up section on why generate one shadow memory per `(T, S)` combination.
2. User has no way to check if the combination of `(T, S)` is being used anywhere else to represent a different state.

### Representing a ghost state

We propose that a ghost state is specific to each combination of type being tracked `T` and the state `S`, i.e.,
we will create a new shadow memory for each combination of the pair `(T, S)`.

First, this ensures there is one specific semantic for each shadow memory.
The semantics is clear and can be easily documented by documenting the trait implementation.

## Open questions

- This is a highly experimental feature, and it might be worth revisiting the solution once we get some user
  perspective.
- What should be the implementation of ghost state for concrete playback? One possible approach is to implement
  memory with a global hash map.
  For initial implementations, I would prefer keeping it simple and maybe just add print statements.
- Should Kani automatically propagate ghost state for copy and clone (at least for temporary variables)?
    - What about transmute?

## Out of scope / Future Improvements

