# Command cheat sheets

Development work in the Kani project depends on multiple tools. Regardless of
your familiarity with the project, the commands below may be useful for
development purposes.

## Kani

### Build

```bash
# Error "'rustc' panicked at 'failed to lookup `SourceFile` in new context'"
# or similar error? Cleaning artifacts might help.
# Otherwise, comment the line below.
cargo clean
cargo build-dev
```

### Test

```bash
# Full regression suite
./scripts/kani-regression.sh
```

```bash
# Delete regression test caches (Linux)
rm -r build/x86_64-unknown-linux-gnu/tests/
```

```bash
# Delete regression test caches (macOS)
rm -r build/x86_64-apple-darwin/tests/
```

```bash
# Test suite run (we can only run one at a time)
# cargo run -p compiletest -- --suite ${suite} --mode ${mode}
cargo run -p compiletest -- --suite kani --mode kani
```

```bash
# Build documentation
cd docs
./build-docs.sh
```

### Debug

These can help understand what Kani is generating or encountering on an example or test file:

```bash
# Enable `debug!` macro logging output when running Kani:
kani --debug file.rs
```

```bash
# Use KANI_LOG for a finer grain control of the source and verbosity of logs.
# E.g.: The command below will print all logs from the kani_middle module.
KANI_LOG="kani_compiler::kani_middle=trace" kani file.rs
```

```bash
# Keep CBMC Symbol Table and Goto-C output (.json and .goto)
kani --keep-temps file.rs
```
```bash
# Generate "C code" from CBMC IR (.c)
kani --gen-c file.rs
```

```bash
# Generate a ${INPUT}.kani.mir file with a human friendly MIR dump
# for all items that are compiled to the respective goto-program.
RUSTFLAGS="--emit mir" kani ${INPUT}.rs
```

The `KANI_REACH_DEBUG` environment variable can be used to debug Kani's reachability analysis.
If defined, Kani will generate a DOT graph `${INPUT}.dot` with the graph traversed during reachability analysis.
If defined and not empty, the graph will be filtered to end at functions that contains the substring
from `KANI_REACH_DEBUG`.

Note that this will only work on debug builds.

```bash
# Generate a DOT graph ${INPUT}.dot with the graph traversed during reachability analysis
KANI_REACH_DEBUG= kani ${INPUT}.rs

# Generate a DOT graph ${INPUT}.dot with the sub-graph traversed during the reachability analysis
# that connect to the given target.
KANI_REACH_DEBUG="${TARGET_ITEM}" kani ${INPUT}.rs
```

## CBMC

```bash
# See CBMC IR from a C file:
goto-cc file.c -o file.out
goto-instrument --print-internal-representation file.out
# or (for json symbol table)
cbmc --show-symbol-table --json-ui file.out
# or (an alternative concise format)
cbmc --show-goto-functions file.out
```
```bash
# Recover C from goto-c binary
goto-instrument --dump-c file.out > file.gen.c
```

## Git

The Kani project follows the [squash and merge option](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/about-pull-request-merges#squash-and-merge-your-pull-request-commits) for pull request merges.
As a result:
 1. The title of your pull request will become the main commit message.
 2. The messages from commits in your pull request will appear by default as a bulleted list in the main commit message body.

But the main commit message body is editable at merge time, so you don't have to worry about "typo fix" messages because these can be removed before merging.

```bash
# Set up your git fork
git remote add fork git@github.com:${USER}/kani.git
```

```bash
# Reset everything. Don't have any uncommitted changes!
git clean -xffd
git submodule foreach --recursive git clean -xffd
git submodule update --init
```

```bash
# Need to update local branch (e.g. for an open pull request?)
git fetch origin
git merge origin/main
# Or rebase, but that requires a force push,
# and because we squash and merge, an extra merge commit in a PR doesn't hurt.
```

```bash
# Checkout a pull request locally without the github cli
git fetch origin pull/$ID/head:pr/$ID
git switch pr/$ID
```

```bash
# Push to someone else's pull request
git origin add $USER $GIR_URL_FOR_THAT_USER
git push $USER $LOCAL_BRANCH:$THEIR_PR_BRANCH_NAME
```

```bash
# Search only git-tracked files
git grep codegen_panic
```

```bash
# Accidentally commit to main?
# "Move" commit to a branch:
git checkout -b my_branch
# Fix main:
git branch --force main origin/main
```
