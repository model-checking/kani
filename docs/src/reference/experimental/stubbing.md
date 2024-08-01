# Stubbing

Stubbing (or mocking) is an unstable feature which allows users to specify that certain items should be replaced with stubs (mocks) of those items during verification.
At present, the only items where stubbing can be applied are functions and methods (see [limitations](#limitations) for more details).

## When to consider stubbing

In general, we have identified three reasons where users may consider stubbing:
 - **Unsupported features:** The code under verification contains features that Kani does not support, such as inline assembly.
 - **Bad performance:** The code under verification contains features that Kani supports, but it leads to bad verification performance (for example, deserialization code).
 - **Compositional reasoning:** The code under verification contains code that has been verified separately.
                                Stubbing the code that has already been verified with a less complex version that mimics its behavior can result in reduced verification workloads.

In most cases, stubbing enables users to verify code that otherwise would be impractical to verify.
Although definitions for *mocking* (normally used in testing) and *stubbing* may slightly differ depending on who you ask, we often use both terms interchangeably.

## Components

The stubbing feature can be enabled by using the `--enable-stubbing` option when calling Kani.
Since it's an unstable feature, it requires passing the `--enable-unstable` option in addition to `--enable-stubbing`.

At present, the only component of the stubbing feature is [the `#[kani::stub(<original>, <replacement>)]` attribute](#the-kanistub-attribute),
which allows you to specify the pair of functions/methods that must be stubbed in a harness.

<!--
the other components expected to be here in the future are: the `stub_set(...)!` macro, off-the-shelf verification-friendly implementations, and automated
stubbing suggestions)
-->

## The `#[kani::stub(...)]` attribute

The stub attribute `#[kani::stub(<original>, <replacement>)]`  is the main tool of the stubbing feature.

It indicates to Kani that the function/method with name `<original>` should be replaced with the function/method with name `<replacement>` during the compilation step.
The names of these functions/methods are **resolved using Rust's standard name resolution rules**.
This includes support for imports like `use foo::bar as baz`, as well as imports of multiple versions of the same crate.

**This attribute must be specified on a per-harness basis**. This provides a high degree of flexibility for users, since they are given the option to stub the same item with different replacements (or not use stubbing at all) depending on the proof harness. In addition, **the attribute can be specified multiple times per harness**, so that multiple (non-conflicting) stub pairings are supported.

### An example: stubbing `random`

Let's see a simple example where we use the [`rand::random`](https://docs.rs/rand/latest/rand/fn.random.html) function
to generate an encryption key.

```rust
#[cfg(kani)]
#[kani::proof]
fn encrypt_then_decrypt_is_identity() {
    let data: u32 = kani::any();
    let encryption_key: u32 = rand::random();
    let encrypted_data = data ^ encryption_key;
    let decrypted_data = encrypted_data ^ encryption_key;
    assert_eq!(data, decrypted_data);
}

```

At present, Kani fails to verify this example due to [issue #1781](https://github.com/model-checking/kani/issues/1781).

However, we can work around this limitation thanks to the stubbing feature:

```rust
#[cfg(kani)]
fn mock_random<T: kani::Arbitrary>() -> T {
    kani::any()
}

#[cfg(kani)]
#[kani::proof]
#[kani::stub(rand::random, mock_random)]
fn encrypt_then_decrypt_is_identity() {
    let data: u32 = kani::any();
    let encryption_key: u32 = rand::random();
    let encrypted_data = data ^ encryption_key;
    let decrypted_data = encrypted_data ^ encryption_key;
    assert_eq!(data, decrypted_data);
}
```

Here, the `#[kani::stub(rand::random, mock_random)]` attribute indicates to Kani that it should replace `rand::random` with the stub `mock_random`.
Note that this is a fair assumption to do: `rand::random` is expected to return any `u32` value, just like `kani::any`.

Now, let's run it through Kani:

```bash
cargo kani --enable-unstable --enable-stubbing --harness encrypt_then_decrypt_is_identity
```

The verification result is composed of a single check: the assertion corresponding to `assert_eq!(data, decrypted_data)`.

```
RESULTS:
Check 1: encrypt_then_decrypt_is_identity.assertion.1
         - Status: SUCCESS
         - Description: "assertion failed: data == decrypted_data"
         - Location: src/main.rs:18:5 in function encrypt_then_decrypt_is_identity


SUMMARY:
 ** 0 of 1 failed

VERIFICATION:- SUCCESSFUL
```

Kani shows that the assertion is successful, avoiding any issues that appear if we attempt to verify the code without stubbing.

## Limitations

In the following, we describe all the limitations of the stubbing feature.

### Usage restrictions

The usage of stubbing is limited to the verification of a single harness.
Therefore, users are **required to pass the `--harness` option** when using the stubbing feature.

In addition, this feature **isn't compatible with [concrete playback](./concrete-playback.md)**.

### Support

Support for stubbing is currently **limited to functions and methods**. All other items aren't supported.

The following are examples of items that could be good candidates for stubbing, but aren't supported:
- Types
- Macros
- Traits
- Intrinsics

We acknowledge that support for method stubbing isn't as ergonomic as it could be.
A common problem when attempting to define method stubs is that we don't have access to the private fields of an object (i.e., the fields in `self`).
One workaround is to use the unsafe function `std::mem::transmute`, as in this example:

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
  
  fn mock_m(foo: &Foo) -> u32 {
      let mock: &MockFoo = unsafe { std::mem::transmute(foo) };
      return mock.x;
  }
  
  #[cfg(kani)]
  #[kani::proof]
  #[kani::stub(Foo::m, mock_m)]
  fn my_harness() { ... }
  ```

However, this isn't recommended since it's unsafe and error-prone.
In general, we don't recommend stubbing for private functions/methods.
Doing so can lead to brittle proofs: private functions/methods are subject to change or removal even in version minor upgrades (they aren't part of the APIs).
Therefore, proofs that rely on stubbing for private functions/methods might incur a high maintenance burden.

## Error conditions

Given a set of `original`-`replacement` pairs, Kani will exit with an error if:
 1. a specified `original` function does not exist;
 2. a specified `replacement` stub does not exist;
 3. the user specifies conflicting stubs for the same harness (e.g., if the same `original` function is mapped to multiple `replacement` functions); or
 4. the signature of the `replacement` stub is not compatible with the signature of the `original` function/method (see next section).

### Stub compatibility and validation

We consider a stub and a function/method to be compatible if all the following conditions are met:

- They have the same number of parameters.
- They have the same return type.
- Each parameter in the stub has the same type as the corresponding parameter in the original function/method.
- The stub must have the same number of generic parameters as the original function/method.
However, a generic parameter in the stub is allowed to have a different name than the corresponding parameter in the original function/method.
For example, the stub `bar<A, B>(x: A, y: B) -> B` is considered to have a type compatible with the function `foo<S, T>(x: S, y: T) -> T`.
- The bounds for each type parameter don't need to match; however, all calls to the original function must also satisfy the bounds of the stub.

The final point is the most subtle.
We don't require that a type parameter in the signature of the stub implements the same traits as the corresponding type parameter in the signature of the original function/method.
However, Kani will reject a stub if a trait mismatch leads to a situation where a statically dispatched call to a trait method cannot be resolved during monomorphization.
For example, this restriction rules out the following harness:

```rust
fn foo<T>(_x: T) -> bool {
    false
}

trait DoIt {
    fn do_it(&self) -> bool;
}

fn bar<T: DoIt>(x: T) -> bool {
    x.do_it()
}

#[kani::proof]
#[kani::stub(foo, bar)]
fn harness() {
    assert!(foo("hello"));
}
```

The call to the trait method `DoIt::do_it` is unresolvable in the stub `bar` when the type parameter `T` is instantiated with the type `&str`.
On the other hand, this approach provides some flexibility, such as allowing our earlier example of mocking `rand::random`:
both `rand::random` and `my_random` have type `() -> T`, but in the first case `T` is restricted such that the type `Standard` implements `Distribution<T>`,
whereas in the latter case `T` has to implement `kani::Arbitrary`.
This trait mismatch is allowed because at this call site `T` is instantiated with `u32`, which implements `kani::Arbitrary`.
