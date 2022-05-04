This example accompanies Kani's post on CVE-2018-1000657 of the Rust Standard Library.

## Files in `src`

The file `issue44800.rs` is a standalone example that can be used to reproduce the CVE.

The files `cve.rs` and `fixed.rs` are implementations of `VecDeque` using `raw_vec.rs` as the underlying buffer. These are taken as-is from the Standard Library except that `cve.rs` contains the issue (try a `diff` between the two). These are used in `harness.rs` in two separate modules:

  - Harnesses in `cve_proofs` use the `VecDeque` implementation *with* the CVE
  - Harnesses in `fixed_proofs` use the fixed `VecDeque` implementation

Finally the file `abstract_vecdeque.rs` is a standalone abstraction of the `VecDeque` data structure that we use to prove that the `reserve` and `remove` methods maintain the data structure's resource invariant.

## Dependencies

  - Rust edition 2018
  - [Kani](https://model-checking.github.io/kani/getting-started.html)
  - [Valgrind](https://valgrind.org/) (Necessary to reproduce the CVE but not needed for Kani verification. On Ubuntu you should be able to do `sudo apt install valgrind`)

## Reproducing CVE-2018-1000657 (Linux only)

This section is only reproducible on a Linux platform.

The following will rollback to an old version of the Rust toolchain, compile the example and run it using `valgrind`, a binary instrumentation framework that can detect memory issues.

```bash
$ rustup install nightly-2017-09-23
$ rustup override set nightly-2017-09-23
$ rustc issue44800.rs
$ valgrind ./issue44800
```

The output should contain a line indicating an invalid write (the number `234600` is the process ID and will vary):

```
==234600== Invalid write of size 4
```

After you're done, unset the toolchain override:

```bash
$ rustup override unset
```

## Using Kani

### Finding the issue

```bash
$ cargo kani --harness minimal_example_with_cve_should_fail --output-format terse
```

The expected output is a verification failure result, like:

```
VERIFICATION RESULT: 
 ** 1 of 549 failed
Failed Checks: assertion failed: self.head < self.cap()
 File: "vecdeque-cve/src/cve.rs", line 190, in cve::VecDeque::<T, A>::handle_capacity_increase

VERIFICATION:- FAILED
```

This is a `debug_assert` failure in the implementation. This assertion failure blocks Kani from reporting the subsequent memory safety violation. To see Kani report a similar error to `valgrind` we can temporarily disable debug asserts:

```bash
$ RUSTFLAGS='--cfg disable_debug_asserts' cargo kani --harness minimal_example_with_cve_should_fail --output-format terse
```

The expected output is a verification failure result reporting an out-of-bounds pointer:

```
VERIFICATION RESULT: 
 ** 1 of 500 failed
Failed Checks: dereference failure: pointer outside object bounds
 File: "vecdeque-cve/src/cve.rs", line 103, in cve::VecDeque::<T, A>::buffer_write

VERIFICATION:- FAILED
```

Now let's check the fixed version.

```bash
$ cargo kani --harness minimal_example_with_cve_fixed --output-format terse
# expected result: verification success
```

### Bounded results

We can write a symbolic proof harness. However running with Kani, at the moment (2022-05), causes a timeout (you will need to ctrl-c the process). Try passing the flag `--verbose` to see what Kani is doing and where it gets stuck.

```bash
# the following command causes Kani to timeout
$ RUSTFLAGS="--cfg enable_symbolic_example_with_cve_fixed" cargo kani --harness symbolic_example_with_cve_fixed
# expected result: no results due to timeout
```

### Going further

One way forward is to use the ideas of parametricity and abstraction. We use these in `abstract_vecdeque.rs`.

First, let's simulate the issue:

```bash
$ cargo kani --harness abstract_reserve_maintains_invariant_with_cve --output-format terse
# expected result: verification fail
```

This result is analogous to the failure we observe for the harness `minimal_example_with_cve`.

Now let's verify the fixed version:

```bash
$ cargo kani --harness abstract_reserve_maintains_invariant_with_cve_fixed --output-format terse
# expected result: verification success
```

Finally, as a bonus, we've written a similar proof harness for the `remove` method. Try it out:

```bash
$ cargo kani --harness abstract_remove_maintains_invariant --output-format terse
# expected result: verification success
```
