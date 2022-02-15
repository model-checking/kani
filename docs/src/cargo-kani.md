# Kani on a package

Kani currently ships with a `cargo-kani` script, but this support is limited. If you find any issue, please [filed a bug report](https://github.com/model-checking/kani/issues/new?assignees=&labels=bug&template=bug_report.md).

To run `cargo-kani` in your crate, execute from the crate directory:

```
cargo kani [KANI_ARGUMENTS]* [--cbmc-args [CBMC_ARGUMENTS]*]
```

To list all the arguments supported by cargo kani, execute:

```
cargo kani --help
```

# Common cargo-kani arguments

Cargo kani supports the kani standalone arguments described in the [Kani single file section](./kani-single-file.md). Additionally, it also accepts the following arguments:

**`--config-toml`** Location of a configuration file in toml format for your project. This defaults to the crate's Cargo.toml.

**`--build-target`** Build for the target triple.

# Package configuration

Users may want to add default configurations for their crate's or workspace's harnesses, and they can do it by adding configurations to their `Cargo.toml` or any other TOML file.

For example, in order to configure a maximum loop unwind threshold for harnesses in a package, you can add the following line to your package `Cargo.toml`:
```toml
[package.metadata.kani]
flags= { unwind = "5" }
```

Note that Kani will extract any extra configuration from sections `[kani]`, `[workspace.metadata.kani]`, and `[package.metadata.kani]` in the TOML file.
