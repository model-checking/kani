# Quantifiers in Kani

Quantifiers are a powerful feature in formal verification that allow you to express properties over a range of values. Kani provides experimental support for quantifiers, enabling users to write concise and expressive specifications for their programs.

## Supported Quantifiers

Kani currently supports the following quantifiers:

1. **Universal Quantifier**:
   - Ensures that a property holds for all values in a given range.
   - Syntax: `kani::forall!(|variable in range| condition)`
   - Example:

```rust
#[kani::proof]
fn test_forall() {
    let v = vec![10; 10];
    kani::assert(kani::forall!(|i in 0..10| v[i] == 10));
}
```

2. **Existential Quantifier**:
   - Ensures that there exists at least one value in a given range for which a property holds.
   - Syntax: `kani::exists!(|variable in range| condition)`
   - Example:

```rust
#[kani::proof]
fn test_exists() {
    let v = vec![1, 2, 3, 4, 5];
    kani::assert(kani::exists!(|i in 0..v.len()| v[i] == 3));
}
```

### Limitations

#### Array Indexing

The performance of quantifiers can be affected by the depth of call stacks in the quantified expressions. If the call stack is too deep, Kani may not be able to evaluate the quantifier effectively, leading to potential timeouts or running out of memory. Actually, array indexing in Rust leads to a deep call stack, which can cause issues with quantifiers. To mitigate this, consider using *unsafe* pointer dereferencing instead of array indexing when working with quantifiers. For example:

```rust

#[kani::proof]
fn vec_assert_forall_harness() {
    let v = vec![10 as u8; 128];
    let ptr = v.as_ptr();
    unsafe {
        kani::assert(kani::forall!(|i in (0,128)| *ptr.wrapping_byte_offset(i as isize) == 10), "");
    }
}
```

#### Types of Quantified Variables

We now assume that all quantified variables are of type `usize`. This means that the range specified in the quantifier must be compatible with `usize`.
 We plan to support other types in the future, but for now, ensure that your quantifiers use `usize` ranges.
