# Kani AI Assistant Guidelines

This document provides guidance for AI coding assistants (Kiro, Claude Code, Copilot, etc.) when working with the Kani codebase.

## Project Overview

**Kani** is a bit-precise model checker for Rust. It verifies both safety and correctness of Rust code by:

- **Safety**: Automatically checking for undefined behavior, making it particularly useful for verifying `unsafe` code blocks
- **Correctness**: Checking for panics (e.g., `unwrap()` on `None`), arithmetic overflows, and custom correctness properties via assertions or function contracts

Kani uses [CBMC](https://github.com/diffblue/cbmc) as its underlying verification engine to provide bit-precise verification of Rust programs.

## Repository Structure

### Core Components

| Directory | Description |
|-----------|-------------|
| `kani-compiler/` | Rust compiler plugin that translates Rust MIR to CBMC's goto-program format |
| `kani-driver/` | CLI driver that orchestrates the verification workflow (invokes compiler, CBMC, processes results) |
| `cprover_bindings/` | Rust bindings for CBMC's internal representation (goto-program, symbols, types) |
| `kani_metadata/` | Data structures for metadata shared between compiler and driver |

### Library & API

| Directory | Description |
|-----------|-------------|
| `library/kani/` | Main Kani library providing the verification API (`kani::any()`, `kani::assume()`, etc.) |
| `library/kani_core/` | Core library with fundamental Kani definitions |
| `library/kani_macros/` | Procedural macros (`#[kani::proof]`, `#[kani::requires]`, etc.) |
| `library/std/` | Standard library overrides for verification |

### Testing & Quality

| Directory | Description |
|-----------|-------------|
| `tests/` | Comprehensive test suites (see [Testing](#testing) section) |
| `scripts/` | Build, test, and CI scripts |
| `tools/` | Supporting tools (compiletest, benchcomp, kani-cov, etc.) |

### Documentation & Resources

| Directory | Description |
|-----------|-------------|
| `docs/` | User and developer documentation (built with mdBook) |
| `rfc/` | Request for Comments documents for design decisions |
| `papers/` | Academic papers related to Kani |

## Build and Development Commands

### Building Kani

```bash
# Development build (debug mode)
cargo build-dev

# Release build (with optimizations)
cargo build-dev -- --release

# To build Kani with both the default CPROVER and the experimental LLBC back-end
# enabled, use:
cargo build-dev -- --features cprover --features llbc

# Clean build artifacts if encountering stale cache issues
cargo clean
cargo build-dev
```

### Running Tests

```bash
# Full regression suite
./scripts/kani-regression.sh
# To run the LLBC-specific regression tests:
./scripts/kani-llbc-regression.sh

# Run a specific test suite
cargo run -p compiletest -- --suite kani --mode kani
cargo run -p compiletest -- --suite expected --mode expected
cargo run -p compiletest -- --suite cargo-kani --mode cargo-kani

# Unit tests for specific packages
cargo test -p cprover_bindings
cargo test -p kani-compiler
cargo test -p kani-driver
cargo test -p kani_metadata
cargo test -p kani --features concrete_playback
```

### Running Kani

```bash
# Verify a single Rust file
kani file.rs

# Verify a Cargo project
cargo kani

# Useful debugging flags
kani --debug file.rs                    # Enable debug logging
kani --keep-temps file.rs               # Keep intermediate files
kani --gen-c file.rs                    # Generate C code from CBMC IR
KANI_LOG="kani_compiler::kani_middle=trace" kani file.rs  # Fine-grained logging
```

### Code Formatting

```bash
# Check formatting
./scripts/kani-fmt.sh --check

# Auto-format code
./scripts/kani-fmt.sh
```

## Testing

### Test Suites Overview

| Suite | Mode | Description |
|-------|------|-------------|
| `kani` | kani | Main test suite; single Rust files verified with Kani |
| `expected` | expected | Tests with expected output verification (`.expected` files) |
| `ui` | expected | User interface tests (warnings, error messages) |
| `cargo-kani` | cargo-kani | Tests for `cargo kani` command with full Cargo projects |
| `cargo-ui` | cargo-kani | Cargo-based UI tests |
| `firecracker` | kani | Tests inspired by Firecracker codebase |
| `prusti` | kani | Tests from Prusti verifier |
| `smack` | kani | Tests from SMACK verifier |
| `kani-fixme` | kani-fixme | Known failing tests (marked with `fixme` or `ignore`) |
| `script-based-pre` | exec | Script-based tests with custom setup |
| `coverage` | coverage-based | Coverage verification tests |

### Writing Tests

**Single-file tests** (in `tests/kani/`):

```rust
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --default-unwind 4  (optional command-line flags)

#[kani::proof]
fn check_something() {
    let x: u32 = kani::any();
    kani::assume(x < 100);
    assert!(x * 2 < 200);
}
```

**Expected output tests** (in `tests/expected/`):
- Create a `.rs` file with the test
- Create a corresponding `.expected` file with expected output patterns

**"Fixme" tests**: Name files with `fixme` or `ignore` to mark known-failing tests that demonstrate bugs or unsupported features.

### Test Naming Conventions

- Use descriptive names that indicate what's being tested
- Include issue numbers in test file names or comments when applicable
- For regression tests, include the GitHub issue number (e.g., `issue_1234.rs`)

## Key Concepts

### Proof Harnesses

Proof harnesses are the entry points for verification, similar to test functions:

```rust
#[kani::proof]
fn check_property() {
    // Create nondeterministic inputs
    let input: u8 = kani::any();
    
    // Add assumptions to constrain inputs
    kani::assume(input > 0);
    
    // Call the function under verification
    let result = function_under_test(input);
    
    // Check properties with assertions
    assert!(result > 0);
}
```

### Core Kani API

| Function/Macro | Purpose |
|----------------|---------|
| `kani::any()` | Generate nondeterministic value of any type implementing `Arbitrary` |
| `kani::assume(cond)` | Restrict verification to paths where `cond` is true |
| `assert!(cond)` | Verify that `cond` holds on all paths |
| `kani::cover!(cond)` | Check if `cond` is reachable |

### Function Contracts

Contracts allow modular verification by specifying function behavior:

```rust
#[kani::requires(x > 0)]                    // Precondition
#[kani::ensures(|result| *result > x)]      // Postcondition
#[kani::modifies(&mut state)]               // Memory modification specification
fn increment(x: u32) -> u32 {
    x + 1
}

// Verify the contract
#[kani::proof_for_contract(increment)]
fn check_increment() {
    let x: u32 = kani::any();
    increment(x);
}

// Use verified contract as a stub
#[kani::proof]
#[kani::stub_verified(increment)]
fn check_caller() {
    let result = increment(5);
    assert!(result == 6);
}
```

### Common Attributes

| Attribute | Description |
|-----------|-------------|
| `#[kani::proof]` | Marks a function as a proof harness |
| `#[kani::unwind(N)]` | Set loop unwinding bound |
| `#[kani::should_panic]` | Expect the harness to panic |
| `#[kani::solver(solver)]` | Specify SAT solver (minisat, cadical, kissat, z3, etc.) |
| `#[kani::stub(orig, repl)]` | Replace function with stub |
| `#[kani::requires(cond)]` | Function precondition |
| `#[kani::ensures(cond)]` | Function postcondition |
| `#[kani::modifies(ptr)]` | Specify modified memory |
| `#[kani::proof_for_contract(fn)]` | Verify a function's contract |
| `#[kani::stub_verified(fn)]` | Use verified contract as stub |

## Code Style and Conventions

### General Guidelines

- Follow the existing code style (enforced by `rustfmt` with `rustfmt.toml`)
- Use Clippy lints (some are allowed, see `.cargo/config.toml`)
- C code uses `clang-format`, Python uses `autopep8`

### Copyright Notice

All new source files must begin with:

```rust
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
```

### Code Quality Principles

1. **Soundness over convenience**: Kani must never produce false negatives (saying code is safe when it isn't)
2. **Clear error messages**: Follow [rustc diagnostic guidelines](https://rustc-dev-guide.rust-lang.org/diagnostics.html)
3. **Explicit over implicit**: Use `#[allow]` annotations locally rather than disabling lints globally
4. **Document TODOs**: Create GitHub issues for TODO items and reference them in comments

### Code Organization

- Sort match arms, enum variants, and struct fields alphabetically
- Prefer exhaustive matches over wildcard patterns
- Prefer declarative over imperative programming
- Keep names concise but meaningful

## Development Best Practices

### Before Implementing Bug Fixes

1. Verify if the bug still exists by creating a regression test using the reported code sample
2. If the bug no longer reproduces, add the test case and create a PR that resolves the issue

### Adding New Features

1. Check if there's an existing RFC or issue discussing the feature
2. For significant changes, open an issue first to discuss the approach
3. Add comprehensive tests covering the new functionality
4. Update documentation as needed

### Pull Request Guidelines

1. Work against the latest `main` branch
2. Focus on one specific change per PR
3. Ensure local tests pass (`./scripts/kani-regression.sh`)
4. Use clear commit messages (PRs are squash-merged)
5. Respond to CI failures and review feedback

## Additional Resources

- [User Documentation](https://model-checking.github.io/kani/)
- [Developer Documentation](https://model-checking.github.io/kani/dev-documentation.html)
- [Coding Conventions](https://model-checking.github.io/kani/conventions.html)
- [Regression Testing Guide](https://model-checking.github.io/kani/regression-testing.html)
- [CONTRIBUTING.md](./CONTRIBUTING.md)
