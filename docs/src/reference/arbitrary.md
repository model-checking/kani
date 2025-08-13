# Arbitrary Trait

The `Arbitrary` trait is the foundation for generating non-deterministic values in Kani proof harnesses. It allows you to create symbolic values that represent all possible values of a given type.

For a type to implement `Arbitrary`, Kani must be able to represent every possible value of it, so unbounded types cannot implement it. For nondeterministic representations of unbounded types, e.g., `Vec`, see the [`BoundedArbitrary` trait](./bounded_arbitrary.md).

## Overview

The `Arbitrary` trait defines methods for generating arbitrary (nondeterministic) values:

```rust
pub trait Arbitrary
where
    Self: Sized,
{
    fn any() -> Self;
    fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH] {
        [(); MAX_ARRAY_LENGTH].map(|_| Self::any())
    }
}
```

## Basic Usage

Use `kani::any()` to generate arbitrary values in proof harnesses:

```rust
#[kani::proof]
fn verify_function() {
    let x: u32 = kani::any();
    let y: bool = kani::any();
    let z: [char; 10] = kani::any();
    
    // x represents all possible u32 values
    // y represents both true and false
    // z represents an array of length 10 where each element can hold all possible char values
    my_function(x, y, z);
}
```

Kani implements `Arbitrary` for primitive types and some standard library types. See the [crate trait documentation](https://model-checking.github.io/kani/crates/doc/kani/trait.Arbitrary.html#foreign-impls) for a full list of implementations. 

## Constrained Values

Use `any_where` or `kani::assume()` to add constraints to arbitrary values:

```rust
#[kani::proof]
fn verify_with_constraints() {
    let x: u32 = kani::any_where(|t| *t < 1000); // Constrain x to be less than 1000
    kani::assume(x % 2 == 0); // Constrain x to be even
    
    // Now x represents all even numbers from 0 to 998
    my_function(x);
}

## Derive Implementations

Kani can automatically derive `Arbitrary` implementations for structs and enums when all their fields/variants implement `Arbitrary`:

### Structs

```rust
#[derive(kani::Arbitrary)]
struct Point {
    x: i32,
    y: i32,
}

#[kani::proof]
fn verify_point() {
    let point: Point = kani::any();
    // point.x and point.y can be any i32 values
    process_point(point);
}
```

### Enums

```rust
#[derive(kani::Arbitrary)]
enum Status {
    Ready,
    Processing(u32),
    Error { code: (char, i32) },
}

#[kani::proof]
fn verify_status() {
    let status: Status = kani::any();
    // `status` can be any of the variants
    match status {
        Status::Ready => { /* ... */ }
        Status::Processing(id) => { /* id can be any u32 */ }
        Status::Error { code } => { /* code can be any (char, i32) tuple */ }
    }
}
```

## Manual Implementations

Implement `Arbitrary` manually when you need constraints or custom logic. For example, Kani [manually implements `Arbitrary` for `NonZero` types](https://github.com/model-checking/kani/blob/100857e99d7506992c4589332a0d7d8dae1ee29a/library/kani_core/src/arbitrary.rs#L48-L60) to exclude zero values, e.g:

```rust
impl Arbitrary for NonZeroU8 {
    fn any() -> Self {
        let val = u8::any();
        kani::assume(val != 0);
        unsafe { NonZeroU8::new_unchecked(val) }
    }
}
```

An alternative means to add value constraints is provided by the [Invariant trait](https://model-checking.github.io/kani/crates/doc/kani/invariant/trait.Invariant.html).

## See Also

- [Nondeterministic Variables Tutorial](../tutorial-nondeterministic-variables.md)
- [Bounded Non-deterministic Variables](./bounded_arbitrary.md)
- [First Steps Tutorial](../tutorial-first-steps.md)