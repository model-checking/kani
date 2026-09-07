# Debugging Slow Proofs

Kani uses SAT/SMT solvers to verify code, which can sometimes result in slow or non-terminating proofs. This chapter outlines common causes of slowness and strategies to debug and improve proof performance.

## Common Causes of Slow Proofs

### Complex/Large Non-deterministic Types
Some types are inherently more expensive to represent symbolically, e.g. strings, which have complex validation rules for UTF-8 encoding,
or large bounded collections, like a vector with a large size.

### Large Value Operations
Mathematical operations on large values can be expensive, e.g., multiplication/division/modulo, especially with larger types (e.g., `u64`).
The cost can grow sharply with bit-width rather than linearly.
For example, on one local machine, an unconstrained proof harness for exact integer division took under 0.2 seconds to verify for `i8`, about 25 seconds for the full `i16` range, and about 55 seconds for the full `u16` range — despite each step only doubling the bit-width.
This can reflect the SAT solver's worst-case exponential behavior on bit-blasted arithmetic circuits rather than a tooling inefficiency, and it means proofs that work fine on `i16` or smaller types may become impractical on `i32` and larger types without further bounding.

### Unbounded Loops
If Kani cannot determine a loop bound, it will unwind forever, c.f. [the loop unwinding tutorial](./tutorial-loop-unwinding.md).

## Debugging Strategies

These are some strategies to debug slow proofs, ordered roughly in terms of in the order you should try them:

### Limit Loop Iterations

First, identify whether (unbounded) loop unwinding may be the root cause. Try the `#[kani::unwind]` attribute or the `--unwind` option to limit [loop unwinding](./tutorial-loop-unwinding.md). If the proof fails because the unwind value is too low, but raising it causing the proof to be too slow, try specifying a [loop contract](./reference/experimental/loop-contracts.md) instead.

### Use Different Solvers

Kani supports multiple SAT/SMT solvers that may perform differently on your specific problem. Try out different solvers with the `#[kani::solver]` [attribute](./reference/attributes.md) or `--solver` option.

### Remove Sources of Nondeterminism

Start by replacing `kani::any()` calls with concrete values to isolate the problem:

```rust
#[kani::proof]
fn slow_proof() {
    // Instead of this:
    // let x: u64 = kani::any();
    // let y: u64 = kani::any();
    
    // Try this:
    let x: u64 = 42;
    let y: u64 = 100;
    
    let result = complex_function(x, y);
    assert!(result > 0);
}
```

If the proof becomes fast with concrete values, the issue is likely with the symbolic representation of your inputs. In that case, see you can [partition the proof](#partition-the-input-space) to cover different ranges of possible values, or restrict the proof to a smaller range of values if that is acceptable for your use case.

### Reduce Collection Sizes

Similarly, if smaller values are acceptable for your proof, use those instead:

```rust
#[kani::proof]
fn test_with_small_collection() {
    // Instead of a large Vec
    // let vec: Vec<u8> = kani::bounded_any::<_, 100>();
    
    // Start with a small size
    let vec: Vec<u8> = kani::bounded_any::<_, 2>();
    
    process_collection(&vec);
}
```

### Partition the Input Space

Break down complex proofs by partitioning the input space:

```rust
// Instead of one slow proof with large inputs
#[kani::proof]
fn test_multiplication_slow() {
    let x: u64 = kani::any();
    let y: u64 = kani::any();
    
    // This might be too slow for the solver
    let result = x.saturating_mul(y);
    assert!(result >= x || x == 0);
}

// Split into multiple proofs with bounded inputs
#[kani::proof]
fn test_multiplication_small_values() {
    let x: u64 = kani::any_where(|x| *x <= 100);
    let y: u64 = kani::any_where(|y| *y <= 100);
    
    let result = x.saturating_mul(y);
    assert!(result >= x || x == 0);
}

// Insert harnesses for other ranges of `x` and `y`
```

See this [tracking issue](https://github.com/model-checking/kani/issues/3006) for adding support for such partitioning automatically.

#### Division: Bound Both Operands

For division and modulo operations specifically, bounding only one operand is often insufficient — both operands typically need to be constrained to make verification tractable:

```rust
// May not converge in reasonable time: only the divisor is bounded
#[kani::proof]
fn test_division_divisor_only() {
    let dividend: i64 = kani::any();
    let divisor: i64 = kani::any_where(|d| *d != 0);
    let _ = dividend / divisor;
}

// Converges quickly: both operands are bounded
#[kani::proof]
fn test_division_both_bounded() {
    let dividend: i64 = kani::any_where(|d| *d >= -1000 && *d <= 1000);
    let divisor: i64 = kani::any_where(|d| *d != 0 && *d >= -1000 && *d <= 1000);
    let _ = dividend / divisor;
}
```

In practice, bounding both operands to a small representative range typically brings verification for `i32` and larger integer types down to well under a second, compared to unconstrained runs that may take significantly longer or fail to converge within a reasonable timeout.

### Use Stubs

If a function has a complex body, consider using a [stub](./reference/experimental/stubbing.md) or a [verified stub](./reference/experimental/contracts.md) to stub the body with a simpler abstraction.

### Disable Unnecessary Checks

If you're focusing on functional correctness rather than safety, you may disable memory safety checks (run `kani --help` for a list of options to do so). Note that disabling these checks may cause Kani to miss undefined behavior, so use it with caution.

Alternatively, to assume that all assertions succeed and only focus on finding safety violations, use the `--prove-safety-only` option.
