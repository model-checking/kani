# Nix support

The repository's `flake.nix` provides an overlay and a development shell for Kani on Linux and
macOS. It uses [oxalica/rust-overlay](https://github.com/oxalica/rust-overlay) for the pinned
nightly toolchains. This directory holds the Nix code the flake uses.

It packages Kani releases from their official release bundles, and provides the tools and a
development shell for building Kani from this checkout.

## What it provides

`overlays.default` adds two package sets with the same attributes:

- `kaniPackages`: the tools for this checkout, read from `rust-toolchain.toml` and
  `kani-dependencies`.
- `kaniReleases.<version>` (e.g. `kaniReleases."0.67.0"`) and `kaniReleases.latest`: each
  recorded Kani release, and the tools it was built with.

| Attribute | Description |
|-----------|-------------|
| `version` | The Kani version |
| `kani` | Kani itself, providing `kani` and `cargo-kani` (`kaniReleases` only, see below) |
| `rust-toolchain` | The nightly toolchain that Kani version needs, with its components |
| `rustup-shim` | A fake `RUSTUP_HOME` and a `cargo` that accepts `+<toolchain>` (see below) |
| `cbmc` | CBMC at the version that Kani version needs, built from source |
| `kissat` | Kissat at the version that Kani version needs (`null` if it uses none) |
| `dev-shell` | Shell for building Kani from source, with all of the above except `kani` |

The flake also exposes `packages.<system>.kani`, which is `kaniReleases.latest.kani`:

```bash
nix run .#kani -- --version
```

### `kani`: Kani from its release bundle

`kaniReleases.<version>.kani` is the release bundle that `cargo kani setup` would download into
`~/.kani`, installed into the Nix store instead:

- Its `toolchain` link points at the rust-overlay nightly that release needs, not a rustup one.
  The build fails if that toolchain's `rustc --version` differs from the one recorded in the
  bundle.
- On Linux, the bundled binaries (Kani, and the CBMC and Kissat it ships) are patched with
  `autoPatchelfHook`, so they run on NixOS too. On macOS, they are used as they are.
- `~/.kani`, rustup and `cargo kani setup` are not used.

It uses the CBMC and Kissat shipped in the bundle, not the `cbmc` and `kissat` attributes.

`cargo` looks for subcommands in `~/.cargo/bin` before `PATH` unless `~/.cargo/bin` is on
`PATH`. If you also installed Kani with `cargo install kani-verifier`, `cargo kani` then runs
that one instead. Use `cargo-kani` directly, or append `~/.cargo/bin` to the end of `PATH` in
your shell:

```nix
shellHook = ''
  export PATH="$PATH:''${CARGO_HOME:-$HOME/.cargo}/bin"
'';
```

Each package set can be customized with `overrideScope`, e.g. to add toolchain components:

```nix
pkgs.kaniPackages.overrideScope (final: prev: {
  rust-toolchain = prev.rust-toolchain.override { extensions = [ "rust-analyzer" ]; };
})
```

### Why a rustup shim

Kani's development build assumes rustup:

- `kani-compiler/build.rs` reads `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` to set the rpath to
  `librustc_driver`.
- `kani-driver` runs `cargo +<toolchain>` when it runs from a development checkout.

The shim provides both, backed by the rust-overlay toolchain, so rustup is not needed.

## Usage

Develop Kani from this checkout:

```bash
git submodule update --init charon   # required by kani-compiler
nix develop
cargo build-dev
./scripts/kani file.rs
```

Use the overlay in another flake:

```nix
{
  inputs.kani.url = "github:model-checking/kani";

  outputs = { nixpkgs, kani, ... }: {
    # ...
    pkgs = import nixpkgs {
      system = "x86_64-linux";
      overlays = [ kani.overlays.default ];
    };
    # pkgs.kaniReleases."0.67.0".rust-toolchain, pkgs.kaniPackages.cbmc, ...
  };
}
```

`overlays.default` also applies rust-overlay. If rust-overlay is already applied, use
`overlays.kani` instead.

## Selecting a Kani version

Pick a Kani release with `kaniReleases`. The overlay then selects everything else that release
was built with:

```nix
pkgs.kaniReleases."0.67.0"   # a specific release
pkgs.kaniReleases.latest     # the newest recorded release
pkgs.kaniPackages            # this checkout, which may be ahead of the latest release
```

| Kani | Nightly (from rust-overlay) | CBMC | CaDiCaL (built into CBMC) | Kissat |
|------|-----------------------------|------|---------------------------|--------|
| 0.66.0 | `nightly-2025-11-05` | 6.8.0 | 2.0.0 | 4.0.1 |
| 0.67.0 | `nightly-2025-11-21` | 6.8.0 | 2.0.0 | 4.0.1 |
| 0.68.0 | `nightly-2026-08-21` | 6.11.0 | 3.0.0 | 4.0.1 |

The table shows the releases recorded when this was written. To list the current ones:

```bash
jq -r '.releases | keys[]' nix/versions.json
```

You choose the Kani version, not the nightly. Each Kani release is built against exactly one
nightly, because `kani-compiler` links against that nightly's `librustc_driver`, so the toolchain
always follows the Kani version. Your project's own `rust-toolchain.toml` does not matter: Kani
compiles your crate with its own toolchain. Your code only has to compile with that nightly.

For example, a shell for verifying your project with Kani 0.67.0:

```nix
devShells.x86_64-linux.kani-0_67 = pkgs.mkShell {
  packages = [
    pkgs.kaniReleases."0.67.0".kani
    pkgs.rust-bin.stable.latest.default   # your project's own toolchain, for `cargo`
  ];
};
```

Name such shells with `_` rather than `.` (`kani-0_67`): Nix reads the `.` in `.#kani-0.67` as an
attribute path separator. A complete example with one shell per version is
[kani-nix-example](https://github.com/lalvesl/kani-nix-example).

To work on Kani's own sources with the tools a release was built with, use its `dev-shell`
instead. Extend it with `overrideAttrs` rather than listing it in another shell's `inputsFrom`:
`inputsFrom` copies only the packages, not the `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` variables
the rustup shim needs.

```nix
devShells.x86_64-linux.default = pkgs.kaniReleases."0.67.0".dev-shell.overrideAttrs (old: {
  nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.just ];   # your extra tools
});
```

Each release is a scope like `kaniPackages`, so `overrideScope` works on it too. Selecting a
version that has not been recorded fails with a missing-attribute error; record it first (see
below).

## Recording releases: `versions.json`

`versions.json` maps each recorded Kani release to its toolchain, CBMC and Kissat versions, and
to the hashes of its release bundles for each platform. It also holds the source hashes of those
CBMC and Kissat versions, and of the CaDiCaL version each CBMC version builds with. It is
generated; update it with:

```bash
nix run .#update-versions              # the version in Cargo.toml
nix run .#update-versions -- 0.67.0    # specific versions
```

Each release is read from its git tag (`kani-<version>`), not from the working tree: after a
release, `main` keeps the released version in `Cargo.toml` while its toolchain moves on. The
command also records the hashes of the CBMC and Kissat versions in the working tree's
`kani-dependencies`, which `kaniPackages` needs.

The `nix-versions-update.yml` workflow runs this automatically when a release is published, and
weekly in case it missed one, and opens a pull request with the result. It can also be run by
hand for a given version.

Bundle hashes come from the SHA-256 digests GitHub publishes for release assets, so the bundles
are not downloaded. A bundle without a digest is downloaded to compute its hash. Set
`GITHUB_TOKEN` if you hit GitHub's API rate limit.

A version without a recorded hash fails evaluation with a message saying so, instead of building
the wrong version.
