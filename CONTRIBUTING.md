# Reporting Bugs and Asking Questions

Bug reports and questions are always welcome, though before asking whether a
feature exists, please glance through the [Proptest
book](https://altsysrq.github.io/proptest-book/intro.html) and try the search
feature built in to the [API
docs](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/) to see if
they answer your question already.

Please include your rust version and proptest version in any reports. Also
include your operating system if you think it is relevant.

# Requesting New Features

Please be specific in feature requests and, as mentioned above, try to see
whether feature does not already exist before requesting it.

There are no hard rules as to what features will or will not be accepted —
ultimately, it depends on what the expected benefit is relative to the expected
maintenance burden. However, here are some general guidelines.

The following will usually be accepted as new features:

- New widely-applicable utilities for generating values. The `proptest::sample`
  module is an example of such an accepted feature.

- Features which enable Proptest to be used in a context where it formerly was
  not. `no_std` is an example.

- Quality of life improvements, such as adding new forms for the macros or
  adding convenience functions.

The following will usually _not_ be accepted as new features (but still might
be if there is a sufficiently compelling reason):

- Direct integration with a third-party crate, in the sense of adding
  strategies or `Abitrary` implementation for that crate's types.

- Features which are too narrowly applicable. (Hypothetical example: a built-in
  strategy to generate valid streams of x86 instructions.)

- Features which are easier to mis-use than to use correctly, or which could
  lead to misunderstanding of how proptest is supposed to be used. (Example:
  adding a method directly on `Strategy` to generate values from it.)

Proptest is maintained on a volunteer basis. If your feature is large, consider
implementing it yourself as a pull request. (Feel free of course to open an
issue first for guidance.)

# Pull Requests

Pull requests which fix bugs or add features are welcome. Below are some
guidelines to keep in mind.

## Formatting

Code is formatted with `rustfmt`. If you are using `rustup`, you can install it
with `rustup component add rustfmt`.

Other files are wrapped to 80 columns where reasonably possible use Unix line
endings. An exception is the `run-tests.bat` file which must use DOS line
endings.

## Copyright Headers

All source files start with the copyright header whose template is found at
`proptest/src/file-preamble`.

When creating a new source file, insert the template and fill it out with the
current year and either your name or "The proptest developers", at your choice.

When making non-trivial changes to an existing file, update the header to _add_
the current year (separated from the previous by a comma) if not already there,
and change the name to "The proptest developers" if the current name is neither
that nor your own. Use your best judgement for what "non-trivial" is.

You retain copyright for any code you add to proptest.

## Updating the changelog

If you make a change which is observable to proptest users (i.e. almost
anything other than documentation changes or test code), add a note to the
`CHANGELOG.md` for the crate.

If there is not already an `## Unreleased` section at the top, create one.
Under the Unreleased version, add a bullet point describing the change to one
of the following subsections (ordered as follows):

- Breaking Changes
- Deprecations
- Bug Fixes
- New Additions
- Nightly-only Breakage
- Other Notes

## Development Notes

### Rust Version

It is generally easiest to work on proptest with the current stable Rust
version. Do keep in mind though that proptest retains compatibility with an
older version of Rust; you can find this in `.travis.yml`. Your pull request
will automatically be checked against that Rust version so you don't need to
worry about it too much unless that fails.

You need to use the latest nightly for the following:

- Working on nightly-only proptest features, including `no_std` support.

- Testing `proptest-derive`.

### Coding

Within the `proptest` crate, you cannot refer to `std` outside of test code or
code which is gated around the `std` feature flag. Instead, `std` names must be
pulled from the `std_facade` module in Proptest.

### Running Tests

To test the `proptest` crate, simply run `cargo test -p proptest`. If you are
working on something not in the default proptest feature set, refer to
`.travis.yml` for examples of how to test those features.

The test code for `proptest` has not been updated to work on `no_std`. Since
`no_std` does not add novel code, we currently only test that it compiles.

Testing the `proptest-derive` crate currently requires nightly. Assuming you
are using `rustup`, you can run its tests with
`cargo +nightly test -p proptest-derive`. The tests can fail with mysterious
errors if the `proptest` crate was previously built with a different
configuration or a different Rust version. If you get build failures that don't
seem to make sense, try running `cargo clean` and then try again.

Tests for test case persistence are not run as part of `cargo test`. To run
them on Unix, run `./run-tests.sh` in `proptest/test-persistence-location`. To
run them on Windows, run `run-tests.bat` in
`proptest\test-persistence-location`.

## Automated Pull Request Checks

Your pull request will automatically be tested against a range of Rust versions
and configurations, including both on Linux and Windows.

Generally, please try to address any failures of these tests yourself if you
are able. There are however a couple classes of problems you do not need to
concern yourself with:

- If a nightly-only feature has been broken by a change in the nightly
  distribution of Rust, do not worry about fixing it in your PR (unless of
  course that is the specific aim of your PR).

- Certain tests are known to occasionally fail spuriously. This most commonly
  happens on a time-sensitive test that can fail under the low-performance
  Appveyor environment. Certain tests have also generated random data and then
  made assertions about the results; these should all be deterministic now but
  some may have been missed.
