- **Feature Name:** Function and method stubbing (`function_stubbing`)
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/1695>
- **RFC PR:** <https://github.com/model-checking/kani/pull/1723> 
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** <https://github.com/aaronbembenek-aws/kani/tree/mir_transform>

## Summary

Allow users to specify that certain functions and methods should be replaced with mock functions (stubs) during verification.

## User Impact

We anticipate that stubbing will have a substantial positive impact on the usability of Kani:

1. Users might need to stub functions/methods containing features that Kani does not support, such as inline assembly.
2. Users might need to stub functions/methods containing code that Kani supports in principle, but which in practice leads to bad verification performance (for example, if it contains deserialization code).
3. Users could use stubbing to perform compositional reasoning: prove the behavior of a function/method `f`, and then in other proofs---that call `f` indirectly---use a stub of `f` that mocks that behavior but is less complex.

In all cases, stubbing would enable users to verify code that cannot currently be verified by Kani (or at least not within a reasonable resource bound).
Even without stubbing types, the ability to stub functions/methods can help provide verification-friendly abstractions for standard data structures.
For example, [Issue 1673](https://github.com/model-checking/kani/issues/1673) suggests that some Kani proofs run more quickly if `Vec::new` is replaced with `Vec::with_capacity`; function stubbing would allow us to make this substitution everywhere in a codebase (and not just in the proof harness).

In what follows, we give two examples of stubbing external code, using the annotations we propose in this RFC.
We are able to run each of these examples on a modified version of Kani using a proof-of-concept MIR-to-MIR transformation implementing stubbing (the prototype does not support stub-related annotations; instead, it reads the stub mapping from a file).
These examples---involving randomization and deserialization---are the types of functions/methods that are commonly stubbed in other verification and program analysis projects.
Other common examples that we should be able to handle include system calls and timer functions.

### Mocking Randomization

The crate [`rand`](https://crates.io/crates/rand) is widely used (150M downloads).
However, Kani cannot currently handle code that uses it (Kani users have run into this; see <https://github.com/model-checking/kani/issues/1727>).
Consider this example:

```rust
#[cfg(kani)]
#[kani::proof]
fn random_cannot_be_zero() {
    assert_ne!(rand::random::<u32>(), 0);
}
```

For unwind values less than 2, Kani encounters an unwinding assertion error (there is a loop used to seed the random number generator); if we set an unwind value of 2, Kani fails to terminate within 5 minutes.

Using stubbing, we can specify that the function `rand::random` should be replaced with a mocked version:

```rust
#[cfg(kani)]
fn mock_random<T: kani::Arbitrary>() -> T {
    kani::any()
}

#[cfg(kani)]
#[kani::proof]
#[kani::stub_by("rand::random", "mock_random")]
fn random_cannot_be_zero() {
    assert_ne!(rand::random::<u32>(), 0);
}
```

Under this substitution, Kani has a single check, which proves that the assertion can fail. Verification time is 0.02 seconds.

### Mocking Deserialization

In this example, we mock a [serde_json](https://crates.io/crates/serde_json) (96M downloads) deserialization method so that we can prove a property about the following [Firecracker function](https://github.com/firecracker-microvm/firecracker/blob/01eba51ded2f5439da91a2d73280f579651b067c/src/api_server/src/request/vsock.rs#L11) that parses a configuration from some raw data:

```rust
fn parse_put_vsock(body: &Body) -> Result<ParsedRequest, Error> {
    METRICS.put_api_requests.vsock_count.inc();
    let vsock_cfg = serde_json::from_slice::<VsockDeviceConfig>(body.raw()).map_err(|err| {
        METRICS.put_api_requests.vsock_fails.inc();
        err
    })?;

    // Check for the presence of deprecated `vsock_id` field.
    let mut deprecation_message = None;
    if vsock_cfg.vsock_id.is_some() {
        // vsock_id field in request is deprecated.
        METRICS.deprecated_api.deprecated_http_api_calls.inc();
        deprecation_message = Some("PUT /vsock: vsock_id field is deprecated.");
    }

    // Construct the `ParsedRequest` object.
    let mut parsed_req = ParsedRequest::new_sync(VmmAction::SetVsockDevice(vsock_cfg));
    // If `vsock_id` was present, set the deprecation message in `parsing_info`.
    if let Some(msg) = deprecation_message {
        parsed_req.parsing_info().append_deprecation_message(msg);
    }

    Ok(parsed_req)
}
```

We manually mocked some of the Firecracker types with simpler versions to reduce the number of dependencies we had to pull in (e.g., we removed some enum variants, unused struct fields).
With these changes, we were able to prove that the configuration data has a vsock ID if and only if the parsing metadata includes a deprecation message: 

```rust
#[cfg(kani)]
fn get_vsock_device_config(action: RequestAction) -> Option<VsockDeviceConfig> {
    match action {
        RequestAction::Sync(vmm_action) => match *vmm_action {
            VmmAction::SetVsockDevice(dev) => Some(dev),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(2)]
#[kani::stub_by("serde_json::deserialize_slice", "mock_deserialize")]
fn test_deprecation_vsock_id_consistent() {
    // We are going to mock the parsing of this body, so might as well use an empty one.
    let body: Vec<u8> = Vec::new();
    if let Ok(res) = parse_put_vsock(&Body::new(body)) {
        let (action, mut parsing_info) = res.into_parts();
        let config = get_vsock_device_config(action).unwrap();
        assert_eq!(
            config.vsock_id.is_some(),
            parsing_info.take_deprecation_message().is_some()
        );
    }
}
```

Crucially, we did this by stubbing out `serde_json::from_slice` and replacing it with our mock version below, which ignores its input and creates a "symbolic" configuration struct:

```rust
#[cfg(kani)]
fn symbolic_string(len: usize) -> String {
    let mut v: Vec<u8> = Vec::with_capacity(len);
    for _ in 0..len {
        v.push(kani::any());
    }
    unsafe { String::from_utf8_unchecked(v) }
}

#[cfg(kani)]
fn mock_deserialize(_data: &[u8]) -> serde_json::Result<VsockDeviceConfig> {
    const STR_LEN: usize = 1;
    let vsock_id = if kani::any() {
        None
    } else {
        Some(symbolic_string(STR_LEN))
    };
    let guest_cid = kani::any();
    let uds_path = symbolic_string(STR_LEN);
    let config = VsockDeviceConfig {
        vsock_id,
        guest_cid,
        uds_path,
    };
    Ok(config)
}
```

The proof takes 170 seconds to complete (using Kissat as the backend SAT solver for CBMC).

## User Experience

This feature is currently limited to stubbing functions and methods.
We anticipate that the annotations we propose here could also be used when stubbing types, although the underlying technical approach might have to change.

Stubs will be specified per harness; that is, different harnesses can use different stubs.
This is one of the main design points.
Users might want to mock the behavior of a function within one proof harness, and then mock it a different way for another harness, or even use the original function definition.
It would be overly restrictive to impose the same stub definitions across all proof harnesses.
A good example of this is compositional reasoning: in some harnesses, we want to prove properties of a particular function (and so want to use its actual implementation), and in other harnesses we want to assume that that function has those properties.

Users will specify stubs by attaching the `#[kani::stub_by(<original>, <replacement>)]` attribute to each harness function.
The arguments `original` and `replacement` give the names of functions (as string literals), relative to the crate of the harness (*not* relative to the module of the harness).
The attribute may be specified multiple times per harness, so that multiple (non-conflicting) stub pairings are supported.

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
    #[kani::stub_by("rand::random", "mock_random")]
    #[kani::stub_by("my_mod::foo", "my_mod::bar")]
    fn my_harness() { ... }

}
```

### Stub Sets

As a convenience, we will provide a macro `kani::stub_set` that allows users to specify sets of stubs that can be applied to multiple harnesses:

```rust
kani::stub_set! {
    my_io_stubs,
    stub_by("std::fs::read", "my_read"),
    stub_by("std::fs::write", "my_write"),
}
```

When declaring a harness, users can use the `#[kani::use_stub_set("<stub_set_name>")]` attribute to apply the stub set:

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::use_stub_set("my_io_stubs")]
fn my_harness() { ... }
```

A similar mechanism can be used to aggregate stub sets:

```rust
kani::stub_set!() {
    all_my_stubs,
    use_stub_set("my_io_stubs"),
    use_stub_set("my_other_stubs"),
}
```

### Error Conditions

Given a set of `original`-`replacement` pairs, Kani will exit with an error if

1. a specified `replacement` stub does not exist;
2. the user specifies conflicting stubs for the same harness (i.e., if the same `original` function is mapped to multiple `replacement` functions); or
3. the signature of the `replacement` function is not compatible with the signature of the `original` function.

### Pedagogy

To teach this feature, we will update the documentation with a section on function and method stubbing, including simple examples showing how stubbing can help Kani handle code that currently cannot be verified.

## Detailed Design

We expect that this feature will require changes primarily to `kani-compiler`.
Before doing code generation, `kani-compiler` already collects harnesses; we will extend this to also collect stub mapping information.
Since stubs are specified on a per-harness basis, we need to generate multiple versions of code if all harnesses do not agree on their stub mappings; accordingly, we will update `kani-compiler` to generate multiple versions of code as appropriate. 
To do the stubbing, we will plug in a new MIR-to-MIR transformation that replaces the bodies of specified functions with their replacements.
This can be achieved via `rustc`'s query mechanism: if the user wants to replace `foo` with `bar`, then when the compiler requests the MIR for `foo`, we instead return the MIR for `bar`.
`kani-compiler` will also be responsible for checking for the error conditions previously enumerated.

We anticipate that this design will evolve and be iterated upon.

## Rationale and alternatives: user experience

Stubbing is a *de facto* necessity for verification tools, and the lack of stubbing has a negative impact on the usability of Kani.

### Benefits

- Because stubs are specified by annotating the harness, the user is able to specify stubs for functions they do not have source access to (like library functions).
This contrasts with annotating the function to be replaced (such as with function contracts).
- The current design provides the user with flexibility, as they can specify different sets of stubs to use for different harnesses.
This is important if users are trying to perform compositional reasoning using stubbing, since in some harnesses a function/method should be fully verified, in in other harnesses its behavior should be mocked.
- The stub selections are located adjacent to the harness, which makes it easy to understand which replacements are going to happen for each harness.

### Risks

- Users can always write stubs that do not correctly correspond to program behavior, and so a successful verification does not actually mean the program is bug-free.
This is similar to other specification bugs.
All the stubbing code will be available, so it is possible to inspect the assumptions it makes.

### Comparison to function contracts

- The [currently proposed function contract mechanism](https://github.com/model-checking/kani/tree/features/function-contracts) does not provide a way to specify contracts on external functions.
This is one of the key motivations for stubbing.
- In many cases, stubs are more user-friendly than contracts.
With contracts, it is sometimes necessary to explicitly provide information that is automatically captured in Rust (such as which memory is written).
Furthermore, contract predicates constitute a DSL of their own that needs to be learned; using stubbing, we can stick to using just Rust.

### Alternative #1: Annotate stubbed functions

In this alternative, users add an attribute `#[kani::stub_by(<replacement>)]` to the function that should be replaced.
This approach is similar to annotating a function with a contract specifying its behavior (the stub acts like a programmatic contract).
The major downside with this approach is that it would not be possible to stub external code. We see this as a likely use case that needs to be supported: users will want to replace `std` library functions or functions from arbitrary external crates.

### Alternative #2: Annotate stubs

In this alternative, users add an attribute `#[kani::stub(<original>)]` to the stub function itself, saying which function it replaces:

```rust
#[cfg(kani)]
#[kani::stub("rand::random")]
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

  #[kani::stub("foo")]
  fn stub1() { ... }

  #[kani::stub("bar")]
  fn stub2() { ... }

}

#(cfg[kani])
#[kani::proof]
#[kani::use_stubs("my_stubs")]
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

This approach entails less user effort than Alternative #1, but it has the same downside that it requires all source code to be available.
It also might be difficult to inject code in a way that names are correctly resolved (e.g., if the replacement code comes from a different crate).
Also, source code is difficult to work with (e.g., unexpanded macros).

On the last two points, we might be able to take advantage of an existing source analysis platform like `rust-analyzer` (which has facilities like structural search replace), but this would add more (potentially fragile) dependencies to Kani.

### Alternative #3: AST-to-AST or HIR-to-HIR transformation

In this alternative, we implement stubbing by rewriting the [AST](https://rustc-dev-guide.rust-lang.org/syntax-intro.html) or [High-Level IR (HIR)](https://rustc-dev-guide.rust-lang.org/hir.html) of the program.
The HIR is a more compiler-friendly version of the AST; it is what is used for type checking.
To swap out a function, method, or type at this level, it looks like it would be necessary to add another pass to `rustc` that takes the initial AST/HIR and produces a new AST/HIR with the appropriate replacements.

The advantage with this approach is, like source transformations, it would be very flexible.
The downside is that it would require modifying `rustc` (as far as we know, there is not an API for plugging in a new AST/HIR pass), and would also require performing the transformations at a very syntactic level: although the AST/HIR would likely be easier to work with than source code directly, it is still very close to the source code and not very abstract.
Furthermore, it would require that we have access to the AST/HIR for all external code, and---provided we supported stubbing across crate boundaries---we would need to figure out how to inject the AST/HIR from one crate into another (the AST/HIR is usually just constructed for the crate currently being compiled).

## Open questions

- Would there ever be the need to stub a particular monomorphization of a function, as opposed to the polymorphic function?
- What does it mean for the replacement function/method to be "compatible" with the original one?
Requiring the replacement's type to be a subtype of the original type is likely stronger than what we want.
For example, if the original function is polymorphic but monomorphized to only a single type, then it seems okay to replace it with a function that matches the monomorphized type.
- How can we allow a user to stub a method and access private fields of `self`?
This should be possible using `std::mem::transmute`, but can we hide this from the user and make it less brittle?

## Future possibilities

- It would increase the utility of stubbing if we supported stubs for types.
The source code annotations could likely stay the same, although the underlying technical approach performing these substitutions might be significantly more complex.
- It would probably make sense to provide a library of common stubs for users, since many applications might want to stub the same functions and mock the same behaviors (e.g., `rand::random` can be replaced with a function returning `kani::any`).