# Attributes

In Kani, attributes are used to mark functions as harnesses and control their execution.
This section explains the attributes available in Kani and how they affect the verification process.

At present, the available Kani attributes are the following:
 - [`#[kani::proof]`](#kaniproof)
 - [`#[kani::should_panic]`](#kanishould_panic)
 - [`#[kani::unwind(<number>)]`](#kaniunwindnumber)
 - [`#[kani::solver(<solver>)]`](#kanisolversolver)
 - [`#[kani::stub(<original>, <replacement>)]`](#kanistuboriginal-replacement)

## `#[kani::proof]`

**The `#[kani::proof]` attribute specifies that a [function](https://doc.rust-lang.org/reference/items/functions.html) is a proof harness.**

Proof harnesses are similar to test harnesses, especially property-based test harnesses,
and they may use functions from the Kani API (e.g., `kani::any()`).
A proof harness is the smallest verification unit in Kani.

When Kani is run, either through `kani` or `cargo kani`, it'll first collect all proof harnesses
(i.e., functions with the attribute `#[kani::proof]`) and then attempt to verify them.

### Example

If we run Kani on this example:

```rust
#[kani::proof]
fn my_harness() {
    assert!(1 + 1 == 2);
}
```

We should see a line in the output that says `Checking harness my_harness...` (assuming `my_harness` is the only harness in our code).
This will be followed by multiple messages that come from CBMC (the verification engine used by Kani) and the [verification results](../verification-results.md).

Using any other Kani attribute without `#[kani::proof]` will result in compilation errors.

### Limitations

The `#[kani::proof]` attribute can only be added to functions without parameters.

## `#[kani::should_panic]`

**The `#[kani::should_panic]` attribute specifies that a proof harness is expected to panic.**

This attribute allows users to exercise *negative verification*.
It's analogous to how [`#[should_panic]`](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics) allows users to exercise [negative testing](https://en.wikipedia.org/wiki/Negative_testing) for Rust unit tests.

This attribute **only affects the overall verification result**.
In particular, using the `#[kani::should_panic]` attribute will return one of the following results:
  - `VERIFICATION:- FAILED (encountered no panics, but at least one was expected)` if there were no failed checks.
  - `VERIFICATION:- FAILED (encountered failures other than panics, which were unexpected)` if there were failed checks but not all them were related to panics.
  - `VERIFICATION:- SUCCESSFUL (encountered one or more panics as expected)` otherwise.

At the moment, to determine if a check is related to a panic, we check if its class is `assertion`.
The class is the second member in the property name, the triple that's printed after `Check X: `: `<function>.<class>.<number>`.
For example, the class in `Check 1: my_harness.assertion.1` is `assertion`, so this check is considered to be related to a panic.

> **NOTE**: The `#[kani::should_panic]` is only recommended for writing
> harnesses which complement existing harnesses that don't use the same
> attribute. In order words, it's only recommended to write *negative harnesses*
> after having written *positive* harnesses that successfully verify interesting
> properties about the function under verification.

### Limitations

The `#[kani::should_panic]` attribute verifies that there are one or more failed checks related to panics.
At the moment, it's not possible to pin it down to specific panics.
Therefore, **it's possible that the panics detected with `#[kani::should_panic]` aren't the ones that were originally expected** after a change in the code under verification.

### Example

Let's assume we're using the `Device` from this example:

```rust
struct Device {
    is_init: bool,
}

impl Device {
    fn new() -> Self {
        Device { is_init: false }
    }

    fn init(&mut self) {
        assert!(!self.is_init);
        self.is_init = true;
    }
}
```

We may want to verify that calling `device.init()` more than once should result in a panic.
We can do so with the following harness:

```rust
#[kani::proof]
#[kani::should_panic]
fn cannot_init_device_twice() {
    let mut device = Device::new();
    device.init();
    device.init();
}
```

Running Kani on it will produce the result `VERIFICATION:- SUCCESSFUL (encountered one or more panics as expected)`

## `#[kani::unwind(<number>)]`

**The `#[kani::unwind(<number>)]` attribute specifies that all loops must be unwound up to `<number>` times.**

By default, Kani attempts to unwind all loops automatically.
However, this unwinding process doesn't always terminate.
The `#[kani::unwind(<number>)]` attribute will:
 1. Disable automatic unwinding.
 2. Unwind all loops up to `<number>` times.

After the unwinding stage, Kani will attempt to verify the harness.
If the `#[kani::unwind(<number>)]` attribute was specified, there's a chance that one or more loops weren't unwound enough times.
In that case, there will be at least one failed unwinding assertion (there's one unwinding assertion for each loop), causing verification to fail.

Check the [*Loops, unwinding and bounds* section](../tutorial-loop-unwinding.md) for more information about unwinding.

### Example

Let's assume we've written this code which contains a loop:

```rust
fn my_sum(vec: &Vec<u32>) -> u32 {
    let mut sum = 0;
    for elem in vec {
        sum += elem;
    }
    sum
}

#[kani::proof]
fn my_harness() {
    let vec = vec![1, 2, 3];
    let sum = my_sum(&vec);
    assert!(sum == 6);
}
```

Running this example on Kani will produce a successful verification result.
In this case, Kani automatically finds the required unwinding value (i.e., the number of times it needs to unwind all loops).
This means that the `#[kani::unwind(<number>)]` attribute isn't needed, as we'll see soon.
In general, the required unwinding value is equal to the maximum number of iterations for all loops, plus one.
The required unwinding value in this example is 4: the 3 iterations in the `for elem in vec` loop, plus 1.

Let's see what happens if we force a lower unwinding value with `#[kani::unwind(3)]`:

```rust
#[kani::proof]
#[kani::unwind(3)]
fn my_harness() {
    let vec = vec![1, 2, 3];
    let sum = my_sum(&vec);
    assert!(sum == 6);
}
```

As we mentioned, trying to verify this harness causes an unwinding failure:

```
SUMMARY:
 ** 1 of 187 failed (186 undetermined)
Failed Checks: unwinding assertion loop 0
 File: "/home/ubuntu/devices/src/main.rs", line 32, in my_sum

VERIFICATION:- FAILED
[Kani] info: Verification output shows one or more unwinding failures.
[Kani] tip: Consider increasing the unwinding value or disabling `--unwinding-assertions`.
```

Kani cannot verify the harness because there is at least one unwinding assertion failure.
But, if we use `#[kani::unwind(4)]`, which is the right unwinding value we computed earlier:

```rust
#[kani::proof]
#[kani::unwind(4)]
fn my_harness() {
    let vec = vec![1, 2, 3];
    let sum = my_sum(&vec);
    assert!(sum == 6);
}
```

We'll get a successful result again:

```
SUMMARY:
 ** 0 of 186 failed

VERIFICATION:- SUCCESSFUL
```

## `#[kani::solver(<solver>)]`

**Changes the solver to be used by Kani's verification engine (CBMC).**

This may change the verification time required to verify a harness.

At present, `<solver>` can be one of:
 - `minisat`: [MiniSat](http://minisat.se/).
 - `cadical` (default): [CaDiCaL](https://github.com/arminbiere/cadical).
 - `kissat`: [kissat](https://github.com/arminbiere/kissat).
 - `bin="<SAT_SOLVER_BINARY>"`: A custom solver binary, `"<SAT_SOLVER_BINARY>"`, that must be in path.

### Example

Kani will use the CaDiCaL solver in the following example:

```rust
#[kani::proof]
#[kani::solver(cadical)]
fn check() {
    let mut a = [2, 3, 1];
    a.sort();
    assert_eq!(a[0], 1);
    assert_eq!(a[1], 2);
    assert_eq!(a[2], 3);
}
```

Changing the solver may result in different verification times depending on the harness.

Note that the default solver may vary depending on Kani's version.
We highly recommend users to annotate their harnesses if the choice of solver
has a major impact on performance, even if the solver used is the current
default one.

## `#[kani::stub(<original>, <replacement>)]`

**Replaces the function/method with name <original> with the function/method with name <replacement> during compilation**

Check the [*Stubbing* section](../reference/stubbing.md) for more information about stubbing.
