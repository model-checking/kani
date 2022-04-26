# Working with `rustc`
Kani is developed on the top of the rust compiler, which is not distributed on [crates.io](https://crates.io/) and it depends on
bootstrapping mechanisms to properly build its components.
Thus, our dependency on `rustc` crates are not declared in our `Cargo.toml`.

Below are a few hacks that will make it easier to develop on the top of `rustc`.

## Code analysis for `rustc` definitions

IDEs rely on `cargo` to find dependencies and sources to provide proper code analysis and code completion.
In order to get these features working for `rustc` crates, you can do the following:

### VSCode

Add the following to the `rust-analyzer` extension settings in `settings.json`:
```json
    "rust-analyzer.rustcSource": "discover",
    "rust-analyzer.workspace.symbol.search.scope": "workspace_and_dependencies",
```

Ensure that any packages that use `rustc` data structures have the following line set in their `Cargo.toml`

```toml
[package.metadata.rust-analyzer]
# This package uses rustc crates.
rustc_private=true
```

You may also need to install the `rustc-dev` package using rustup

```
rustup toolchain install nightly --component rustc-dev
```

### CLion / IntelliJ
This is not a great solution, but it works for now (see <https://github.com/intellij-rust/intellij-rust/issues/1618>
for more details).
Edit the `Cargo.toml` of the package that you're working on and add artificial dependencies on the `rustc` packages that you would like to explore.

```toml
# This configuration doesn't exist so it shouldn't affect your build.
[target.'cfg(KANI_DEV)'.dependencies]
# Adjust the path here to point to a local copy of the rust compiler.
# The best way is to use the rustup path. Replace <toolchain> with the
# proper name to your toolchain.
rustc_driver = { path = "~/.rustup/toolchains/<toolchain>/lib/rustlib/rustc-src/rust/compiler/rustc_driver" }
rustc_interface = { path = "~/.rustup/toolchains/<toolchain>/lib/rustlib/rustc-src/rust/compiler/rustc_interface" }
```

**Don't forget to rollback the changes before you create your PR.**

## Custom `rustc`

There are a few reasons why you may want to use your own copy of `rustc`. E.g.:
- Enable more verbose logs.
- Use a debug build to allow you to step through `rustc` code.
- Test changes to `rustc`.

We will assume that you already have a Kani setup and that the variable `KANI_WORKSPACE` contains the path to your Kani workspace.

**It's highly recommended that you start from the commit that corresponds to the current `rustc` version from your workspace.**
To get that information, run the following command:
```bash
cd ${KANI_WORKSPACE} # Go to your Kani workspace.
rustc --version # This will print the commit id. Something like:
# rustc 1.60.0-nightly (0c292c966 2022-02-08)
#                       ^^^^^^^^^ this is used as the ${COMMIT_ID} below
# E.g.:
COMMIT_ID=0c292c966
```

First you need to clone and build stage 2 of the compiler.
You should tweak the configuration to satisfy your use case.
For more details, see <https://rustc-dev-guide.rust-lang.org/building/how-to-build-and-run.html> and <https://rustc-dev-guide.rust-lang.org/building/suggested.html>.

```bash
git clone https://github.com/rust-lang/rust.git
cd rust
git checkout ${COMMIT_ID:?"Missing rustc commit id"}
./configure --enable-extended --tools=src,rustfmt,cargo --enable-debug --set=llvm.download-ci-llvm=true
./x.py build -i --stage 2
```

Now create a custom toolchain (here we name it `custom-toolchain`):

```bash
# Use x86_64-apple-darwin for MacOs
rustup toolchain link custom-toolchain build/x86_64-unknown-linux-gnu/stage2
cp build/x86_64-unknown-linux-gnu/stage2-tools-bin/* build/x86_64-unknown-linux-gnu/stage2/bin/
```

Finally, override the current toolchain in your kani workspace and rebuild kani:
```bash
cd ${KANI_WORKSPACE}
rustup override set custom-toolchain
cargo clean
cargo build
```

# Enable `rustc` logs

In order to enable logs, you can just define the `RUSTC_LOG` variable, as documented here: <https://rustc-dev-guide.rust-lang.org/tracing.html>.

Note that depending on the level of logs you would like to enable, you'll need to build your own version of `rustc` as described above.
