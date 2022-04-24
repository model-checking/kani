# Usage on a package

Kani is also integrated with `cargo` and can be invoked from a crate directory as follows:

```bash
cargo kani [<kani-args>]*
```

`cargo kani` supports all `kani` arguments.

`cargo kani` is the recommended approach for using Kani on a project, due to its
ability to handle external dependencies and the option add configurations via the `Cargo.toml` file.

## Configuration

Users can add a default configuration to the `Cargo.toml` file for running harnesses in a package.
Kani will extract any arguments from these sections:
 * `[kani]`
 * `[workspace.metadata.kani]`
 * `[package.metadata.kani]`

For example, say you want to set a loop unwinding bound of `5` for all the harnesses in a package.
You can achieve this by adding the following lines to the package's `Cargo.toml`:

```toml
[package.metadata.kani]
flags = { default-unwind = "5" }
```
