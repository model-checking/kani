- **Feature Name:** Function and method stubbing (`function_stubbing`)
- **Feature Request Issue:** [model-checking#1695](https://github.com/model-checking/kani/issues/1695)
- **RFC PR:** *Link to original PR*
- **Status:** Under Review
- **Version:** 0

## Summary

This will feature will allow users to specify that certain functions and methods should be replaced with mock functions (stubs) during verification.

## User Impact

We anticipate that stubbing will have a substantial positive impact on the usability of Kani. There are two main motivations for stubbing:

1. Users might need to stub functions/methods containing features that Kani does not support, such as inline assembly.
2. Users might need to stub functions/methods containing code that Kani supports in principle, but which in practice leads to bad verification performance.

In both cases, stubbing would enable users to verify code that cannot currently be verified by Kani (or at least not within a reasonable resource bound).

### A Simple Example

**Categorize this example somehow; like why does it currently fail?**

Consider verifying the following assertion (which can fail):

```rust
assert!(rand::random::<u32>() != 0);
```

Currently, running Kani on this leads to 1503 checks: one is a failure because of a missing definition, and the other 1502 are undetermined. That is, Kani is currently unable to prove that this assertion can fail. Verification time is 0.52 seconds.

Using stubbing, we can specify that the function `rand::random` should be replaced with the function `mock_random`, which we can define as

```rust
#[cfg(kani)]
fn mock_random<T: kani::Arbitrary>() -> T {
    kani::any()
}
```

Under this substitution, Kani has a single check, which proves that the assertion can fail. Verification time is 0.02 seconds.

### Mocking IO

- `std::fs::read`
- `std::fs::write`
- `std::fs::File`

### A Real-World Example

**TODO; hopefully something from Tokio**

## User Experience

This feature is currently limited to stubbing functions and methods.
We anticipate that the user experience we propose here could also be used when stubbing types, although the underlying technical approach might have to change.

Stubs will be specified per harness; that is, different harnesses can use different stubs (the reasoning being that users might want to mock different behavior for different harnesses).
Users will specify stubs by attaching the `#[kani::stub_by(<original>, <replacement>)]` attribute to each harness function.
The attribute may be specified multiple times per harness, so that multiple (non-conflicting) stub pairings are supported.
The arguments `original` and `replacement` give the names of functions, relative to the crate of the harness (*not* relative to the module of the harness).

For example, this code specifies that the function `mock_random` should be used in place of the function `rand::random` and the function `my_mod::foo` should be used in place of the function `my_mod::bar` for the harness `my_mod::my_harness`:

```rust
#[cfg(kani)]
fn mock_random<T: kani::Arbitrary>() -> T {
    kani::any()
}

mod my_mod {

    fn foo(x: u32) -> u32 { ... }

    fn bar(x: u32) -> u32 { ... }

    #[cfg(kani)]
    #[kani::proof]
    #[kani::stub_by(rand::random, mock_random)]
    #[kani::stub_by(my_mod::foo, my_mod::bar)]
    fn my_harness() { ... }

}
```

### Stub Sets

As a convenience, users will also be able to specify sets of stubs that can be applied to multiple harnesses.
First, users write a "dummy" function with the name of the stub set, annotate it with the `#[kani::stub_set]` attribute, and add the desired stub pairings as further attributes:

```rust
#[cfg(kani)]
#[kani::stub_set]
#[kani::stub_by(std::fs::read, my_read)]
#[kani::stub_by(std::fs::write, my_write)]
fn my_io_stubs() {}
```

When declaring a harness, users can use the `#[kani::use_stub_set(<stub_set_name>)]` attribute to apply the stub set:

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::use_stub_set(my_io_stubs)]
fn my_harness() { ... }
```

The same mechanism can be used to union together stub sets:

```rust
#[cfg(kani)]
#[kani::stub_set]
#[kani::use_stub_set(my_io_stubs)]
#[kani::use_stub_set(other_stub_set)]
fn all_my_stubs() {}
```

### Error Conditions

Given a set of (`original`, `replacement`) pairs, Kani will exit with an error if

1. a specified `replacement` stub does not exist;
2. the user specifies conflicting stubs for the same harness (i.e., if the same `original` function is mapped to multiple `replacement` functions); or
3. the signature of the `replacement` function is not compatible with the signature of the `original` function.

### Pedagogy

To teach this feature, we will update the documentation with a section on function stubbing, including simple examples showing how stubbing can help Kani handle code that currently cannot be verified.

## Detailed Design

**Update: reduce the scope of this to avoid substantial changes to `kani-driver`**

This feature will require substantial changes both to `kani-driver` and `kani-compiler`.

`kani-driver` will need to be responsible for determining which stubs are required for each harness, which will require moving the code for identifying harnesses from `kani-compiler` to `kani-driver`.
After `kani-driver` has determined the set of stubs to use for each harness, it will invoke `kani-compiler` once for each set of stubs, passing the relevant stub pairings as command line arguments.
The output of `kani-compiler` will be specialized to that stub set.
After `kani-compiler` has finished compiling under a given stub set, `kani-driver` will run the harnesses that use that stub set.

`kani-compiler` will be extended with a command line option specifying stub pairings, and a new MIR-to-MIR transformation that replaces the bodies of specified functions with their replacements.
This can be achieved via `rustc`'s query mechanism: if the user wants to replace `foo` with `bar`, then when the compiler requests the MIR for `foo`, we instead return the MIR for `bar`.
`kani-compiler` will be responsible for checking for the error conditions enumerated in the previous section.

## Rationale and alternatives

**TODO**: Emphasize more the ability to stub code that the user does not have source access to.

The lack of stubbing has a substantial negative impact on the usability of Kani: stubbing is a *de facto* necessity for verification tools.

### Benefits

- Because stubs are specified by annotating the harness, the user is able to specify stubs for functions they do not have source access to (like library functions).
This contrasts with annotating the function to be replaced (such as with function contracts).
- The current design provides the user with flexibility, as they can specify different sets of stubs to use for different harnesses.
- The stub mappings are all located right by the harness, which makes it easy to understand which replacements are going to happen for each harness.

### Risks

- Allowing per-harness stubs complicates the architecture of Kani, as (according to the current design) it requires `kani-driver` to call `kani-compiler` multiple times.
If stubs were uniformly applied, then we could get away with a single call to `kani-compiler`.

### Comparison to function contracts

- In many cases, stubs are more user-friendly than contracts. With contracts, it is necessary to explicitly provide information that is automatically captured in Rust (such as which memory is written).
- The currently proposed function contract mechanism does not provide a way to put contracts on external functions. **[CHECK]**

### Alternative #1: Annotate stubbed functions

In this alternative, users add an attribute `#[kani::stub_by(<replacement>)]` to the function that should be replaced.
This approach is similar to annotating a function with a contract specifying its behavior (the stub acts like a programmatic contract).
The major downside with this approach is that it would not be possible to stub external code. We see this as a likely use case that needs to be supported: users will want to replace `std` library functions or functions from arbitrary external crates.

### Alternative #2: Annotate stubs

In this alternative, users add an attribute `#[kani::stub(<original>)]` to the stub function itself, saying which function it replaces:

```rust
#[cfg(kani)]
#[kani::stub(rand::random)]
fn mock_random<T: kani::Arbitrary>() -> T { ... }
```

The downside is that this stub must be uniformly applied across all harnesses and the stub specifications might be spread out across multiple files.
It would also require an extra layer of indirection to use a function as a stub if the user does not have source code access to it.

### Alternative #3: Annotate harnesses and stubs 

This alternative combines the proposed solution and Alternative #2.
Users annotate the stub (as in Alternative #2) and specify for each harness which stubs to use using an annotation `#[kani::use_stubs(<stub>+)]` placed above the harness.

This could be combined with modules, so that a module can be used to group stubs together, and then harnesses could pull in all the stubs in the module:

```rust
#[cfg(kani)]
mod my_stubs {

  #[kani::stub(foo)]
  fn stub1() { ... }

  #[kani::stub(bar)]
  fn stub2() { ... }

}

#(cfg[kani])
#[kani::proof]
#[kani::use_stubs(my_stubs)]
fn my_harness() { ... }
```

The benefit is that stubs are specified per harness, and (using modules) it might be possible to group stubs together.
The downside is that multiple annotations are required and the stub mappings themselves are remote from the harness.

### Alternative #4:  Specify stubs in a file 

One alternative would be to specify stubs in a file that is passed to `kani-driver` via a command line option.
Users would specify per-harness stub pairings in the file; JSON would be a possible format.
Using a file would eliminate the need for `kani-driver` to extract harness information from the Rust source code; the rest of the implementation would stay the same.
It would also allow the same harness to be run with different stub selections (by supplying a different file).
The disadvantage is that the stub selection is remote from the harness itself.

## Open questions

- Is it worth supporting per-harness stubs, given the extra complication (over using a single stub set for all harnesses)? Do we have good use cases for this?
- How will the required updates to `kani-driver` (e.g., calling `kani-compiler` multiple times) mesh with efforts to parallelize `kani-driver`?
- Would there ever be the need to stub a particular monomorphization of a function, as opposed to the polymorphic function?
This would impact at what stage of the compiler we do the function replacements. 

## Future possibilities

- It would increase the utility of stubbing if we supported stubs for features beyond functions, such as methods and types.
The source code annotations and the interaction between `kani-driver` and `kani-compiler` could likely stay the same, although the underlying technical mechanisms in `kani-compiler` performing these substitutions might be significantly more complex.
**Update**
- It would probably make sense to provide a library of common stubs for users, since many applications might want to stub the same functions and mock the same behaviors (e.g., `rand::random` can be replaced with a function returning `kani::any`).
- Users might reasonably want to use the same set of stubs across multiple harnesses; it might be useful to provide a mechanism for defining and referencing a stub group.