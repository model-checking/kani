# Usage

At present, Kani can used in two ways:
 * [On a single file](./kani-single-file.md) with the `kani` command.
 * [On a package](./cargo-kani.md) with the `cargo-kani` command.

Running [Kani on a single file](./kani-single-file.md) is quite useful for small
examples or projects that don't use `cargo`.

However, if you plan to integrate Kani in your projects, the recommended
approach is to use [Kani on a package](./cargo-kani.md) because of its ability
to handle external dependencies.
