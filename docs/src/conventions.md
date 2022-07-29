# Coding Conventions

## Formatting

We automate most of our formatting preferences. Our CI will run format checkers for PRs and pushes.
These checks are required for merging any PR.

For Rust, we use [rustfmt](https://github.com/rust-lang/rustfmt)
which is configured via the [rustfmt.toml](https://github.com/model-checking/kani/blob/main/rustfmt.toml) file.
We are also in the process of enabling `clippy`.
Because of that, we still have a couple of lints disabled (see [.cargo/config](https://github.com/model-checking/kani/blob/main/.cargo/config.toml) for the updated list).

We also have a bit of C and Python code in our repository.
For C we use `clang-format` and for Python scripts we use `autopep8`.
See [.clang-format](https://githubcom/model-checking/kani/blob/main/.clang-format)
and [pyproject.toml](https://github.com/model-checking/kani/blob/main/scripts/pyproject.toml)
for their configuration.


### Exceptions

We recognize that in some cases, the formatting and lints automation may not be applicable to a specific code.
In those cases, we usually prefer explicitly allowing exceptions by locally disabling the check.
E.g., use `#[allow]` annotation instead of disabling a link on a crate or project level.

### Copyright notice

All source code files begin with a copyright and license notice. If this is a new file, please add the following notice:

```rust
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
```

When modifying a file from another project, please keep their headers as is and append the following notice after them:

```rust
// ... existing licensing headers ...

// Modifications Copyright Kani Contributors
// See GitHub history for details.
```

We also have automated checks for the copyright notice.
There are a few file types where this rule doesn't apply.
You can see that list in the [copyright-exclude](
https://github.com/model-checking/kani/blob/main/scripts/ci/copyright-exclude) file.


## Code for soundness

We are developing Kani to provide assurance that critical Rust components are verifiably free of certain classes of
security and correctness issues.
Thus, it is critical that we provide a verification tool that is sound.
For the class of errors that Kani can verify, we should not produce a “No Error” result if there was in fact an
error in the code being verified, i.e., it has no
“False Negatives”.

Because of that, we caution on the side of correctness.
Any incorrect modeling
that may trigger an unsound analysis that cannot be fixed in the short term should be mitigated.
Here are a few ways how we do that.

### Compilation errors

Make sure to add user-friendly errors for constructs that we can't handle.
For example, Kani cannot handle panic unwind strategy, and it will fail compilation if the crate uses this
configuration.

### Internal compiler errors

Even though this doesn't provide users the best experience, you are encouraged to add checks in the compiler for any
assumptions you make during development.
Those check can be on the form of `assert!()` or `unreachable!()`
statement.
Please provide a meaningful message to help user understand why something failed, and also help us figure out what
went wrong.

We don't formally use any specific formal representation of [function contract](https://en.wikipedia.org/wiki/Design_by_contract),
but whenever possible we do instrument the code with assertions that may represent the function pre- and
post-conditions to ensure we are modeling the user code correctly.

### Verification errors

In cases where Kani fails to model a certain instruction or local construct that doesn't have a global effect,
we encode this failure as a verification error.
I.e., we generate an assertion failure instead of the construct we are modeling using
[`codegen_unimplemented()`](https://github.com/model-checking/kani/blob/f719b565968568335d9be03ef27c5d05bb8fd0b7/kani-compiler/src/codegen_cprover_gotoc/utils/utils.rs#L50).

This will allow users to verify their crate successfully as long as
that construct is not reachable in any harness. If a harness has at least one possible execution path that reaches
such construct, Kani will fail the verification and it will mark all checks with `UNDETERMINED` status.

### Create detailed issues for "TODO" tasks

It is OK to add "TODO" comments as long as they don't compromise user experience or the tool correctness.
When doing so, please create an issue that captures the task.
Add details about the task at hand including any impact to the user.
Finally, add the link to the issue that captures the "TODO" task as part of your comment.

E.g.:
```rust
// TODO: This function assumes type cannot be ZST. Check if that's always the case.
// https://github.com/model-checking/kani/issues/XXXX
assert!(!typ.is_zst(), "Unexpected ZST type");
```

## Performant but readable

We aim at writing code that is performant but also readable and easy to maintain.
Avoid compromising the code quality if the performance gain is not significant.

Here are few tips that can help the readability of your code:

- Sort match arms alphabetically.
- Use concise but meaningful names.