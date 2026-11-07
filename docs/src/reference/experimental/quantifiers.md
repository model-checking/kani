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
    let v : [u8;10] = [10; 10];
    assert!(kani::forall!(|i in (0,10)| v[i] == 10) );
}
```

2. **Existential Quantifier**:
   - Ensures that there exists at least one value in a given range for which a property holds.
   - Syntax: `kani::exists!(|variable in range| condition)`
   - Example:

```rust
#[kani::proof]
fn test_exists() {
    let v : [u8;5] = [1, 2, 3, 4, 5];
    assert!(kani::exists!(|i in (0,v.len())| v[i] == 3));
}
```


### Limitations

#### Safety Checks in Predicates

With `-Z valid-value-checks` or `-Z uninit-checks`, predicates containing explicit safety checks are currently rejected because those checks cannot be retained in a pure quantifier expression. The scan includes statements embedded in expressions. With these options enabled, function calls are preserved even when their return value is unused, and helper bodies are checked before quantifier inlining. It recognizes the `safety_check`, `arithmetic_overflow`, `exact_div`, and `finite_check` property classes; it does not establish preservation of every assertion class. Safety checks performed before constructing the closure remain active.

With either extra UB option, existential predicates that require statement-expression fallback are also rejected because of [CBMC #9189](https://github.com/diffblue/cbmc/issues/9189). Predicates that can be lowered directly to pure expressions are unaffected. Calls requiring quantifier inlining are rejected when no GOTO body is available, including bodyless builtins.

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

#### Solver Backend Support

The default SAT-based solver backend only supports quantifiers with constant bounds. A quantifier whose bound is symbolic (not known at compile time) cannot be encoded and would be silently replaced with an unconstrained value, which is unsound: a `kani::assume` containing such a quantifier would not be enforced, and a `kani::assert` containing one could fail spuriously.

To keep results sound, Kani reports verification as `FAILED` when the backend drops a quantifier, and directs you to an SMT solver backend that supports quantifiers. Use `#[kani::solver(z3)]` on the harness (or `--solver z3` on the command line) to verify these quantifiers.
