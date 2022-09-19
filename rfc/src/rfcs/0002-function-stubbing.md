- **Feature Name:** Function stubbing (`function_stubbing`)
- **Feature Request Issue:** [model-checking#1695](https://github.com/model-checking/kani/issues/1695)
- **RFC PR:** *Link to original PR*
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

## Summary

This will feature will allow users to specify that certain functions should be replaced with mock functions (stubs) during verification.

## User Impact

We anticipate that stubbing will have a substantial positive impact on the usability of Kani. There are two main motivations for stubbing:


1. Users might need to stub functions containing features that Kani does not support, such as inline assembly.
2. Users might need to stub functions containing code that Kani supports in principle, but which in practice leads to bad verification performance.

In both cases, stubbing would enable users to verify code that cannot currently be verified by Kani (or at least not within a reasonable resource bound).

As an example, consider verifying the following assertion (which can fail):

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

Under this substitution, Kani has a single check, which proves that the assertion can fail. Verification time is 0.03 seconds.

## User Experience

This feature is currently limited to stubbing functions; however, the hope is that the basic ideas and mechanisms will carry over to stubbing other features in the future (such as methods and types).

Stubs will be specified per harness; that is, different harnesses can use different stubs (the reasoning being that users might want to mock different behavior for different harnesses).
Users will specify stubs by attaching the `#[kani::stub_by(original, replacement)]` attribute to each harness function.
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

Kani will exit with an error if

1. a specified `replacement` stub does not exist;
2. the user specifies conflicting stubs for the same harness (i.e., if the same `original` function is mapped to multiple `replacement` functions); or
3. the signature of the `replacement` function is not compatible with the signature of the `original` function.

To teach this feature, we will update the documentation with a section on function stubbing, including simple examples showing how stubbing can help Kani handle code that currently cannot be verified.


## Detailed Design

This is the technical portion of the RFC. Please provide high level details of the implementation you have in mind:

- What are the main components that will be modified? (E.g.: changes to `kani-compiler`, `kani-driver`, metadata,
  installation...)
- How will they be modified? Any changes to how these components communicate?
- Will this require any new dependency?
- What corner cases do you anticipate?

## Rationale and alternatives

- What are the pros and cons of this design?
- What is the impact of not doing this?
- What other designs have you considered? Why didn't you choose them?

## Open questions

- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design?
- Would there ever be the need to stub particular monomorphizations of functions, as opposed to the generic function?

## Future possibilities

What are natural extensions and possible improvements that you predict for this feature that is out of the
scope of this RFC? Feel free to brainstorm here.