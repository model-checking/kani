# List Command

The `list` subcommand provides an overview of harnesses and contracts in the provided project/file. This is useful for understanding which parts of your codebase have verification coverage and tracking verification progress.

## Usage
For basic usage, run `cargo kani list` or `kani list <FILE>`. The current options are:
- `--format [pretty|markdown|json]`: Choose output format
- `--std`: List harnesses and contracts in the standard library (standalone `kani` only)

The default format is `pretty`, which prints a table to the terminal, e.g:

```
Kani Rust Verifier 0.65.0 (standalone)

Contracts:
+-------+----------+-------------------------------+----------------------------------------------------------------+
|       | Crate    | Function                      | Contract Harnesses (#[kani::proof_for_contract])               |
+=================================================================================================================+
|       | my_crate | example::implementation::bar  | example::verify::check_bar                                     |
|-------+----------+-------------------------------+----------------------------------------------------------------|
|       | my_crate | example::implementation::foo  | example::verify::check_foo_u32, example::verify::check_foo_u64 |
|-------+----------+-------------------------------+----------------------------------------------------------------|
|       | my_crate | example::implementation::func | example::verify::check_func                                    |
|-------+----------+-------------------------------+----------------------------------------------------------------|
|       | my_crate | example::prep::parse          | NONE                                                           |
|-------+----------+-------------------------------+----------------------------------------------------------------|
| Total |          | 4                             | 4                                                              |
+-------+----------+-------------------------------+----------------------------------------------------------------+

Standard Harnesses (#[kani::proof]):
+-------+----------+-------------------------------+
|       | Crate    | Harness                       |
+==================================================+
|       | my_crate | example::verify::check_modify |
|-------+----------+-------------------------------|
|       | my_crate | example::verify::check_new    |
|-------+----------+-------------------------------|
| Total |          | 2                             |
+-------+----------+-------------------------------+
```

The "Contracts" table overviews which functions have contract attributes (`#[requires]`, `#[ensures]`, or `modifies`), and which harnesses exist for those functions.
The "Standard Harnesses" table lists all of the `#[kani::proof]` harnesses found.

The `markdown` and `json` options write the same information to Markdown or JSON files, respectively.

For `--std`, ensure that the provided path points to a local copy of the standard library, e.g. `kani list --std rust/library`. (Compiling the standard library [works differently](https://doc.rust-lang.org/cargo/reference/unstable.html#build-std) than compiling a normal Rust project, hence the separate option).

For a full list of options, run `kani list --help`. 

## Autoharness
Note that by default, this subcommand only detects manual harnesses. The [experimental autoharness feature](./experimental/autoharness.md) accepts a `--list` argument, which runs this subcommand for both manual harnesses and automatically generated harnesses.
