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

In what follows, we give three examples of stubbing being put to work.
Each of these examples runs on a prototype version of the stubbing mechanism we propose (except that the prototype does not support stubbing annotations; instead, it reads stubbing pairs from a file).

### Mocking Randomization

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

### Mocking Vec

[Issue 1673](https://github.com/model-checking/kani/issues/1673) documents that Kani performs poorly on the following program:

```rust
const N: usize = 9;

#[cfg_attr(kani, kani::proof, kani::unwind(10))]
fn vec_harness() {
    let mut v: Vec<String> = Vec::new();
    for _i in 0..N {
        v.push(String::from("ABC"));
    }
    assert_eq!(v.len(), N);
    let index: usize = kani::any();
    kani::assume(index < v.len());
    let x = &v[index];
    assert_eq!(*x, "ABC");
}
```

On my laptop, it takes 400 seconds to complete.
The issue reports that performance is much improved if `Vec::new()` is replaced with `Vec::with_capacity(N)`.
Using stubbing, we can perform this transformation without modifying the harness's code:

```rust
const N: usize = 9;

fn mock_vec_new<T>() -> Vec<T> {
    Vec::with_capacity(N)
}

#[cfg_attr(kani, kani::proof, kani::unwind(10))]
#[cfg_attr(kani, kani::stub_by(std::vec::Vec::<T>::new, mock_vec_new))]
fn vec_harness() {
    let mut v: Vec<String> = Vec::new();
    for _i in 0..N {
        v.push(String::from("ABC"));
    }
    assert_eq!(v.len(), N);
    let index: usize = kani::any();
    kani::assume(index < v.len());
    let x = &v[index];
    assert_eq!(*x, "ABC");
}
```

The harness now runs in 17 seconds (23x speedup).
What is intriguing is that, with stubbing, we can make this substitution not only in the harness (where we could have always done it by hand), but also everywhere else in the code base, including external code that we could not have otherwise modified.

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

### Accessing Private Fields in Method Stubs

When stubbing a method, users might need to access private fields within the struct.
Because users are writing stubs in Rust and are subject to Rust's restrictions, they cannot access private fields directly.
Instead, they have to create a mock struct that has the same layout as the original struct, but with public fields, and then use `std::mem::transmute` to cast between them.

```rust
struct Foo {
    x: u32,
}

impl Foo {
    pub fn m(&self) -> u32 {
        0
    }
}

struct MockFoo {
    pub x: u32,
}

fn mock_m(foo: &Foo) {
    let mock: &MockFoo = unsafe { std::mem::transmute(foo) };
    return mock.x;
}

#[cfg(kani)]
#[kani::proof]
#[kani::stub_by(Foo::m, mock_m)]
fn my_harness() { ... }
```

We acknowledge that this approach is both slightly clunky and brittle, since it requires knowing the layout of the original struct (which could change) and depends on `rustc` producing the same layout for both structs (which is not guaranteed unless both are annotated with `repr(C)`).
It is an open question whether we can provide a mechanism to hide some of this ugliness from the user, or at least error if, say, the struct layouts differ.

### Error Conditions

Given a set of `original`-`replacement` pairs, Kani will exit with an error if

1. a specified `replacement` stub does not exist;
2. the user specifies conflicting stubs for the same harness (i.e., if the same `original` function is mapped to multiple `replacement` functions); or
3. the signature of the `replacement` function is not compatible with the signature of the `original` function.

### Pedagogy

To teach this feature, we will update the documentation with a section on function and method stubbing, including simple examples showing how stubbing can help Kani handle code that currently cannot be verified.

## Detailed Design

We discuss both the design in its full form and a simplified version appropriate for a first step.
We anticipate that this design will evolve and be iterated upon.

### Full form

In its full form, we expect that this feature will require substantial changes both to `kani-driver` and `kani-compiler`.

`kani-driver` will need to be responsible for determining which stubs are required for each harness, which will require moving the code for identifying harnesses from `kani-compiler` to `kani-driver`.
After `kani-driver` has determined the set of stubs to use for each harness, it will invoke `kani-compiler` once for each set of stubs, passing the relevant stub pairings as command line arguments.
The output of `kani-compiler` will be specialized to that stub set.
After `kani-compiler` has finished compiling under a given stub set, `kani-driver` will run the harnesses that use that stub set.

`kani-compiler` will be extended with a command line option specifying stub pairings, and a new MIR-to-MIR transformation that replaces the bodies of specified functions with their replacements.
This can be achieved via `rustc`'s query mechanism: if the user wants to replace `foo` with `bar`, then when the compiler requests the MIR for `foo`, we instead return the MIR for `bar`.
`kani-compiler` will be responsible for checking for the error conditions enumerated in the previous section.

### First step

As a first step, we will require that stubbing will only be enabled if Kani is also run with the `--harness` flag.
Since there is only a single stub set in this situation, `kani-driver` needs to run `kani-compiler` only once.

## Rationale and alternatives: user experience

Stubbing is a *de facto* necessity for verification tools, and the lack of stubbing has a negative impact on the usability of Kani.

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

### Alternative #4: Specify stubs in a file 

One alternative would be to specify stubs in a file that is passed to `kani-driver` via a command line option.
Users would specify per-harness stub pairings in the file; JSON would be a possible format.
Using a file would eliminate the need for `kani-driver` to extract harness information from the Rust source code; the rest of the implementation would stay the same.
It would also allow the same harness to be run with different stub selections (by supplying a different file).
The disadvantage is that the stub selection is remote from the harness itself.

## Rationale and alternatives: stubbing mechanism

Our approach is based on a MIR-to-MIR transformation.
The advantages are that it operates over a relatively simple intermediate representation and `rustc` has good support for plugging in MIR-to-MIR transformations, so it would not require any changes to `rustc` itself.
At this stage of the compiler, names have been fully resolved, and there is no problem with swapping in the body of a function defined in one crate for a function defined in another.

The major downside with the MIR-to-MIR transformation is that it does not appear to be possible to stub types at that stage (there is no way to change the definition of a type through the MIR).
Thus, our proposed approach will not be a fully general stubbing solution.
However, it is technically feasible and relatively clean, and provides benefits over having no stubbing at all (as can be seen in the examples in the first part of this document).

Furthermore, it can be used as part of a portfolio of stubbing approaches, where users stub local types using conditional compilation (see Alternative #1), and Kani provides a modified version of the standard library with verification-friendly versions of types like `std::vec::Vec`.

### Alternative #1: Conditional compilation

In this baseline alternative, we do not provide any stubbing mechanism at all.
Instead, users can effectively stub local code (functions, methods, and types) using conditional compilation.
For example, they could specify using `#[cfg(kani)]` to turn off the original definition and turn on the replacement definition when Kani is running, similarly to the ghost state approach taken in the [Tokio Bytes proof](https://model-checking.github.io/kani-verifier-blog/2022/08/17/using-the-kani-rust-verifier-on-tokio-bytes.html).

The disadvantage with this approach is that it does not provide any way to stub external code, which is one of the main motivations of our proposed approach.

### Alternative #2: Source-to-source transformation

In this alternative, we rewrite the source code before it even gets to the compiler.
The advantage with this approach is that it is very flexible, allowing us to stub functions, methods, and types, either by directly replacing them, or appending their replacements and injecting appropriate conditional compilation guards.

The downside with this approach is that it requires all source code to be available.
It also might be difficult to inject code in a way that names are correctly resolved (e.g., if the replacement code comes from a different crate).
Also, source code is difficult to work with, and includes things like unexpanded macros.

On the last two points, we might be able to take advantage of an existing source analysis platform like `rust-analyzer` (which has facilities like structural search replace), but this would add more (potentially fragile) dependencies to Kani.

### Alternative #3: AST-to-AST or HIR-to-HIR transformation

In this alternative, we implement stubbing by rewriting the AST or [High-Level IR (HIR)](https://rustc-dev-guide.rust-lang.org/hir.html) of the program.
The HIR is a more compiler-friendly version of the AST; it is what is used for type checking.
To swap out a function, method, or type at this level, it looks like it would be necessary to add another pass to `rustc` that takes the initial AST/HIR and produces a new AST/HIR with the appropriate replacements.

The advantage with this approach is, like source transformations, it would be very flexible.
The downside is that it would require modifying `rustc` (as far as we know, there is not an API for plugging in a new AST/HIR pass), and would also require performing the transformations at a very syntactic level: although the AST/HIR would likely be easier to work with than source code directly, it is still very close to the source code and not very abstract.
Furthermore, it would require that we have access to the AST/HIR for all external code, and--provided we supported stubbing across crate boundaries--we would need to figure out how to inject the AST/HIR from one crate into another (the AST/HIR is usually just constructed for the crate currently being compiled).

## Open questions

- Is it worth supporting per-harness stubs, given the extra complication (over using a single stub set for all harnesses)? Do we have good use cases for this?
- How will the required updates to `kani-driver` (e.g., potentially calling `kani-compiler` multiple times) mesh with efforts to parallelize `kani-driver`?
- Would there ever be the need to stub a particular monomorphization of a function, as opposed to the polymorphic function?
- What does it mean for the replacement function/method to be "compatible" with the original one?
Requiring the replacement's type to be a subtype of the original type is likely stronger than what we want.
For example, if the original function is polymorphic but monomorphized to only a single type, then it seems okay to replace it with a function that matches the monomorphized type.
- When a user stubs a method and wants access to private fields, is there some way we can hide the `std::mem::transmute` ugliness?
Can we error if the mock struct's layout differs from the original struct's?

## Future possibilities

- It would increase the utility of stubbing if we supported stubs for types.
The source code annotations could likely stay the same, although the underlying technical approach performing these substitutions might be significantly more complex.
- It would probably make sense to provide a library of common stubs for users, since many applications might want to stub the same functions and mock the same behaviors (e.g., `rand::random` can be replaced with a function returning `kani::any`).