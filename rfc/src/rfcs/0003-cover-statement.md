- **Feature Name:** Cover statement `cover_statement`
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/696>
- **RFC PR:** <https://github.com/model-checking/kani/pull/1906>
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

## Summary

A new Kani API that allows users to check that a certain condition can occur at a specific location in the code.

## User Impact

Users typically want to gain confidence that a proof checks what it is supposed to check, i.e. that properties are not passing vacuously due to an over-constrained environment.

A new Kani function, `cover` will be created with the following signature:
```rust
fn cover(cond: bool)
```
This function can be used for checking that a certain condition _can_ occur at a specific location in the code, to verify, for example, that assumptions are not ruling out those conditions, e.g.:
```rust
let mut v: Vec<i32> = Vec::new();
let len: usize = kani::any();
kani::assume(len < 5);
for _i in 0..len {
    v.push(kani::any());
}
// make sure we can get a vector of length 5
kani::cover(v.len() == 5);
```
This is typically used to ensure that verified checks are not passing _vacuously_, e.g. due to overconstrained pre-conditions.

The special case of verifying that a certain line of code is reachable can be achieved using `kani::cover(true)`, e.g.
```rust
match x {
    val_1 => ...,
    val_2 => ...,
    ...
    val_i => kani::cover(true), // verify that `x` can be `val_i`
}
```

## User Experience

The `cover` function instructs Kani to find _at least one_ possible execution that satisfies the specified condition at that line of code. If there is no such execution, verification *fails*.

Each cover statements will be reported as a check whose description is "condition is satisfiable" and whose status is:
- `SUCCESS` (green): if Kani found an execution that satisfies the condition
- `FAILURE` (red): if Kani proved that the condition cannot be satisfied

For example:
```
Check 2: main.cover.2
         - Status: SUCCESS
         - Description: "condition is satisfiable"
         - Location: foo.rs:9:5 in function main
```

If one or more unwinding assertions fail or an unsupported construct is found to be reachable, and Kani proved that the condition cannot be satisfied, the result will be reported as `UNDETERMINED` instead of `FAILURE`.

## Detailed Design

The implementation will touch the following components:
- Kani library: The `cover` function will be added there
- `kani-compiler`: The `cover` function will be handled via a hook and codegen as an assertion (`cover(cond)` will be codegen as `assert(!cond)`)
- `kani-driver`: The CBMC output parser will extract cover properties through their property class, and their result will be flipped.

## Rationale and alternatives

- What is the impact of not doing this?
The current workaround to accomplish the same effect of verifying that a condition can be covered is to use `assert!(!cond)`.
However, if the condition can indeed be covered, verification would fail due to the failure of the assertion.

## Open questions

## Other Considerations

We need to make sure the concrete playback feature can be used with `cover` statements that were found to be coverable.

## Future possibilities

Users may want to specify a custom message for the cover property, in which case, we may consider adding a separate function that takes a static `str`, e.g.
```rust
fn cover_msg(cond: bool, msg: &'static str)
```
or a `cover` macro (similar to Rust's `assert` macro).
