- **Feature Name:** Cover statement (`cover-statement`)
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/696>
- **RFC PR:** <https://github.com/model-checking/kani/pull/1906>
- **Status:** Unstable
- **Version:** 1

-------------------

## Summary

A new Kani API that allows users to check that a certain condition can occur at a specific location in the code.

## User Impact

Users typically want to gain confidence that a proof checks what it is supposed to check, i.e. that properties are not passing vacuously due to an over-constrained environment.

A new Kani macro, `cover` will be created that can be used for checking that a certain condition _can_ occur at a specific location in the code.
The purpose of the macro is to verify, for example, that assumptions are not ruling out those conditions, e.g.:
```rust
let mut v: Vec<i32> = Vec::new();
let len: usize = kani::any();
kani::assume(len < 5);
for _i in 0..len {
    v.push(kani::any());
}
// make sure we can get a vector of length 5
kani::cover!(v.len() == 5);
```
This is typically used to ensure that verified checks are not passing _vacuously_, e.g. due to overconstrained pre-conditions.

The special case of verifying that a certain line of code is reachable can be achieved using `kani::cover!()` (which is equivalent to `cover!(true)`), e.g.
```rust
match x {
    val_1 => ...,
    val_2 => ...,
    ...
    val_i => kani::cover!(), // verify that `x` can take the value `val_i`
}
```

Similar to Rust's `assert` macro, a custom message can be specified, e.g.
```rust
kani::cover!(x > y, "x can be greater than y");
```

## User Experience

The `cover` macro instructs Kani to find _at least one_ possible execution that satisfies the specified condition at that line of code.  If no such execution is possible, the check is reported as *unsatisfiable*.

Each cover statement will be reported as a check whose description is `cover condition: cond` and whose status is:
- `SATISFIED` (green): if Kani found an execution that satisfies the condition.
- `UNSATISFIABLE` (yellow): if Kani proved that the condition cannot be satisfied.
- `UNREACHABLE` (yellow): if Kani proved that the cover statement itself cannot be reached.

For example, for the following `cover` statement:
```rust
kani::cover!(a == 0);
```
An example result is:
```
Check 2: main.cover.2
         - Status: SATISFIED
         - Description: "cover condition: a == 0"
         - Location: foo.rs:9:5 in function main
```

### Impact on Overall Verification Status

By default, unsatisfiable and unreachable `cover` checks will not impact verification success or failure.
This is to avoid getting verification failure for harnesses for which a `cover` check is not relevant.
For example, on the following program, verification should not fail for `another_harness_that_doesnt_call_foo` because the `cover` statement in `foo` is unreachable from it.
```rust
[kani::proof]
fn a_harness_that_calls_foo() {
    foo();
}

#[kani::proof]
fn another_harness_that_doesnt_call_foo() {
    // ...
}

fn foo() {
    kani::cover!( /* some condition */);
}
```

The `--fail-uncoverable` option will allow users to fail the verification if a cover property is unsatisfiable or unreachable.
This option will be integrated within the framework of [Global Conditions](https://model-checking.github.io/kani/rfc/rfcs/0007-global-conditions.html), which is used to define properties that depend on other properties.

Using the `--fail-uncoverable` option will enable the global condition with name `fail_uncoverable`.
Following the format for global conditions, the outcome will be one of the following:
 1. `` - fail_uncoverable: FAILURE (encountered one or more cover statements which were not satisfied)``
 2. `` - fail_uncoverable: SUCCESS (all cover statements were satisfied as expected)``

Note that the criteria to achieve a `SUCCESS` status depends on all properties of the `"cover"` class having a `SATISFIED` status.
Otherwise, we return a `FAILURE` status.

### Inclusion in the Verification Summary

Cover checks will be reported separately in the verification summary, e.g.
```
SUMMARY:
 ** 1 of 206 failed (2 unreachable)
 Failed Checks: assertion failed: x[0] == x[1]

 ** 30 of 35 cover statements satisfied (1 unreachable) <--- NEW
 ```
In this example, 5 of the 35 cover statements were found to be unsatisfiable, and one of those 5 is additionally unreachable.
### Interaction with Other Checks

If one or more unwinding assertions fail or an unsupported construct is found to be reachable (which indicate an incomplete path exploration), and Kani found the condition to be unsatisfiable or unreachable, the result will be reported as `UNDETERMINED`.

## Detailed Design

The implementation will touch the following components:
- Kani library: The `cover` macro will be added there along with a `cover` function with a `rustc_diagnostic_item`
- `kani-compiler`: The `cover` function will be handled via a hook and codegen as two assertions (`cover(cond)` will be codegen as `__CPROVER_assert(false); __CPROVER_assert(!cond)`).
The purpose of the `__CPROVER_assert(false)` is to determine whether the `cover` statement is reachable.
If it is, the second `__CPROVER_assert(!cond)` indicates whether the condition is satisfiable or not.
- `kani-driver`: The CBMC output parser will extract cover properties through their property class, and their result will be set based on the result of the two assertions:
  - The first (reachability) assertion is proven: report as `FAILURE (UNREACHABLE)`
  - The first assertion fails, and the second one is proven: report as `FAILURE` to indicate that the condition is unsatisfiable
  - The first assertion fails, and the second one fails: report as `SUCCESS` to indicate that the condition is satisfiable

## Rationale and alternatives

- What are the pros and cons of this design?
CBMC has its own [cover API (`__CPROVER_cover`)](https://diffblue.github.io/cbmc//cprover__builtin__headers_8h.html#a44f072b21e93cb0f72adcccc9005f307), for which `SUCCESS` is reported if an execution is found, and `FAILURE` is reported otherwise.
However, using this API currently requires running CBMC in a separate ["cover" mode](https://github.com/diffblue/cbmc/issues/6613).
Having to run CBMC in that mode would complicate the Kani driver as it will have to perform two CBMC runs, and then combine their results into a single report.
Thus, the advantage of the proposed design is that it keeps the Kani driver simple.
In addition, the proposed solution does not depend on a feature in the backend, and thus will continue to work if we were to integrate a different backend.

- What is the impact of not doing this?
The current workaround to accomplish the same effect of verifying that a condition can be covered is to use `assert!(!cond)`.
However, if the condition can indeed be covered, verification would fail due to the failure of the assertion.

## Open questions

Should we allow format arguments in the macro, e.g. `kani::cover!(x > y, "{} can be greater than {}", x, y)`?
Users may expect this to be supported since the macro looks similar to the `assert` macro, but Kani doesn't include the formatted string in the message description, since it's not available at compile time.

## Other Considerations

We need to make sure the concrete playback feature can be used with `cover` statements that were found to be coverable.

## Future possibilities

The new cover API subsumes the current `kani::expect_fail` function.
Once it's implemented, we should be able to get rid of `expect_fail`, and all the related code in `compiletest` that handles the `EXPECTED FAILURE` message in a special manner.
