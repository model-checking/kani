![](./kani-logo.png)
![Kani regression](https://github.com/model-checking/kani/actions/workflows/kani.yml/badge.svg)
![Nightly: CBMC Latest](https://github.com/model-checking/kani/actions/workflows/cbmc-latest.yml/badge.svg)

The Kani Rust Verifier is a bit-precise model checker for Rust.

Kani is particularly useful for verifying unsafe code blocks in Rust, where the "[unsafe superpowers](https://doc.rust-lang.org/stable/book/ch19-01-unsafe-rust.html#unsafe-superpowers)" are unchecked by the compiler.
___
Kani verifies:
 * Memory safety (e.g., null pointer dereferences)
 * User-specified assertions (i.e., `assert!(...)`)
 * The absence of panics (e.g., `unwrap()` on `None` values)
 * The absence of some types of unexpected behavior (e.g., arithmetic overflows)

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
use my_crate::{function_under_test, meets_specification, precondition};

#[kani::proof]
fn check_my_property() {
   // Create a nondeterministic input
   let input = kani::any();

   // Constrain it according to the function's precondition
   kani::assume(precondition(input));

   // Call the function under verification
   let output = function_under_test(input);

   // Check that it meets the specification
   assert!(meets_specification(input, output));
}
```

Kani will then try to prove that all valid inputs produce acceptable outputs, without panicking or executing unexpected behavior.
Otherwise Kani will generate a trace that points to the failure.
We recommend following [the tutorial](https://model-checking.github.io/kani/kani-tutorial.html) to learn more about how to use Kani.

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
