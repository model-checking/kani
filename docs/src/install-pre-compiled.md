# Installing from pre-compiled binaries

This installation option is better suited for Kani users
who don't expect to change the Kani source code.

## Dependencies

The following must already be installed:

* **Python version 3.8 or greater** and the package installer pip.
* Rust installed via `rustup`.
* `ctags` is required for Kani's `--visualize` option to work correctly.

## Installing the latest version

To install the latest version of Kani, run:

```bash
cargo install --locked kani-verifier
cargo-kani setup
```

This will build and place in `~/.cargo/bin` (in a typical environment) the `kani` and `cargo-kani` binaries.
The second step (`cargo-kani setup`) will download the Kani compiler and other necessary dependencies (and place them under `~/.kani/`).


## Installing an older version

```bash
cargo install --lock kani-verifier --version <VERSION>
cargo-kani setup
```

## Next steps

To check your install, you can
[run a basic Kani test](./install-check.md).
If you're learning Kani for the first time, you may be interested in our [tutorial](kani-tutorial.md).
