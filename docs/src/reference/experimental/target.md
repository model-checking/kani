# Verifying for another target

By default Kani verifies a crate as it would be compiled for the host: the same pointer width,
endianness, `cfg(target_arch)` and C type sizes. With `--target`, Kani verifies it for another
target triple instead, from the same host. This is useful for code that only compiles for a
platform you do not develop on, such as a kernel's `riscv64` modules on an x86_64 or Apple
Silicon machine.

This is an experimental feature, tracked in [#2402](https://github.com/model-checking/kani/issues/2402).

## Supported targets

A target needs a CBMC machine model in Kani. These are the targets that have one:

| Target | Notes |
|--------|-------|
| `x86_64-unknown-linux-gnu` | |
| `aarch64-unknown-linux-gnu` | |
| `riscv64gc-unknown-linux-gnu` | LP64D data model |
| `x86_64-apple-darwin` | |
| `aarch64-apple-darwin` | |

Any other target is rejected by the compiler with an error that lists these.

## Building the libraries

Kani verifies against its own build of the standard library and of the `kani` crate, compiled for
the target. Release bundles contain these for the host only, so today `--target` needs Kani
built from source ([build from source](../../build-from-source.md)), with each extra target named:

```bash
cargo build-dev --lib-target riscv64gc-unknown-linux-gnu
```

The option may be repeated. Each target's libraries go to `targets/<TRIPLE>/lib/` beside the
host's `lib/`, so one Kani installation can verify for several targets. The rustup target itself
does not need to be installed: the standard library is built from the `rust-src` component.

## Usage

```bash
kani file.rs --target riscv64gc-unknown-linux-gnu -Z unstable-options
cargo kani --target riscv64gc-unknown-linux-gnu -Z unstable-options
```

## Limitations

* `--concrete-playback` is rejected with a non-host `--target`, because the generated test runs
  on the host.
* The `verify-std` subcommand does not support `--target`.
* Inline assembly is unsupported on every target, as it is on the host. Code behind
  `core::arch::asm!` has to be [stubbed](stubbing.md) to be verified.
* Only 64-bit little-endian targets are supported. A 32-bit target needs `goto-cc -m32` and a
  review of Kani's 64-bit assumptions ([#2086](https://github.com/model-checking/kani/issues/2086)).
