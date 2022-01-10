# RMC on a package

RMC currently ships with a `cargo-rmc` script, but this support is limited. If you find any issue, please [filed a bug report](https://github.com/model-checking/rmc/issues/new?assignees=&labels=bug&template=bug_report.md).

# Building and running

Let's assume you have a library project that you can build with `cargo build` and you've written a proof harness somewhere in it that you want to run RMC on:

```rust
#[cfg(rmc)]
#[rmc::proof]
fn my_harness() {
}
```

You should be able to verify the function by using the following command:
```shell
cargo rmc --function my_harness
```

# Package configuration

Users may want to add default configurations for their crate's or workspace's harnesses, and they can do it by adding configurations to their `Cargo.toml` or any other TOML file.

For example, in order to configure a maximum loop unwind threshold for harnesses in a package, you can add the following line to your package `Cargo.toml`:
```toml
[package.metadata.rmc]
flags= { unwind = "5" }
```

Note that RMC will extract any extra configuration from sections `[rmc]`, `[workspace.metadata.rmc]`, `[package.metadata.rmc]` in the TOML file.

# Common cargo-rmc arguments

Cargo rmc supports the rmc standalone arguments described in the [RMC single file section](./rmc-single-file.md). Additionally, it also accepts the following arguments:

**`--config-toml`** Location of a configuration file in toml format for your project. This defaults to to crate's Cargo.toml.

**`--no-config-toml`** Do not use any configuration TOML file.

**`--build-target`** Build for the target triple.