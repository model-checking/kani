# Kani Rust Verifier

The Kani Rust Verifier aims to be a bit-precise model-checker for Rust.
Kani ensures that unsafe Rust code is actually safe, and verifies that safe Rust code will not panic at runtime.

## Installing Kani

Until an official release is out, you can [read documentation on how to check out and build Kani yourself](https://model-checking.github.io/kani/install-guide.html).

## What can Kani do?

Our documentation covers:

* [Comparison with other tools](https://model-checking.github.io/kani/tool-comparison.html)
* [Failures that Kani can spot](https://model-checking.github.io/kani/tutorial-kinds-of-failure.html)
* [Kani's current limitations](https://model-checking.github.io/kani/limitations.html)

## How does Kani work?

You write a _proof harness_ that looks a lot like a test harness, except that you can check all possible values using `kani::any()`:

```rust
use my_crate::{function_under_test, is_acceptable, is_valid};

#[kani::proof]
fn check_my_property() {
   let input = kani::any();
   kani::assume(is_valid(input));
   let output = function_under_test(input);
   assert!(is_acceptable(output));
}
```

Kani will then prove that all valid inputs will produce acceptable outputs, without panicking or executing undefined behavior.
You can learn more about how to use Kani [by following the Kani tutorial](https://model-checking.github.io/kani/kani-tutorial.html).

## Security
See [SECURITY](https://github.com/model-checking/kani/security/policy) for more information.

## Developer guide
See [Kani developer documentation](https://model-checking.github.io/kani/dev-documentation.html).

## License
### Kani
Kani is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

### Rust compiler
Kani contains code from the Rust compiler.
The rust compiler is primarily distributed under the terms of both the MIT license and the Apache License (Version 2.0), with portions covered by various BSD-like licenses.

See [the Rust repository](https://github.com/rust-lang/rust) for details.
