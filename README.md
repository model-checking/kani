![](./kani-logo.png)
[![Kani regression](https://github.com/model-checking/kani/actions/workflows/kani.yml/badge.svg)](https://github.com/model-checking/kani/actions/workflows/kani.yml)
[![Nightly: CBMC Latest](https://github.com/model-checking/kani/actions/workflows/cbmc-latest.yml/badge.svg)](https://github.com/model-checking/kani/actions/workflows/cbmc-latest.yml)

The Kani Rust Verifier is a bit-precise model checker for Rust.

Kani is useful for checking both safety and correctness of Rust code.
- *Safety*: Kani automatically checks for many kinds of [undefined behavior](https://model-checking.github.io/kani/undefined-behaviour.html).
This makes it particularly useful for verifying unsafe code blocks in Rust, where the "[unsafe superpowers](https://doc.rust-lang.org/stable/book/ch19-01-unsafe-rust.html#unsafe-superpowers)" are unchecked by the compiler.
- *Correctness*: Kani automatically checks panics (e.g. `unwrap()` on `None`), arithmetic overflows, and custom correctness properties, either in the form of assertions (`assert!(...)`) or [function contracts](https://model-checking.github.io/kani/reference/experimental/contracts.html).

## Installation

To install the latest version of Kani ([Rust 1.58+; Linux or Mac](https://model-checking.github.io/kani/install-guide.html)), run:

```bash
cargo install --locked kani-verifier
cargo kani setup
```

See [the installation guide](https://model-checking.github.io/kani/install-guide.html) for more details.

## How to use Kani

Similar to testing, you write a harness, but with Kani you can check all possible values using `kani::any()`:

```rust
use my_crate::{function_under_test, meets_specification};

#[kani::proof]
fn check_my_property() {
   // Create a nondeterministic input
   let input: u8 = kani::any();

   // Call the function under verification
   let output = function_under_test(input);

   // Check that it meets the specification
   assert!(meets_specification(input, output));
}
```

Kani will try to prove that all valid inputs produce outputs that satisfy the specification, without panicking or exhibiting unexpected behavior.
This example is simple; we highly recommend following [the tutorial](https://model-checking.github.io/kani/kani-tutorial.html) to learn more about how to use Kani.

## GitHub Action

Use Kani in your CI with `model-checking/kani-github-action@VERSION`. See the
[GitHub Action section in the Kani
book](https://model-checking.github.io/kani/install-github-ci.html)
for details.

## Security
See [SECURITY](https://github.com/model-checking/kani/security/policy) for more information.

## Contributing
If you are interested in contributing to Kani, please take a look at [the developer documentation](https://model-checking.github.io/kani/dev-documentation.html).

## License
### Kani
Kani is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

### Rust
Kani contains code from the Rust project.
Rust is primarily distributed under the terms of both the MIT license and the Apache License (Version 2.0), with portions covered by various BSD-like licenses.

See [the Rust repository](https://github.com/rust-lang/rust) for details.
