This repo contains contains a minimal example that breaks compilation when using [kani](https://github.com/model-checking/kani), where I would expect compilation to work.

Deleting the `binary/build.rs` script makes the compilation work suddenly, despite it being skipped anyways:

```
binary$ cargo kani -v
Kani Rust Verifier 0.48.0 (cargo plugin)
Skipped the following unsupported targets: 'build-script-build'.
...
```
