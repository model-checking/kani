# Working with `rustc`

Kani is developed on the top of the Rust compiler, which is not distributed on [crates.io](https://crates.io/) and depends on
bootstrapping mechanisms to properly build its components.
Thus, our dependency on `rustc` crates are not declared in our `Cargo.toml`.

Below are a few hacks that will make it easier to develop on the top of `rustc`.

## Code analysis for `rustc` definitions

IDEs rely on `cargo` to find dependencies and sources to provide proper code analysis and code completion.
In order to get these features working for `rustc` crates, you can do the following:

### VSCode

Add the following to the `rust-analyzer` extension settings in `settings.json`:
```json
    "rust-analyzer.rustc.source": "discover",
    "rust-analyzer.workspace.symbol.search.scope": "workspace_and_dependencies",
```

Ensure that any packages that use `rustc` data structures have the following line set in their `Cargo.toml`

```toml
[package.metadata.rust-analyzer]
# This package uses rustc crates.
rustc_private=true
```

You may also need to install the `rustc-dev` package using rustup

```
rustup toolchain install nightly --component rustc-dev
```

#### Debugging in VS code

To debug Kani in VS code, first install the [CodeLLDB extension](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb).
Then add the following lines at the start of the `main` function (see [the CodeLLDB manual](https://github.com/vadimcn/vscode-lldb/blob/master/MANUAL.md#attaching-debugger-to-the-current-process-rust) for details):

```rust
{
    let url = format!(
        "vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','sourceLanguages':['rust'],'waitFor':true,'pid':{}}}",
        std::process::id()
    );
    std::process::Command::new("code").arg("--open-url").arg(url).output().unwrap();
}
```

Note that pretty printing for the Rust nightly toolchain (which Kani uses) is not very good as of June 2022.
For example, a vector may be displayed as `vec![{...}, {...}]` on nightly Rust, when it would be displayed as `vec![Some(0), None]` on stable Rust.
Hopefully, this will be fixed soon.

### RustRover / CLion
This is not a great solution, but it works for now (see <https://github.com/intellij-rust/intellij-rust/issues/1618>
for more details).

Open the `Cargo.toml` of your crate (e.g.: `kani-compiler`), and do the following:

1. Add optional dependencies on the `rustc` crates you are using.
2. Add a feature that enable those dependencies.
3. Toggle that feature using the IDE GUI.

Here is an example:

```toml
# ** At the bottom of the dependencies section: **
# Adjust the path here to point to a local copy of the rust compiler.
# E.g.: ~/.rustup/toolchains/<toolchain>/lib/rustlib/rustc-src/rust/compiler
rustc_smir = { path = "<path_to_rustc>/rustc_smir", optional = true }
stable_mir = { path = "<path_to_rustc>/stable_mir", optional = true }

[features]
clion = ['rustc_smir', 'stable_mir']
```

**Don't forget to rollback the changes before you create your PR.**

### EMACS (with `use-package`)
First, `Cargo.toml` and `rustup toolchain` steps are identical to VS
Code. Install Rust-analyzer binary under `~/.cargo/bin/`.

On EMACS, add the following to your EMACS lisp files. They will
install the necessary packages using the `use-package` manager.
```elisp
;; Install LSP
(use-package lsp-mode
  :commands lsp)
(use-package lsp-ui)

;; Install Rust mode
(use-package toml-mode)
(use-package rust-mode)

(setq lsp-rust-server 'rust-analyzer)
(setenv "PATH" (concat (getenv "PATH") ":/home/USER/.cargo/bin/"))
```
If EMACS complains that it cannot find certain packages, try running
`M-x package-refresh-contents`.

For LSP to be able to find `rustc_private` files used by Kani, you
will need to modify variable `lsp-rust-analyzer-rustc-source`. Run
`M-x customize-variable`, type in `lsp-rust-analyzer-rustc-source`,
click `Value Menu` and change it to `Path`. Paste in the path to
`Cargo.toml` of `rustc`'s source code. You can find the source code
under `.rustup`, and the path should end with
`compiler/rustc/Cargo.toml`. **Important**: make sure that this
`rustc` is the same version and architecture as what Kani uses. If
not, LSP features like definition lookup may be break.

This ends the basic install for EMACS. You can test your configuration
with the following steps.
1. Opening up a rust file with at least one `rustc_private` import.
2. Activate LSP mode with `M-x lsp`.
3. When asked about the root of the project, pick one of them. **Make
   sure** that whichever root you pick has a `Cargo.toml` with
   `rustc_private=true` added.
4. If LSP asks if you want to watch all files, select yes. For less
   powerful machines, you may want to adjust that later.
5. On the file with `rustc_private` imports, do the following. If both
   work, then you are set up.
   - Hover mouse over the `rustc_private` import. If LSP is working,
	 you should get information about the imported item.
   - With text cursor over the same `rustc_private` import, run `M-x
     lsp-find-definition`. This should jump to the definition within
     `rustc`'s source code.

LSP mode can integrate with `flycheck` for instant error checking and
`company` for auto-complete. Consider adding the following to the
configuration.
```elisp
(use-package flycheck
  :hook (prog-mode . flycheck-mode))

(use-package company
  :hook (prog-mode . company-mode)
  :config
   (global-company-mode))
```

`clippy` linter can be added by changing the LSP install to:
```elisp
(use-package lsp-mode
  :commands lsp
  :custom
  (lsp-rust-analyzer-cargo-watch-command "clippy"))
```

Finally lsp-mode can run rust-analyzer via TRAMP for remote
development. **We found this way of using rust-analyzer to be unstable
as of 2022-06**. If you want to give it a try you will need to add a
new LSP client for that remote with the following code.
```elisp
(lsp-register-client
  (make-lsp-client
	:new-connection (lsp-tramp-connection "/full/path/to/remote/machines/rust-analyzer")
	:major-modes '(rust-mode)
	:remote? t
	:server-id 'rust-analyzer-remote))
```

For further details, please see https://emacs-lsp.github.io/lsp-mode/page/remote/.

## Custom `rustc`

There are a few reasons why you may want to use your own copy of `rustc`. E.g.:
- Enable more verbose logs.
- Use a debug build to allow you to step through `rustc` code.
- Test changes to `rustc`.

We will assume that you already have a Kani setup and that the variable `KANI_WORKSPACE` contains the path to your Kani workspace.

**It's highly recommended that you start from the commit that corresponds to the current `rustc` version from your workspace.**
To get that information, run the following command:
```bash
cd ${KANI_WORKSPACE} # Go to your Kani workspace.
rustc --version # This will print the commit id. Something like:
# rustc 1.60.0-nightly (0c292c966 2022-02-08)
#                       ^^^^^^^^^ this is used as the ${COMMIT_ID} below
# E.g.:
COMMIT_ID=0c292c966
```

First you need to clone and build stage 2 of the compiler.
You should tweak the configuration to satisfy your use case.
For more details, see <https://rustc-dev-guide.rust-lang.org/building/how-to-build-and-run.html> and <https://rustc-dev-guide.rust-lang.org/building/suggested.html>.

```bash
git clone https://github.com/rust-lang/rust.git
cd rust
git checkout ${COMMIT_ID:?"Missing rustc commit id"}
./configure --enable-extended --tools=src,rustfmt,cargo --enable-debug --set=llvm.download-ci-llvm=true
./x.py build -i --stage 2
```

Now create a custom toolchain (here we name it `custom-toolchain`):

```bash
# Use x86_64-apple-darwin for MacOs
rustup toolchain link custom-toolchain build/x86_64-unknown-linux-gnu/stage2
cp build/x86_64-unknown-linux-gnu/stage2-tools-bin/* build/x86_64-unknown-linux-gnu/stage2/bin/
```

Finally, override the current toolchain in your kani workspace and rebuild kani:
```bash
cd ${KANI_WORKSPACE}
rustup override set custom-toolchain
cargo clean
cargo build-dev
```
# Rust compiler utilities to debug `kani-compiler`

## Enable `rustc` logs

In order to enable logs, you can just define the `RUSTC_LOG` variable, as documented here: <https://rustc-dev-guide.rust-lang.org/tracing.html>.

Note that, depending on the level of logs you would like to get (debug and trace are not enabled by default), you'll need to build your own version of `rustc` as described above.
For logs that are related to `kani-compiler` code, use the `KANI_LOG` variable.

## Debugging type layout

In order to print the type layout computed by the Rust compiler, you can pass the following flag to `rustc`: `-Zprint-type-sizes`.
This flag can be passed to `kani` or `cargo kani` by setting the `RUSTFLAG` environment variable.

```
RUSTFLAGS=-Zprint-type-sizes kani test.rs
```

When enabled, the compiler will print messages that look like:

```
print-type-size type: `std::option::Option<bool>`: 1 bytes, alignment: 1 bytes
print-type-size     variant `Some`: 1 bytes
print-type-size         field `.0`: 1 bytes
print-type-size     variant `None`: 0 bytes
```

## Inspecting the MIR

You can easily visualize the MIR that is used as an input to code generation by setting the Rust flag `--emit mir`. I.e.:

```
RUSTFLAGS=--emit=mir kani test.rs
```

The compiler will generate a few files, but we recommend looking at the files that have the following suffix: `kani.mir`.
Those files will include the entire MIR collected by our reachability analysis.
It will include functions from all dependencies, including the `std` library.
One limitation is that we dump one copy of each specialization of the MIR function, even though the MIR body itself doesn't change.
