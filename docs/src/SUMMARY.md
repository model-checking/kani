# Kani Rust Verifier

- [Getting started](./getting-started.md)
  - [Installation](./install-guide.md)
  - [Usage](./usage.md)
    - [On a single file](./kani-single-file.md)
    - [On a package](./cargo-kani.md)
  - [Verification results](./verification-results.md)

- [Tutorial](./kani-tutorial.md)
  - [First steps](./tutorial-first-steps.md)
  - [Failures that Kani can spot](./tutorial-kinds-of-failure.md)
  - [Loop unwinding](./tutorial-loop-unwinding.md)
  - [Nondeterministic variables](./tutorial-nondeterministic-variables.md)

- [Application](./application.md)
  - [Comparison with other tools](./tool-comparison.md)
  - [Where to start on real code](./tutorial-real-code.md)

- [Developer documentation](dev-documentation.md)
  - [Working with CBMC](./cbmc-hacks.md)
  - [Working with `rustc`](./rustc-hacks.md)
  - [Command cheat sheets](./cheat-sheets.md)
  - [Testing](./testing.md)
    - [Regression testing](./regression-testing.md)
    - [Book runner](./bookrunner.md)

- [Limitations](./limitations.md)
  - [Undefined behaviour](./undefined-behaviour.md)
  - [Rust feature support](./rust-feature-support.md)
    - [Intrinsics](./rust-feature-support/intrinsics.md)
    - [Unstable features](./rust-feature-support/unstable.md)
  - [Overrides](./overrides.md)
