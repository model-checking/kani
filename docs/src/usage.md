# Usage

At present, Kani can used in two ways:
 * [On a single file](./kani-single-file.md) with the `kani` command.
 * [On a package](./cargo-kani.md) with the `cargo-kani` command.

Running [Kani on a single file](./kani-single-file.md) is quite useful for small examples.

However, if you plan to integrate Kani in your projects, the recommended
approach is to use [Kani on a package](./cargo-kani.md) because it includes
utilities for tagging, extracting and running all proof harnesses in your
package without making changes to its codebase.
