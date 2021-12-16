This is a temporary wrapper that can be used to compiler rust into gotoc. This
binary should not be used on its own and it should be used via `rmc` or
`cargo-rmc` commands.

### Notes for developers:

To build / install:

```
cargo build
cargo install --path <project_path> --root <install_path>
```

To run:

```
cargo run
```

or 

```
LD_LIBRARY_PATH=<path_to_rustc_lib> <install_path>/rmc_compiler
```

