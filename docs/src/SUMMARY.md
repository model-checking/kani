# Kani Rust Verifier

- [Getting started](./getting-started.md)
  - [Installation](./install-guide.md)
    - [Building from source](./build-from-source.md)
    - [GitHub CI Action](./install-github-ci.md)
  - [Using Kani](./usage.md)
  - [Verification results](./verification-results.md)

- [Tutorial](./kani-tutorial.md)
  - [First steps](./tutorial-first-steps.md)
  - [Failures that Kani can spot](./tutorial-kinds-of-failure.md)
  - [Loop unwinding](./tutorial-loop-unwinding.md)
  - [Nondeterministic variables](./tutorial-nondeterministic-variables.md)

- [Reference](./reference.md)
  - [Attributes](./reference/attributes.md)
  - [Experimental features](./reference/experimental/experimental-features.md)
    - [Coverage](./reference/experimental/coverage.md)
    - [Stubbing](./reference/experimental/stubbing.md)
    - [Debugging verification failures](./reference/experimental/debugging-verification-failures.md)
- [Application](./application.md)
  - [Comparison with other tools](./tool-comparison.md)
  - [Where to start on real code](./tutorial-real-code.md)

- [Developer documentation](dev-documentation.md)
  - [Coding conventions](./conventions.md)
  - [Working with CBMC](./cbmc-hacks.md)
  - [Working with `rustc`](./rustc-hacks.md)
  - [Migrating to StableMIR](./stable-mir.md)
  - [Command cheat sheets](./cheat-sheets.md)
  - [cargo kani assess](./dev-assess.md)
  - [Testing](./testing.md)
    - [Regression testing](./regression-testing.md)
    - [(Experimental) Testing with a Large Number of Repositories](./repo-crawl.md)
  - [Performance comparisons](./performance-comparisons.md)
    - [`benchcomp` command line](./benchcomp-cli.md)
    - [`benchcomp` configuration file](./benchcomp-conf.md)
    - [Custom parsers](./benchcomp-parse.md)

- [Limitations](./limitations.md)
  - [Undefined behaviour](./undefined-behaviour.md)
  - [Rust feature support](./rust-feature-support.md)
    - [Intrinsics](./rust-feature-support/intrinsics.md)
    - [Unstable features](./rust-feature-support/unstable.md)
  - [Overrides](./overrides.md)

- [Crates Documentation](./crates/index.md)

---

- [FAQ](./faq.md)
