# Automatic Harness Generation

Recall the harness for `estimate_size` that we wrote in [First Steps](../../tutorial-first-steps.md):
```rust
{{#include ../../tutorial/first-steps-v1/src/lib.rs:kani}}
```

This harness first declares a local variable `x` using `kani::any()`, then calls `estimate_size` with argument `x`.
Many proof harnesses follow this predictable format—to verify a function `foo`, we create arbitrary values for each of `foo`'s arguments, then call `foo` on those arguments.

The `autoharness` subcommand leverages this observation to automatically generate harnesses and run Kani against them.
Kani scans the crate for functions whose arguments all implement the `kani::Arbitrary` trait, generates harnesses for them, then runs them.
These harnesses are internal to Kani—i.e., Kani does not make any changes to your source code.

## Usage
Run either:
```
# cargo kani autoharness -Z autoharness
```
or
```
# kani autoharness -Z autoharness <FILE>
```

If Kani detects that all of a function `foo`'s arguments implement `kani::Arbitrary`, it will generate and run a `#[kani::proof]` harness, which prints:

```
Autoharness: Checking function foo against all possible inputs...
<VERIFICATION RESULTS>
```

However, if Kani detects that `foo` has a [function contract](./contracts.md), it will instead generate a `#[kani::proof_for_contract]` harness and verify the contract:
```
Autoharness: Checking function foo's contract against all possible inputs...
<VERIFICATION RESULTS>
```

Similarly, Kani will detect the presence of [loop contracts](./loop-contracts.md) and verify them.

Thus, `-Z autoharness` implies `-Z function-contracts` and `-Z loop-contracts`, i.e., opting into the experimental
autoharness feature means that you are also opting into the function contracts and loop contracts features.

Kani generates and runs these harnesses internally—the user only sees the verification results.

### Options
The `autoharness` subcommand has options `--include-pattern [REGEX]` and `--exclude-pattern [REGEX]` to include and exclude particular functions using regular expressions.
When matching, Kani prefixes the function's path with the crate name. For example, a function `foo` in the `my_crate` crate will be matched as `my_crate::foo`.

The selection algorithm is as follows:
- If only `--include-pattern`s are provided, include a function if it matches any of the provided patterns.
- If only `--exclude-pattern`s are provided, include a function if it does not match any of the provided patterns.
- If both are provided, include a function if it matches an include pattern *and* does not match any of the exclude patterns. Note that this implies that the exclude pattern takes precedence, i.e., if a function matches both an include pattern and an exclude pattern, it will be excluded.

Here are some examples:

```bash
# Include functions containing foo but not bar
kani autoharness -Z autoharness --include-pattern 'foo' --exclude-pattern 'bar'

# Include my_crate::foo exactly
kani autoharness -Z autoharness --include-pattern '^my_crate::foo$'

# Include functions in the foo module, but not in foo::bar
kani autoharness -Z autoharness --include-pattern 'foo::.*' --exclude-pattern 'foo::bar::.*'

# Include functions starting with test_, but not if they're in a private module
kani autoharness -Z autoharness --include-pattern 'test_.*' --exclude-pattern '.*::private::.*'

# This ends up including nothing since all foo::bar matches will also contain bar.
# Kani will emit a warning that these options conflict.
kani autoharness -Z autoharness --include-pattern 'foo::bar' --exclude-pattern 'bar'
```

Note that because Kani prefixes function paths with the crate name, some patterns might match more than you expect.
For example, given a function `foo_top_level` inside crate `my_crate`, the regex `.*::foo_.*` will match `foo_top_level`, since Kani interprets it as `my_crate::foo_top_level`.
To match only `foo_` functions inside modules, use a more specific pattern, e.g. `.*::[^:]+::foo_.*`.

Autoharness also accepts a `--list` argument, which runs the [list subcommand](../list.md) including automatic harnesses.

For a full list of options, run `kani autoharness --help`.

## Example
Using the `estimate_size` example from [First Steps](../../tutorial-first-steps.md) again:
```rust
{{#include ../../tutorial/first-steps-v1/src/lib.rs:code}}
```

We get:

```
# cargo kani autoharness -Z autoharness
Autoharness: Checking function estimate_size against all possible inputs...
RESULTS:
Check 3: estimate_size.assertion.1
         - Status: FAILURE
         - Description: "Oh no, a failing corner case!"
[...]

Verification failed for - estimate_size
Complete - 0 successfully verified functions, 1 failures, 1 total.
```

## Request for comments
This feature is experimental and is therefore subject to change.
If you have ideas for improving the user experience of this feature,
please add them to [this GitHub issue](https://github.com/model-checking/kani/issues/3832).

## Raw Pointers
For a function with raw pointer arguments (`*const T`/`*mut T`, including nested raw pointers),
the generated harness produces pointers in a nondeterministic allocation state, provided that the
pointee type implements `Arbitrary` (or can derive it). Each generated pointer is aligned and is either:
- null,
- out of bounds of its allocation (and thus invalid for reads or writes), or
- valid: pointing to a nondeterministic value of the pointee type, which stays allocated for the entire harness.

As a consequence, a function that dereferences a raw pointer argument without being able to rule out
the null and out-of-bounds states will fail verification. For safe functions, such a failure points at a
real robustness issue, since safe code can pass any pointer value. For functions whose safety relies on
caller obligations (e.g., `unsafe fn`s with documented preconditions), add
[function contracts](contracts.md) with Kani's
[memory predicates](https://model-checking.github.io/kani/crates/doc/kani/mem/index.html) such as
`#[kani::requires(kani::mem::can_dereference(ptr))]`: the automatic contract harness assumes the
precondition, which excludes the invalid pointer states.

Current limitations of the generated pointers:
- Pointers are always aligned; misaligned-pointer bugs are not covered.
- No pointers to deallocated objects are generated (Kani's memory predicates cannot reason about those).
- Distinct pointer arguments never alias each other, and the pointee is always initialized in the valid state.
- Raw pointers are only supported as direct arguments (possibly nested in other raw pointers), not behind
  references or inside user-defined types.

## Type Safety Invariants
If a type implements the [`Invariant`](https://model-checking.github.io/kani/crates/doc/kani/trait.Invariant.html) trait,
Kani assumes that the nondeterministic struct and enum values it generates for automatic harnesses respect the type's safety invariant,
i.e., each generated value `v` satisfies `v.is_safe()`.
This assumption applies to nested values as well: if a field of a generated value has a struct or enum type that implements `Invariant`,
the field's safety invariant is assumed to hold, even if the enclosing type does not implement `Invariant` itself.
Invariants implemented for non-ADT types (e.g., tuples or arrays) are currently not assumed.

This matches the [Unsafe Code Guidelines' definition of a safety invariant](https://rust-lang.github.io/unsafe-code-guidelines/glossary.html#validity-and-safety-invariant):
safe code is allowed to assume that the values it receives uphold their types' safety invariants,
so verifying a function against invariant-violating inputs would produce spurious counterexamples.

Note that automatic harnesses do not *assert* type invariants, e.g., they do not check that a function's return value satisfies `is_safe()`.
To verify that a function preserves an invariant, add a [function contract](contracts.md) such as `#[kani::ensures(|result| result.is_safe())]`;
autoharness verifies a function against its contract if it has one.

## Bounded Arguments (opt-in: `--bounded-arguments`)
By default, autoharness only generates harnesses whose nondeterministic inputs cover *all*
possible values, so that a successful result carries Kani's usual guarantee. Some argument
types (e.g. slices) can only be generated in a *bounded* fashion; because a bug that requires
a larger input would then be missed, these are **disabled by default** and require the
`--bounded-arguments` option. Functions that would become eligible with the option are
reported in the skipped-functions table with reason "Requires --bounded-arguments". Harnesses
that use bounded values are marked **"(bounded)"** in the summary table, and a note after the
table repeats the caveat.

With `--bounded-arguments`, for a function with `&[T]`/`&mut [T]` arguments (where `T`
implements or can derive `Arbitrary`) or `&str` arguments, the generated harness produces a
slice of nondeterministic length, backed by nondeterministic storage that lives for the entire
harness: **up to 16 elements** for slices and **up to 4 bytes** for strings. Strings cover all
valid UTF-8 contents up to the bound (the generated string is the longest valid-UTF-8 prefix of
nondeterministic bytes, the same approach as `String`'s `BoundedArbitrary` implementation); the
smaller bound reflects the cost of reasoning about UTF-8 for symbolic execution. The bounds are
chosen to stay below the default loop-unwinding bound of 20, so that loops over the slice can
be fully unwound by default.

Nested slice references (e.g. `&&[u8]`) and slices inside user-defined types remain unsupported.

## Limitations
### Arguments Implementing Arbitrary
Kani will only generate an automatic harness for a function if it can represent each of its arguments nondeterministically.
By default, it must be able to do so *without bounds*; the `--bounded-arguments` option (see above) relaxes this to
additionally allow argument types that can only be represented up to a bound, such as slice (`&[T]`/`&mut [T]`) and
string (`&str`) references.
In technical terms, each of the arguments needs to implement the `Arbitrary`
trait or be capable of deriving it, or be a reference (mutable or immutable)
where any of the prior requirements is fulfilled by the referenced type.
Kani will detect if a struct or enum could implement `Arbitrary` and derive it automatically.
Note that this automatic derivation feature is only available for autoharness.

### Reference and Pointer Arguments
Each reference, pointer, slice, or string argument is generated from its own independent
nondeterministic storage. Autoharness therefore does *not* explore aliasing *between* distinct
arguments: for example, given `fn f(a: &T, b: &T)`, the generated harness always passes two
references to separate allocations, so `a` and `b` never share an address (`core::ptr::eq(a, b)`
is always `false`), even though a caller could pass the same reference twice. A successful
automatic harness is thus an underapproximation with respect to caller-controlled aliasing, in
the same way that verifying a single monomorphization is an underapproximation for [generic
functions](#generic-functions). This applies to all reference/pointer arguments and is
independent of the length bound that `--bounded-arguments` introduces.
Modeling caller-controlled aliasing between arguments is tracked in
[#4750](https://github.com/model-checking/kani/issues/4750).

### Generic Functions
For a generic function, Kani generates a harness for a single monomorphic instantiation of the function:
it substitutes every type parameter with the first candidate from a fixed list of primitive types
(starting with `i32`) such that all of the function's trait bounds are satisfied, and erases lifetime parameters.
For example, given:
```rust
fn foo<T: Eq>(x: T, y: T) {
    if x == y {
        panic!("x and y are equal");
    }
}
```
Kani generates and runs a harness that verifies `foo::<i32>`, and the summary table shows the
instantiated name, e.g.:
```
| Crate    | Selected Function | Kind of Automatic Harness | Verification Result |
| my_crate | foo::<i32>        | #[kani::proof]            | Failure             |
```
Note that verifying a single instantiation is an underapproximation of all of the function's possible behaviors:
a successful result for `foo::<i32>` does not imply that other instantiations of `foo` are also safe.
Kani makes this explicit by displaying the instantiated name of the verified function.

`usize` const generic parameters (e.g. array lengths) are instantiated with the value 2.

Kani skips a generic function (with skip reason "Generic Function") if:
- no candidate type satisfies the function's trait bounds, or
- the function has non-`usize` const generic parameters, which Kani does not instantiate yet.

If some caller of a generic function is eligible for an automatic harness, then additional monomorphized
versions of the generic function may still be reachable (and thus verified) through the caller's harness.
