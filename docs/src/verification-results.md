# Verification results

Running Kani on a harness produces an output that includes a set of checks as
follows:

```
RESULTS:
Check 1: example.assertion.1
         - Status: <status>
         - Description: <description>
         - Location: <location>
[...]
```

Kani determines the verification result for the harness based on the
result (i.e., `<status>`) of each individual check (also known as "properties"). If all
checks are successful then the overall verification result of the harness is successful. Otherwise the
verification fails, which indicates issues with the code under verification.

## Check results

The result (or `Status`) of a check in Kani can be one of the following:

1. `SUCCESS`: This indicates that the check passed (i.e., the property holds).
Note that in some cases, the property may hold _vacuously_. This can occur
because the property is unreachable, or because the harness is
_over-constrained_.

Example:
```rust
{{#include getting-started/verification-results/src/main.rs:success_example}}
```
The output from Kani indicates that the assertion holds:
```
Check 4: success_example.assertion.4
         - Status: SUCCESS
         - Description: "assertion failed: sum == 6"
```

2. `FAILURE`: This indicates that the check failed (i.e., the property doesn't
hold). In this case, please see the [concrete playback](./experimental/concrete-playback.md)
section for more help.

Example:
```rust
{{#include getting-started/verification-results/src/main.rs:failure_example}}
```
The assertion doesn't hold as Kani's output indicates:
```
Check 2: failure_example.assertion.2
         - Status: FAILURE
         - Description: "assertion failed: arr.len() != 3"
```

3. `UNREACHABLE`: This indicates that the check is unreachable (i.e., the
property holds _vacuously_). This occurs when there is no possible execution
trace that can reach the check's line of code.
This may be because the function that contains the check is unused, or because
the harness does not trigger the condition under which the check is invoked.
Kani currently checks reachability for the following assertion types:
    1. Assert macros (e.g. `assert`, `assert_eq`, etc.)
    2. Arithmetic overflow checks
    3. Negation overflow checks
    4. Index out-of-bounds checks
    5. Divide/remainder-by-zero checks

Example:

```rust
{{#include getting-started/verification-results/src/main.rs:unreachable_example}}
```

The output from Kani indicates that the assertion is unreachable:
```
Check 2: unreachable_example.assertion.2
         - Status: UNREACHABLE
         - Description: "assertion failed: x < 8"
```

4. `UNDETERMINED`: This indicates that Kani was not able to conclude whether the
property holds or not. This can occur when the Rust program contains a construct
that is not currently supported by Kani. See
[Rust feature support](./rust-feature-support.md) for Kani's current support of the
Rust language features.

Example:
```rust
{{#include getting-started/verification-results/src/main.rs:undetermined_example}}
```
The output from Kani indicates that the assertion is undetermined due to the
missing support for inline assembly in Kani:
```
Check 2: undetermined_example.assertion.2
         - Status: UNDETERMINED
         - Description: "assertion failed: x == 0"
```

## Cover property results

Kani provides a [`kani::cover`](https://model-checking.github.io/kani/crates/doc/kani/macro.cover.html) macro that can be used for checking whether a condition may occur at a certain point in the code.

The result of a cover property can be one of the following:

1. `SATISFIED`: This indicates that Kani found an execution that triggers the specified condition.

The following example uses `kani::cover` to check if it's possible for `x` and `y` to hold the values 24 and 72, respectively, after 3 iterations of the `while` loop, which turns out to be the case.
```rust
{{#include getting-started/verification-results/src/main.rs:cover_satisfied_example}}
```
Results:
```
Check 1: cover_satisfied_example.cover.1
         - Status: SATISFIED
         - Description: "cover condition: i > 2 && x == 24 && y == 72"
         - Location: src/main.rs:60:9 in function cover_satisfied_example
```

2. `UNSATISFIABLE`: This indicates that Kani _proved_ that the specified condition is impossible.

The following example uses `kani::cover` to check if it's possible to have a UTF-8 encoded string consisting of 5 bytes that correspond to a string with a single character.
```rust
{{#include getting-started/verification-results/src/main.rs:cover_unsatisfiable_example}}
```
which is not possible as such string will contain at least two characters.
```
Check 46: cover_unsatisfiable_example.cover.1
         - Status: UNSATISFIABLE
         - Description: "cover condition: s.chars().count() <= 1"
         - Location: src/main.rs:75:9 in function cover_unsatisfiable_example
```

3. `UNREACHABLE`: This indicates that the `cover` property itself is unreachable (i.e. it is _vacuously_ unsatisfiable).

In contrast to an `UNREACHABLE` result for assertions, an unreachable (or an unsatisfiable) cover property may indicate an incomplete proof.

Example:
In this example, a `kani::cover` call is unreachable because if the outer `if` condition holds, then the non-empty range `r2` is strictly larger than the non-empty range `r1`, in which case, the condition in the inner `if` condition is impossible.
```rust
{{#include getting-started/verification-results/src/main.rs:cover_unreachable_example}}
```
```
Check 3: cover_unreachable_example.cover.1
         - Status: UNREACHABLE
         - Description: "cover condition: r2.contains(&0)"
         - Location: src/main.rs:90:13 in function cover_unreachable_example
```

4. `UNDETERMINED`: This is the same as the `UNDETERMINED` result for normal checks (see [check_results]).

## Verification summary

Kani reports a summary at the end of the verification report, which includes the overall results of all checks, the overall results of cover properties (if the package includes cover properties), and the overall verification result, e.g.:
```
SUMMARY:
 ** 0 of 786 failed (41 unreachable)

 ** 0 of 1 cover properties satisfied


VERIFICATION:- SUCCESSFUL
```
