Using an implementation of a `Rectangle` we use unit tests, property-based testing and Kani

## Reproducing results locally

### Dependencies

  - Rust edition 2018
  - [Kani](https://model-checking.github.io/kani/getting-started.html)

If you have problems installing Kani then please file an [issue](https://github.com/model-checking/kani/issues/new/choose).

###  Unit testing and proptest

Use `cargo test` to run the unit test and property-based test.

```bash
$ cargo test
# --snip--
running 2 tests
test rectangle::tests::stretched_rectangle_can_hold_original ... ok
test rectangle::proptests::stretched_rectangle_can_hold_original ... ok
```

### Using Kani

Use `cargo kani` to verify the first proof harness `stretched_rectangle_can_hold_original`. As we explain in the post, verification failure is expected.

```bash
$ cargo kani --function stretched_rectangle_can_hold_original --output-format terse
# --snip--
VERIFICATION:- FAILED
```

In order to view a trace (a step-by-step execution of the program) use the `--visualize` flag:

```bash
$ cargo kani --function stretched_rectangle_can_hold_original --output-format terse --visualize
# --snip--
VERIFICATION:- FAILED
# and generates a html report in target/report/html/index.html
```

And open the report in a browser.

After fixing the problem we can re-run Kani (on the proof harness `stretched_rectangle_can_hold_original_fixed`). This time we expect verification success:

```bash
$ cargo kani --function stretched_rectangle_can_hold_original_fixed --output-format terse
# --snip--
VERIFICATION:- SUCCESSFUL
```
