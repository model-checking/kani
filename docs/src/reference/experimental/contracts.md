# Contracts

Consider the following example:

```rust
fn gcd(mut max: u8, mut min: u8) -> u8 {
    if min > max {
        std::mem::swap(&mut max, &mut min);
    }

    let rest = max % min;
    if rest == 0 { min } else { gcd(min, rest) }
}
```
Let's assume we want to verify some code that calls `gcd`.
In the [worst case](https://en.wikipedia.org/wiki/Euclidean_algorithm#Worst-case), the number of steps (recursions) in `gcd` approaches 1.5 times the number of bits needed to represent the input numbers. 
So, for two large 64-bit numbers, a single call to `gcd` can take almost 96 iterations.
It would be very expensive for Kani to unroll each of these iterations and then perform symbolic execution.

Instead, we can write *contracts* with guarantees about `gcd`'s behavior.
Once Kani verifies that `gcd`s contracts are correct, it can replace each invocation of `gcd` with its contracts, which reduces verification time for `gcd`'s callers.
For example, perhaps we want to ensure that the returned `result` does indeed divide both `max` and `min`.
In that case, we could write contracts like these:

```rust
#[kani::requires(min != 0 && max != 0)]
#[kani::ensures(|result| *result != 0 && max % *result == 0 && min % *result == 0)]
#[kani::recursion]
fn gcd(mut max: u8, mut min: u8) -> u8 { ... }
```

Since `gcd` performs `max % min` (and perhaps swaps those values), passing zero as an argument could cause a division by zero.
The `requires` contract tells Kani to restrict the range of nondeterministic inputs to nonzero ones so that we don't run into this error.
The `ensures` contract is what actually checks that the result is a correct divisor for the inputs.
(The `recursion` attribute is required when using contracts on recursive functions).

Then, we would write a harness to *verify* those contracts, like so:

```rust
#[kani::proof_for_contract(gcd)]
fn check_gcd() {
    let max: u8 = kani::any();
    let min: u8 = kani::any();
    gcd(max, min);
}
```

and verify it by running `kani -Z function-contracts`.

Once Kani verifies the contracts, we can use Kani's [stubbing feature](stubbing.md) to replace all invocations to `gcd` with its contracts, for instance:

```rust
// Assume foo() invokes gcd().
// By using stub_verified, we tell Kani to replace 
// invocations of gcd() with its verified contracts.
#[kani::proof]
#[kani::stub_verified(gcd)]
fn check_foo() {
    let x: u8 = kani::any();
    foo(x);
}
```
By leveraging the stubbing feature, we can replace the (expensive) `gcd` call with a *verified abstraction* of its behavior, greatly reducing verification time for `foo`.

There is far more to learn about contracts.
We highly recommend reading our [blog post about contracts](https://model-checking.github.io/kani-verifier-blog/2024/01/29/function-contracts.html) (from which this `gcd` example is taken). We also recommend looking at the `contracts` module in our [documentation](../../crates/index.md).
