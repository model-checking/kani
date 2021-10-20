# RMC developer documentation

## Build command cheat sheet

```bash
# Normal build
./x.py build -i --stage 1 library/std
```
```bash
# Full regression suite
./scripts/rmc-regression.sh
```
```bash
# Test suite run (to run a specific suite from src/test/, just remove the others)
./x.py test -i --stage 1 rmc firecracker prusti smack expected cargo-rmc
```
```bash
# Dashboard run
./scripts/setup/install_dashboard_deps.sh
./x.py run -i --stage 1 dashboard
```
```bash
# Documentation build
cd rmc-docs
./build-docs.sh 
```

### Resolving development issues

```bash
# "error[E0514]: found crate `std` compiled by an incompatible version of rustc"
# or similar error? Clean build RMC:
rm -rf target build
./x.py build -i --stage 1 library/std
```
```bash
# firecracker build problem with rmc-regression? Probably a similar issue:
rm -rf firecracker/build
```

## Git command cheat sheet

RMC follows the "squash and merge pull request" pattern.
As a result, the "main commit message" will be the title of your pull request.
The individual commit message bodies you commit during development will by default be a bulleted list in the squashed commit message body, but these are editable at merge time.
So you don't have to worry about a series of "oops typo fix" messages while fixing up your pull request, these can be edited out of the final message when you click merge.

```bash
# Set up your git fork
git remote add fork git@github.com:${USER}/rmc.git
```
```bash
# Reset everything. Don't have any uncommitted changes!
git clean -xffd
git submodule foreach --recursive git clean -xffd
git submodule update --init
# Don't forget to re-configure your RMC build:
./configure \
    --enable-debug \
    --set=llvm.download-ci-llvm=true \
    --set=rust.debug-assertions-std=false \
    --set=rust.deny-warnings=false
```
```bash
# Done with that PR, time for a new one?
git switch main
git pull origin
git submodule update --init
./x.py build -i --stage 1 library/std
```
```bash
# Need to update local branch (e.g. for an open pull request?)
git fetch origin
git merge origin/main
```
```bash
# Search only git-tracked files
git grep codegen_panic
```
```bash
# See all commits that are part of RMC, not part of Rust
git log --graph --oneline origin/upstream-rustc..origin/main
```
```bash
# See all files modified by RMC (compared to upstream Rust)
git diff --stat origin/upstream-rustc..origin/main
```

## RMC command cheat sheet

These can help understand what RMC is generating or encountering on an example or test file:

```bash
# Enable `debug!` macro logging output when running RMC:
rmc --debug file.rs
```
```bash
# Keep CBMC Symbol Table and Goto-C output (.json and .goto)
rmc --keep-temps file.rs
```
```bash
# Generate "C code" from CBMC IR:
rmc --gen-c file.rs
```

## CBMC command cheat sheet

```bash
# See CBMC IR from a C file:
goto-cc file.c -o file.out
goto-instrument --print-internal-representation file.out
# or (for json symbol table)
cbmc --show-symbol-table --json-ui file.out
```
```bash
# Recover C from goto-c binary
goto-instrument --dump-c file.out > file.gen.c
```
