# CI helpers

Helper scripts and container definitions used by the GitHub Actions workflows in
[`.github/workflows/`](../../.github/workflows). Nothing here is part of a normal
local build.

## `Dockerfile.bundle-release-24-04`

Builds the container image published for each Kani release,
`ghcr.io/model-checking/kani-ubuntu-24.04:<version>` (also tagged `latest`).

It is used by the `Package Docker` job in
[`release.yml`](../../.github/workflows/release.yml), which runs only on pushes of a
`kani-*` tag. That job produces the release artifacts first, then builds this image
with the repository root as the build context — so the root
[`.dockerignore`](../../.dockerignore) is what keeps `.git`, `firecracker/`, and
`target/{debug,release}` out of the uploaded context — and pushes the result to GHCR.

Because it installs Kani *from those artifacts* rather than from source, it cannot be
built from a clean checkout. Two things must already exist in the build context:

| Path in the build context | Produced by |
|---------------------------|-------------|
| `kani-<version>-x86_64-unknown-linux-gnu.tar.gz` | `cargo bundle` |
| `target/package/kani-verifier-<version>/` | `cargo package -p kani-verifier` |

To reproduce the release image locally:

```bash
cargo bundle
cargo package -p kani-verifier
docker build -t kani-ubuntu-24.04 -f scripts/ci/Dockerfile.bundle-release-24-04 .
```

Note that installing a release bundle is *not* tested through Docker. The `TestBundle`
job in `release.yml` does that natively across the macOS and Ubuntu runners it supports.

## Copyright check

`run-copyright-check.sh` runs `copyright_check.py` over every tracked file that does not
match a pattern in `copyright-exclude`, requiring the two-line header described in
[`docs/src/conventions.md`](../../docs/src/conventions.md). It runs in the
`format-check` workflow.

The script uses `xargs -d`, which is GNU-only, so it does not run as-is on macOS.
