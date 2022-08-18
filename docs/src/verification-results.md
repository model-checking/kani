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
hold). In this case, please see the [debugging verification failures](./concrete-playback.md)
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
