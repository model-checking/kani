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

### Parallel verification

Since autoharness typically generates many harnesses, it verifies them in parallel by default,
i.e. as if `--jobs` (the thread pool's default number of threads, normally one per logical CPU)
and `--output-format=terse` had been passed. Note that plain `kani`/`cargo kani` verification is
unaffected and remains sequential by default.

To override the default:
- `-j <N>` / `--jobs=<N>` caps the number of harnesses verified concurrently. Each thread runs
  its own CBMC process, so peak memory grows with the number of threads; lower `<N>` if a run
  exhausts the available memory. `--jobs=1` keeps the terse output but verifies sequentially.
- `--output-format=regular` verifies harnesses sequentially, with Kani's default, more detailed
  per-check output. (Parallel verification requires terse output, because interleaved detailed
  output is hard to read; passing `--jobs` together with `--output-format=regular` is therefore
  an error.)

In parallel runs each harness result line is prefixed with the thread that produced it, and
results arrive in nondeterministic order; the summary table printed at the end is always sorted.

### Constructor-based generation (--constructor-args)

By default, when a type does not implement `Arbitrary`, Kani synthesizes values field by field.
For types whose private fields carry a representation invariant (e.g. a date type storing a
packed, validated ordinal), raw field synthesis can produce values that violate the invariant,
causing false alarms in every harness that generates the type. With `--constructor-args`, Kani
instead generates values of private-field struct types through one of the type's own
constructors, preferring (in order):

1. An *assert-guarded representation constructor*: an `unsafe`, `#[doc(hidden)]`, or
   `*_unchecked`-named associated function returning `Self` directly, whose preconditions are
   stated as assertions (e.g. `debug_assert!`) rather than validated returns. Kani inlines its
   body with nondeterministic arguments and converts every validity statement — `kani::assert`
   and `assert_unchecked` calls, panic entry points, and overflow (`Assert`) checks — into an
   assumption, so the constructor's own assertions filter the arguments down to the values the
   crate considers valid. Calls the constructor makes to further assert-guarded helpers are
   inlined recursively (bounded in depth and size). Visibility is irrelevant here, since the
   body is inlined rather than called.
2. Otherwise, a *checked public constructor*: a public associated function returning `Self`,
   `Option<Self>`, or `Result<Self, E>`, called with nondeterministic arguments (assuming
   success for the `Option<Self>`/`Result<Self, E>` shapes).

Zero-argument constructors, and constructors generic over their own parameters, are not
considered.

This option is opt-in because it under-approximates: harnesses whose values are generated this
way are marked "(ctor)" in the output, and their verification results only cover values
reachable through the chosen constructor; a bug that requires a different value will not be
found. Note also that a checked constructor which itself panics for some of its inputs (rather
than rejecting them via `Option`/`Result`) turns those inputs into harness failures, so this
option can trade one class of false alarm for another.

> **Caveat:** if the chosen constructor is *unsatisfiable* for the generated type — an
> assert-guarded constructor every argument of which trips an assertion, or a checked
> constructor that always returns `None`/`Err` — the generated body assumes `false` on all
> paths and the harness becomes **vacuous**, reporting `Success` without checking anything.
> Kani does not yet detect this case; see
> [#4757](https://github.com/model-checking/kani/issues/4757).

## Example
Using the `estimate_size` example from [First Steps](../../tutorial-first-steps.md) again:
```rust
{{#include ../../tutorial/first-steps-v1/src/lib.rs:code}}
```

We get (passing `--output-format=regular` so that the per-check detail is shown, c.f.
[Parallel verification](#parallel-verification)):

```
# cargo kani autoharness -Z autoharness --output-format=regular
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

Additionally (also requiring `--bounded-arguments`), for arguments whose type implements
[`BoundedArbitrary`](../bounded_arbitrary.md)
(e.g. `Vec<T>`, `String`, or user types deriving it), the harness generates a bounded
nondeterministic value with **bound 4** (via `kani::bounded_any`). The same caveat applies:
verification results only hold up to the bound. The smaller bound reflects that these values are
heap allocated and, for `String`, involve UTF-8 reasoning, both of which are costly for symbolic
execution.

Nested slice references (e.g. `&&[u8]`) and slices inside user-defined types remain unsupported.

## Debug and Display Implementations
For the `fmt` methods of `Debug` and `Display` implementations, the `&mut Formatter` argument
cannot be generated nondeterministically. Instead, Kani generates a harness that formats a
nondeterministic value of the implementing type into a sink that discards the output: the
`Formatter` is constructed by the core formatting machinery (so it is always valid), and panics
or undefined behavior inside the `fmt` implementation are detected as usual.
These harnesses are unbounded with respect to the implementing type (its value is generated with
`kani::any`), so they are generated by default and are unaffected by `--bounded-arguments`. If the
implementing type implements [`Invariant`](#type-safety-invariants), the generated value is assumed
to satisfy it, as for any other automatic harness.

Current limitations:
- The `Formatter` carries the default formatting parameters, i.e. it is the one that
  `format!("{:?}", value)`/`format!("{}", value)` would produce. Code paths that a `fmt`
  implementation takes only for a non-default width, precision, fill, alignment, sign, or the
  alternate (`{:#?}`) flag are therefore not covered.
- The sink never fails, so `fmt` implementations that propagate write errors with `?` are not
  verified against the error path.
- Because the harness goes through `core::fmt`, the core formatting machinery is verified along
  with the `fmt` implementation, so a reported failure may point at a location inside `core`.
- A `fmt` method that carries a [function contract](contracts.md) is not handled this way: the
  automatic contract harness calls the function directly, so it is skipped for its
  `&mut Formatter` argument like any other function Kani cannot call.

## Limitations
### Arguments Implementing Arbitrary
Kani will only generate an automatic harness for a function if it can represent each of its arguments nondeterministically.
By default, it must be able to do so *without bounds*: each argument needs to implement the `Arbitrary`
trait or be capable of deriving it, or be a reference (mutable or immutable)
where any of the prior requirements is fulfilled by the referenced type.
The `--bounded-arguments` option (see above) relaxes this to
additionally allow argument types that can only be represented up to a bound: slice (`&[T]`/`&mut [T]`) and
string (`&str`) references, and types implementing [`BoundedArbitrary`](../bounded_arbitrary.md)
(e.g. `Vec<T>`, `String`, or user types deriving it).
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
it substitutes the function's type parameters with concrete types such that all of the function's
trait bounds are satisfied, and erases lifetime parameters. Kani first tries a fixed list of
primitive types (starting with `i32`, and including the wider integer and float types) uniformly
for all parameters; if that fails, it searches per-parameter combinations, drawing additional
candidate types from the concrete implementations of the traits each parameter is bound by
(so, e.g., a parameter bound by a crate-local trait can be instantiated with a crate-local struct
implementing it). The search is capped, so functions with many type parameters or very complex
bounds may still be skipped.
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
