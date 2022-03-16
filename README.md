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

## What guarantees does Kani provide?
If Kani verifies your code under a proof harness, then, under the assumptions of the specification, you have assurance that all code reachable from the proof harness:
* Does not have runtime panics 
* All user-added assertions are correct.
* There are no memory safety issues, including in unsafe code.

Adding support for additional checks is prioritized based on customer need.
[We would appreciate your feedback as to which are the highest priority areas for your projects](https://github.com/model-checking/kani/issues/new/choose).

## What do I need to trust when running Kani?
Kani analyzes the MIR representation of a Rust program.
If the real-world behaviour of the program diverges from the MIR, then Kani's guarantees may not hold.
Examples of how MIR can differ from real-world behaviour are discussed below. 

Kani uses the program analysis tool [CBMC](https://github.com/diffblue/cbmc) as a back-end.
If the interpretation of MIR using CBMC diverges from the real-world behaviour of the MIR, then Kani's guarantees may not hold.
Examples of how Kani may fail to faithfully model MIR behaviour are also discussed below.

In particular, you need to trust the following stages of the Kani verification pipeline:

1. **You need to trust the correctness of your specifications.**
Kani analyzes code against a user specification, typically given as a “proof harness”.
The assurance given by Kani depends on the specification correctly describing the environment in which the code runs, and the expected behavior of the code.
If the specification has bugs, then Kani may (accurately) declare that buggy code is OK, since it complies with the buggy specification.
We have developed training materials and checklists to aid developers in writing sound specifications for code-level model-checking (available at
https://github.com/awslabs/aws-templates-for-cbmc-proofs/tree/master/training-material).
Currently, the examples focus on C verification; we are working on adding Rust examples.

1. **You need to trust the rustc compiler to properly parse rust into MIR.**
Kani analyzes code at the MIR level.
It uses the standard rustc compiler to translate Rust into MIR.
Correctness guarantees, therefore, apply only to the particular compiler, and set of compilation flags (e.g. optimizations, machine architecture, etc) used.
A different compiler, or the same compiler with different flags, may produce semantically different MIR from the same underlying source code.
This can occur even in cases where there are no bugs in the underlying Rustc compiler, as parts of the Rust language are either undefined in the standard, or called out as implementation specific (for example, object layouts).
Specific examples of these issues are called out later in the document, and we plan to work with the rustc team to produce MIR that makes explicit these implementation dependent choices.

1. **You need to trust the correctness of Kani’s interpretation of MIR.**
Kani operates by translating MIR into the GOTO format used by the CBMC tool.
If this translation is incorrect, then it is possible for Kani to be unsound – i.e. to report “No Error”, when there was in fact an error.

1. **You need to trust CBMC.**
Kani uses CBMC under the hood as a verification engine.
CBMC is a mature verification tool which has successfully been used to verify a variety of C codebases at Amazon.
However, it is possible that CBMC may have latent soundness bugs, which would affect the soundness of an Kani proof.

1. **You need to trust the correctness of the rest of your system.**
Kani analyzes code at the MIR level.
Even if code is correct at the MIR level, it may still exhibit bugs due to issues in the rustc compiler, llvm, the operating system, the CPU, etc. Kani cannot detect issues that happen after MIR translation.

## How does Kani work?

You write a _proof harness_ that looks a lot like a test harness, except that you can check all possible values using `kani::any()`:

```rust
use my_crate::{function_under_test, is_valid, meets_specification};

#[kani::proof]
fn check_my_property() {

 let input = kani::any();

 kani::assume(is_valid(input));

 let output = function_under_test(input);

 assert!(meets_specification(input, output));
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
