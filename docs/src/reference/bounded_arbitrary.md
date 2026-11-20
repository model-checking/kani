# Bounded Non-deterministic variables

This is an experimental feature that allows you to bound otherwise unbounded types. For example, `Vec<T>` does not have an `Arbitrary` implementation because vectors can grow arbitrarily in size. One way of handling proofs about such types is to make the problem easier, and only prove a property up to some bound. Of course, the proof is only valid up to the bound, but can still be useful in providing confidence that your code is correct.

## Example

As a toy example, let's prove, up to some bound, that reversing a vector twice gives you back the original vector. Here's a reversing function:

```rust
fn reverse_vector<T>(mut input: Vec<T>) -> Vec<T> {
    let mut reversed = vec![];
    for _ in 0..input.len() {
        reversed.push(input.pop().unwrap());
    }
    reversed
}
```

We can use `BoundedAny` to write a proof harness:

```rust
#[kani::proof]
#[kani::unwind(17)]
fn check_reverse_is_its_own_inverse() {
    // We use BoundedAny to construct a vector that has at most length 16
    let input: Vec<bool> = kani::bounded_any::<_, 16>();

    let double_reversed = reverse_vector(reverse_vector(input.clone()));

    // we assert that every value in the input is the same as the value in the
    // doubly reversed list
    for i in 0..input.len() {
        assert_eq!(input[i], double_reversed[i])
    }
}
```

Then, with `kani` we can prove that our reverse function is indeed its own inverse, for vectors up to size 16.

## Proof Incompleteness

It's very important to note, that this is **not** a complete proof that this function is correct. To drive this point home, consider this bad implementation of `reverse_vector`:

```rust
fn bad_reverse_vector<T: Default>(mut input: Vec<T>) -> Vec<T> {
    let mut reversed = vec![];
    for i in 0..input.len() {
        if i < 16 {
            reversed.push(input.pop().unwrap());
        } else {
            reversed.push(T::default())
        }
    }
    reversed
}
```

Now the same harness as before is still successful! Even though this implementation is obviously wrong. If only we had tried a slightly bigger bound...

So, while bounded proofs can be useful, beware that they are also incomplete. It might be worth-while to test multiple bounds.

## Custom Bounded Arbitrary implementations

Kani provides several implementations of `BoundedArbitrary`, but you can also implement `BoundedArbitrary` for yourself.

We provide a derive macro that should work in most cases:

```rust
#[derive(BoundedArbitrary)]
struct MyVector<T> {
    #[bounded]
    vector: Vec<T>,
    capacity: usize
}
```

You must specify which fields should be bounded using the `#[bounded]` attribute. All other fields must derive `Arbitrary`.

### Limitations

Currently you can only specify a single bound for the entire type, and all bounded fields use the same bound. If different bounds would be useful, let us know through [filing an issue](https://github.com/model-checking/kani/issues/new/choose) and we can probably lift this restriction.
