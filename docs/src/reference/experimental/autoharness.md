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

For a full list of options, run `kani autoharness --help`.

To run autoharness on the Rust standard library itself, pass `--std` to the standalone
command and point it at the `library` directory of a standard library checkout, such as the
one in [verify-rust-std](https://github.com/model-checking/verify-rust-std):

```
# kani autoharness -Z autoharness --std <PATH>/library
```

The path must be a directory named `library`. Kani builds it with `cargo -Z build-std` through
a temporary crate in the target directory, as `kani verify-std` does. `--std` is only available
on `kani autoharness`, not on `cargo kani autoharness`.

When verifying the standard library, only the Kani definitions available to `core` can be
used. The argument models that need `alloc` (`Box`, `Rc` and `Arc` arguments, unbounded
primitive slices and `Vec`s, and the closure models for `Fn`-bound type parameters) are
therefore not available with `--std`. Affected arguments are generated with bounded values
where `core` provides a bounded generator and `--bounded-arguments` is passed; otherwise the
function is skipped.

Concrete playback (`-Z concrete-playback`) is not supported by the `autoharness` subcommand.

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

## Selecting functions

AutoHarness considers functions defined in the crate being verified, including free functions, inherent methods, trait-implementation methods, and trait methods with default implementations. Functions from dependencies are not candidates. For each eligible function, Kani selects one automatic harness; functions that fail a selection check are skipped. Kani reports selected functions and visible skip reasons before verification, unless `--quiet` is specified.

Selection first attempts to instantiate generic functions, then applies the include and exclude patterns, and finally checks whether the function's arguments can be generated. The details of argument generation are described in [Generating harnesses](#generating-harnesses).

### Include and exclude patterns

The `autoharness` subcommand has options `--include-pattern [REGEX]` and `--exclude-pattern [REGEX]` to include and exclude particular functions using regular expressions.
When matching, Kani prefixes the function's path with the crate name. For example, a non-generic function `foo` in the `my_crate` crate is matched as `my_crate::foo`.
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

Patterns match the **instantiated name** of a generic function. For example, `my_crate::foo::<i32>` does not match `^my_crate::foo$`, although an unanchored `foo` pattern matches it. Regular expressions are unanchored unless explicitly anchored; invalid expressions and patterns containing whitespace are rejected. Kani also warns if an include-pattern string contains an exclude-pattern string.

Generic instantiation happens before include/exclude filtering. Consequently, a generic function for which Kani cannot find an instantiation is reported with the `Generic Function` skip reason even if the function would not match the supplied filters.

### Listing functions (--list)

AutoHarness also accepts `--list`, which compiles the project and runs the [list subcommand](../list.md), including automatic harnesses, **without running verification**. Unless `--quiet` is passed, Kani first prints the selected-functions and skipped-functions tables.

Use `--list --format <FORMAT>` to choose the output format:

| Format | Output |
| --- | --- |
| `pretty` (default) | Print the list to the terminal. |
| `markdown` | Write `kani-list.md` in the current directory. |
| `json` | Write `kani-list.json` in the current directory. |

For file-based formats, Kani prints the path of the written file. The `--format` option requires `--list`. Combining `--quiet` with `--list --format pretty` is not supported.

For example:

```bash
# List functions and harnesses without verification
cargo kani autoharness -Z autoharness --list

# Write the list as JSON
cargo kani autoharness -Z autoharness --list --format json
```

### Skip reasons

AutoHarness records one skip reason per skipped function: the first failed check in its selection process. The skipped-functions table has `Crate`, `Skipped Function`, and `Reason for Skipping` columns. Reasons visible to users include:

| Reason shown | Meaning |
| --- | --- |
| `Can only be called at compile time` | The function is marked for compile-time execution and cannot be selected for an automatic harness. |
| `Generic Function: <detail>` | Kani could not find a supported instantiation satisfying the function's constraints. The detail provides the specific cause; see [Generic Functions](#generic-functions). |
| `The function does not have a body` | There is no function body available to verify, as with a trait method without a default implementation. |
| `Did not match provided filters` | The function was excluded by the include/exclude patterns. |
| `Unsupported variadic calling convention` | A variadic function uses an unsupported, non-C calling convention. |
| `Missing Arbitrary implementation for argument(s) x: T, y: U` | At least one argument cannot be generated using a supported argument model. The argument names and types depend on the function. |
| `Requires --bounded-arguments for argument(s) x: T` | At least one argument requires opting into bounded generation. The argument names and types depend on the function. |

Missing argument names are displayed as `_`. If a function has both unsupported arguments and arguments requiring bounded generation, `Missing Arbitrary implementation` is the skip reason reported. See [Generating harnesses](#generating-harnesses) for the supported argument models and [Bounded Arguments](#bounded-arguments-opt-in---bounded-arguments) for the opt-in behavior.

Kani's internal implementation functions and functions already used as proof harnesses may be excluded internally with the `Kani implementation` reason, but these entries are **not displayed** in the skipped-functions table. The selected-functions table contains `Crate` and `Selected Function` columns. Where a generic instantiation is chosen, the tables display the instantiated function name (for example, `foo::<i32>`); a generic function that cannot be instantiated is listed under its uninstantiated name.

### Generic Functions

For a generic function, Kani generates a harness for a **single monomorphic instantiation**: it substitutes concrete types for the function's type parameters, requires the instantiation to satisfy the trait bounds and `where` clauses, and erases lifetime parameters.

Kani first tries the following primitive candidate types, using the same type for each type parameter: `i32`, `u32`, `usize`, `u8`, `i64`, `u64`, `f64`, `f32`, `bool`, and `char`. If no uniform choice works, it searches combinations of candidate types for individual parameters. Alongside the primitives, the candidates can include up to 16 concrete types derived from implementations of the parameter's trait bounds (for example, a crate-local struct implementing a crate-local trait). This combinatorial search is limited to 256 attempts. If it finds no suitable instantiation, Kani makes one final attempt using `()` for every type parameter.

For example, given:

```rust
fn foo<T: Eq>(x: T, y: T) {
    if x == y {
        panic!("x and y are equal");
    }
}
```

Kani generates and runs a harness that verifies `foo::<i32>`, and the summary table shows the instantiated name, e.g.:

```text
| Crate    | Selected Function | Kind of Automatic Harness | Verification Result |
| my_crate | foo::<i32>        | #[kani::proof]            | Failure             |
```

Verifying a single instantiation is an underapproximation of all of the function's possible behaviors: a successful result for `foo::<i32>` does not imply that other instantiations of `foo` are also safe. Kani makes the chosen instantiation explicit by displaying its name. See [Single monomorphization](#single-monomorphization) for the implications of this limitation.

**Const generic parameters.** `usize` const generic parameters, such as array lengths, are instantiated with the value `2`, so an instantiated function name might include `::<2>`. Other const-generic parameter types are currently unsupported. Kani also skips a function if its const-generic parameter can reach a `const {}` block whose evaluation may constrain that parameter (for example, `const { assert!(N >= 4) }`). This check is conservative: Kani may skip a function even if the substituted value would satisfy the assertion, rather than risk an invalid compile-time evaluation.

**Function-trait bounds.** For parameters bounded by `Fn`, `FnMut`, or `FnOnce`, Kani can try nondeterministic function models instead of skipping the generic function. The models support zero to three by-value inputs, or one to two inputs involving shared references (including mixed reference/value arguments). The return type must implement `Arbitrary`. Models can also be selected for signatures that involve the generic function's other type parameters, when the resulting bounds can be satisfied. If no available model is suitable, the function may be skipped as a generic function. See [Fn-bound closures](#fn-bound-closures) for the model's behavior and limitations.

**Other instantiation failures.** Kani may skip a generic function when no candidate satisfies its bounds or the instantiation search reaches its limit. It also rejects candidate instantiations that would call a `simd_*` intrinsic in the function's own body with incompatible non-SIMD types or an invalid SIMD comparison-result type; another valid candidate may still be selected. The `Generic Function` skip-reason detail identifies applicable failures, including unsupported const generics, const-generic preconditions, SIMD constraints, exhausted search attempts, or unsatisfied trait bounds.

If some caller of a generic function is eligible for an automatic harness, then additional monomorphized versions of the generic function may still be reachable (and thus verified) through the caller's harness.

After Kani selects a function and, when necessary, an instantiation, it generates the harness arguments as described in [Generating harnesses](#generating-harnesses).

## Generating harnesses

### Arguments

#### Unbounded (default)

Kani uses the following argument generators by default, without requiring
`--bounded-arguments`. Their supported types and modeling limits are described below.

##### Arguments Implementing Arbitrary
Kani generates an automatic harness when it can generate each of the function's arguments
nondeterministically. For an argument implementing [`Arbitrary`](../arbitrary.md), it uses
`kani::any()`. For a shared or mutable reference to such a type, it generates a value of the
referenced type in storage that lives for the entire harness, then passes a reference to it.

For supported structs and enums without an `Arbitrary` implementation, Kani can synthesize
one by generating fields and, for enums, selecting a variant nondeterministically. Private
fields are supported, but field generation has additional restrictions: bare reference
fields and `NonNull` fields (including wrappers such as `Option<NonNull<T>>`) are not
synthesized, and types with lifetime arguments cannot use automatic derivation. Other
fields must implement `Arbitrary` or meet the recursive derivation requirements. Automatic
derivation is available only for autoharness and does not require adding a derive attribute
to the type. The argument models below do not make unsupported field types derivable.

The following sections describe additional argument models. Types that require a bounded
generator are covered under [Bounded Arguments](#bounded-arguments-opt-in---bounded-arguments).

##### Raw Pointers
For a function with raw pointer arguments (`*const T`/`*mut T`, including nested raw pointers),
the generated harness produces pointers in a nondeterministic allocation state, provided that the
pointee type implements `Arbitrary` (or can derive it). Each generated pointer is aligned and is either:
- null,
- out of bounds of its allocation (and thus invalid for reads or writes), or
- valid: pointing to a nondeterministic value of the pointee type, which stays allocated for the entire harness.

Therefore, a function that dereferences a raw pointer argument without being able to rule out
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

##### Box, Rc and Arc

`Box<T>`, `Rc<T>` and `Arc<T>` arguments are supported when the sized pointee type `T`
implements `Arbitrary` or Kani can derive it automatically. Kani generates a nondeterministic
pointee and constructs a new smart pointer around it. These arguments require no
`--bounded-arguments` flag and do not add a "(bounded)" marker.

The generation models require allocation support and use the default allocator. They cover
the pointee's generated values; `Rc` and `Arc` start with one strong reference and no weak
references, so states with existing owners or weak references are not explored. Unsized
pointees such as `str` and `[T]` are outside these models; some types, such as `Box<[T]>`,
can instead be generated through `BoundedArbitrary` with `--bounded-arguments`.

##### Primitive slices and Vec

Direct `&[T]`, `&mut [T]` and `Vec<T>` arguments are generated without a length bound when
`T` is a primitive integer or floating-point type, for example `u8`, `i32` or `f64`. Each
argument uses a fresh allocation with nondeterministic length and contents. The length is
restricted only by Rust's allocation-size requirements. Mutable slices have exclusive
backing storage; the generated `Vec` uses the global allocator and a capacity of
`len.max(1)`.

These models require allocation support. They are enabled by default, do not add a
"(bounded)" marker, and take precedence over bounded generation even when
`--bounded-arguments` is passed. `--slice-bound` and `--bounded-arbitrary-bound` do not limit
them. Element types such as `bool`, `char`, `NonZeroU32` and user-defined structs use the
bounded generators instead, when supported.

Unbounded argument length does not remove the loop-unwinding bound. A function that
iterates over the input can fail an unwinding assertion if that bound is insufficient;
see [Loop unwinding](../../tutorial-loop-unwinding.md). Nested slice references such as
`&&[u8]` and slices inside user-defined types are outside these argument models.

##### Fn-bound closures

For supported generic parameters bounded by `Fn`, `FnMut` or `FnOnce`, Kani supplies a
function item that returns a fresh `kani::any()` value on each call. The return type must
implement `Arbitrary`. Repeated calls can return different values, even for the same inputs.
The function item itself needs no `Arbitrary` implementation.

Models support zero through three arguments passed by value. For shared-reference inputs,
models support one or two arguments, including a mix of shared references and values.
The referenced types must be sized. These reference models can satisfy higher-ranked bounds
such as `for<'a> Fn(&'a T) -> R`; there is no corresponding model for higher-ranked mutable
references. Eligibility also depends on satisfying the generic function's other bounds;
see [Generic Functions](#generic-functions).

These models check the function under verification against arbitrary callback results.
They do not execute a particular closure's body or model its captures, mutations or other
side effects. Use a handwritten harness when verification depends on a particular callback
implementation.

##### Formatting Trait Implementations
For the `fmt` methods of `Debug`, `Display`, `Binary`, `Octal`, `LowerHex`, `UpperHex`, `LowerExp`,
`UpperExp` and `Pointer` implementations, Kani uses a special harness rather than the
general [bounded `Formatter` model](#bounded-arguments-opt-in---bounded-arguments). It formats a
nondeterministic value of the implementing type into a sink that discards the output: the
`Formatter` is constructed by the core formatting machinery (so it is always valid), and panics
or undefined behavior inside the `fmt` implementation are detected as usual.
These harnesses are unbounded with respect to the implementing type (its value is generated with
`kani::any`), so they are generated by default and are unaffected by `--bounded-arguments`. If the
implementing type implements [`Invariant`](#type-safety-invariants), the generated value is assumed
to satisfy it, as for any other automatic harness.

Current limitations:
- The `Formatter` carries the default formatting parameters, i.e. it is the one that `format!`
  with the trait's bare specifier (`{:?}`, `{}`, `{:x}`, ...) would produce. Code paths that a `fmt`
  implementation takes only for a non-default width, precision, fill, alignment, sign, or the
  alternate flag (`{:#?}`, `{:#x}`, and so on) are therefore not covered.
- The sink never fails, so `fmt` implementations that propagate write errors with `?` are not
  verified against the error path.
- Because the harness goes through `core::fmt`, the core formatting machinery is verified along
  with the `fmt` implementation, so a reported failure may point at a location inside `core`.
- A `fmt` method that carries a [function contract](contracts.md) is not handled this way: the
  automatic contract harness calls the function directly, so its `&mut Formatter` argument comes
  from the bounded `Formatter` model and requires `--bounded-arguments`.

#### Opt-in under-approximations

##### Bounded Arguments (opt-in: `--bounded-arguments`)
Some argument types require a bounded generator, including strings and slices whose
elements are not primitive integers or floats. Because a bug requiring a larger input
would be missed, these generators are **disabled by default** and require
`--bounded-arguments`. Functions that would become eligible with the option are reported
in the skipped-functions table with reason "Requires --bounded-arguments". Harnesses that
use bounded values are marked **"(bounded)"** in the summary table, and a note after the
table repeats this limitation.

With `--bounded-arguments`, for a function with `&[T]`/`&mut [T]` arguments outside the
[unbounded primitive models](#primitive-slices-and-vec) (where `T` implements or can derive
`Arbitrary`), or with `&str`, `&CStr`, `&ByteStr` or `&Wtf8` arguments, the
generated harness produces a slice of nondeterministic length, backed by nondeterministic storage
that lives for the entire harness: by default **up to 16 elements** for slices and byte strings,
**up to 4 bytes** for strings and WTF-8 strings, and **up to 15 bytes** plus the terminating NUL
for C strings (which follow the slice bound, less one for the NUL). Strings cover all
valid UTF-8 contents up to the bound (the generated string is the longest valid-UTF-8 prefix of
nondeterministic bytes, the same approach as `String`'s `BoundedArbitrary` implementation); the
smaller bound reflects the cost of reasoning about UTF-8 for symbolic execution. WTF-8 strings are
generated the same way, as the longest well-formed WTF-8 prefix, so they also cover values holding
unpaired surrogate code points. The bounds are
chosen to stay below the default loop-unwinding bound of 20, so that loops over the slice can
be fully unwound by default.

The bounds are configurable with `--slice-bound` and `--string-bound` (and
`--bounded-arbitrary-bound`, see below). A larger bound covers more inputs at a higher solver
cost. A bound that is not below the effective loop-unwinding bound (`--default-unwind`, 20 by
default) leaves loops over such an argument only partially unwound, and Kani warns when a
configured bound reaches it. The note printed after the summary table reports the bounds a run
actually used.

`&Formatter` and `&mut Formatter` arguments (also requiring `--bounded-arguments`) are generated by
`any_formatter`: a `Formatter` over a sink that discards its output, with every formatting option
nondeterministic and its width and precision bounded by the slice bound. The sink never fails, so
paths that handle a write error are not reached.

Additionally (also requiring `--bounded-arguments`), for arguments whose type implements
[`BoundedArbitrary`](../bounded_arbitrary.md)
(e.g. `Vec<bool>`, `String`, or user types deriving it), the harness generates a bounded
nondeterministic value with a **default bound of 4** (via `kani::bounded_any`), overridable
with `--bounded-arbitrary-bound`. The same caveat applies:
verification results only hold up to the bound. The smaller bound reflects that these values are
heap allocated and, for `String`, involve UTF-8 reasoning, both of which are costly for symbolic
execution.

Nested slice references (e.g. `&&[u8]`) and slices inside user-defined types remain unsupported.

<!-- TODO(#4979 item 8): bound flags require --bounded-arguments -->

##### Constructor-based generation (--constructor-args)

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

`--constructor-args` additionally enables *mined-invariant filtering*: if a type has no viable
constructor but its own `&self` methods assert conditions over its fields (see
[Mined invariants](#mined-invariants) below), the generated value is
constrained to satisfy those mined conditions via `kani::assume`. This covers types with no
usable constructor, at lower formula cost than constructor inlining.

This option is opt-in because it under-approximates: harnesses whose values are generated or
constrained this way are marked "(ctor)" in the output. That marker therefore covers all of the
mechanisms above — assert-guarded/checked constructors *and* mined-invariant filtering — and
signals that verification results only cover values the chosen mechanism admits; a bug that
requires a different value will not be found. Note also that a checked constructor which itself
panics for some of its inputs (rather than rejecting them via `Option`/`Result`) turns those
inputs into harness failures, so this option can trade one class of false alarm for another.

See [Soundness caveats](#soundness-caveats).

### Assumptions

#### Contracts (requires)

<!-- TODO(#4979): contract preconditions are assumed -->

#### Type Safety Invariants
If a type implements the [`Invariant`](https://model-checking.github.io/kani/crates/doc/kani/trait.Invariant.html) trait,
Kani assumes that the nondeterministic struct and enum values it generates for automatic harnesses respect the type's safety invariant,
i.e., each generated value `v` satisfies `v.is_safe()`.
This assumption applies to nested values as well: if a field of a generated value has a struct or enum type that implements `Invariant`,
the field's safety invariant is assumed to hold, even if the enclosing type does not implement `Invariant` itself.
Invariants implemented for non-ADT types (e.g., tuples or arrays) are currently not assumed.

This matches the [Unsafe Code Guidelines' definition of a safety invariant](https://rust-lang.github.io/unsafe-code-guidelines/glossary.html#validity-and-safety-invariant):
safe code is allowed to assume that the values it receives uphold their types' safety invariants,
so verifying a function against invariant-violating inputs would produce spurious counterexamples.

#### Layout niches

<!-- TODO(#4979 item 4): layout niche assumptions -->

#### Mined invariants

Many types state their representation invariant implicitly, as assertions over `self`'s fields
in their own `&self` methods (e.g. `assert!(self.value >= 1)`). Kani can *mine* these: it
extracts a condition as a type invariant when the assertion executes on every normal return of
the method (post-dominance), its condition reads only `self`'s fields and constants (a pure,
call-free slice), and the same conjunct is asserted in at least two distinct methods (so that
method-local preconditions are not mistaken for type invariants). For enums, a conjunct read
from a matched variant is guarded by that variant's discriminant.

Mined invariants are used in two ways:

- Under `--constructor-args`, they are *assumed* for generated values (as described above), and
  such harnesses are marked "(ctor)".

  See [Soundness caveats](#soundness-caveats).

### Checks

#### Undefined behavior and panics

<!-- TODO(#4979): undefined behavior and panic checks -->

#### Contracts (ensures)
Automatic harnesses do not *assert* type invariants, e.g., they do not check that a function's return value satisfies `is_safe()`.
To verify that a function preserves an invariant, add a [function contract](contracts.md) such as `#[kani::ensures(|result| result.is_safe())]`;
autoharness verifies a function against its contract if it has one.

#### `--check-invariants`

- With `--check-invariants`, they are *checked* on the values returned by verified functions:
  Kani asserts that each returned value (direct `T`, `&T`, or the payload of an
  `Option<T>`/`Result<T, E>` — `None`/`Err` pass vacuously) satisfies the type's mined
  invariants. A failure is reported as a distinct property class naming the asserting methods;
  because the mined predicate is heuristic, a failure means the returned value *would* trip the
  type's own assertions when used, which may or may not indicate a bug in the returning
  function.

## Verifying and reading results

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

### Default settings

Unless you pass them yourself, `kani autoharness` sets two verification options so that a
single harness cannot hold up the whole run:

- `--harness-timeout 60s`: a harness whose verification takes longer than 60 seconds is
  stopped and reported as a failure.
- `--default-unwind 20`: loops are unwound up to 20 times. Automatic harnesses carry no
  `#[kani::unwind]` attribute, so this bound applies to every loop unless you pass
  `--unwind`. Kani's unwinding checks stay enabled, so a loop that needs more iterations fails
  an unwinding assertion instead of being silently cut short.

To change either value, pass the option explicitly. When any automatic harness fails, Kani
prints a reminder of these two defaults after the summary table, because the failure may come
from a timeout or an insufficient unwinding bound rather than from a bug in the function. In
that case, try larger values or, where possible, add a [loop contract](./loop-contracts.md).
The default bounds for [bounded arguments](#bounded-arguments-opt-in---bounded-arguments) are
chosen to stay below the unwinding bound, and Kani warns if a configured bound reaches it.

Verification itself runs in parallel by default; see
[Parallel verification](#parallel-verification).

### Summary table and markers

A run of `kani autoharness` prints its results in three stages:

1. Before verification, unless `--quiet` is passed, the tables of selected and skipped
   functions (see [Skip reasons](#skip-reasons)).
2. During verification, the result of each harness as it finishes.
3. After verification, the `Autoharness Summary` table, followed by a line of the form
   `Complete - N successfully verified functions, M failures, T total.` If the crate also
   contains manual harnesses, they are verified in the same run and reported separately in a
   `Manual Harness Summary`.

The summary table has the columns `Crate`, `Selected Function`, `Kind of Automatic Harness`
and `Verification Result`. Rows are sorted by function name, with successes listed before
failures. For a generic function, `Selected Function` shows the verified instantiation, e.g.
`foo::<i32>`. `Kind of Automatic Harness` is `#[kani::proof]`, or
`#[kani::proof_for_contract]` when the function has a [function contract](contracts.md), and
may carry one or both of these markers:

- **"(bounded)"**: some arguments were generated with bounded values under
  `--bounded-arguments`. A note after the table lists the bounds the run used (slice length,
  string length, and the `BoundedArbitrary` bound) and repeats that the result only holds up
  to them.
- **"(ctor)"**: some values were generated through one of a type's own constructors, or
  constrained by mined invariants, under
  [`--constructor-args`](#constructor-based-generation-constructor-args). A note after the
  table repeats that the result only covers values reachable through that constructor.

A `Success` without either marker still has the limitations described in
[Soundness caveats](#soundness-caveats).

## Soundness caveats

A `Success` result means that Kani found no failing check for any of the inputs the harness
generated. The cases below are where those inputs, or the instantiation being verified, do not
cover everything a caller could do, so a `Success` can still miss a bug. With Kani's default
checks, a harness that times out or needs more loop unwinding than the bound allows is reported
as a failure, not a success (see [Default settings](#default-settings)).

### Single monomorphization

For a generic function, autoharness verifies a single instantiation (see
[Generic Functions](#generic-functions)): each type parameter is replaced by one concrete type
that satisfies its bounds, and each `usize` const generic parameter by the value 2. A `Success`
for `foo::<i32>` therefore does not imply that `foo::<u8>` or `foo::<MyType>` is also safe. The
summary table always shows the instantiated name, so it is visible which instantiation was
verified.

Type parameters bounded by `Fn`, `FnMut` or `FnOnce` are the exception: they are instantiated
with a model that covers every value a closure with that signature could return, rather than
with one particular closure (see [Other limitations](#other-limitations) for what the model does
not cover). The function's other type parameters are still verified for one choice only.

### No aliasing between arguments
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

### Bounded arguments

Harnesses marked "(bounded)" use bounded nondeterministic values for some arguments, which
`--bounded-arguments` enables (see [Bounded Arguments](#bounded-arguments-opt-in---bounded-arguments)).
Their results only hold for inputs up to those bounds: a bug that needs a longer slice or string,
or a larger `BoundedArbitrary` value, is not found. The bounds a run used are printed in a note
after the summary table. Arguments that are generated without bounds by default, such as slices
and `Vec`s of primitive integers or floats, are not affected.

### Vacuous constructors

This applies to harnesses marked "(ctor)", i.e. runs with `--constructor-args` (see
[Constructor-based generation](#constructor-based-generation-constructor-args)). If the chosen
constructor is *unsatisfiable* for the generated type — an assert-guarded constructor every
argument of which trips an assertion, or a checked constructor that always returns
`None`/`Err` — the generated body assumes `false` on all paths and the harness becomes
**vacuous**, reporting `Success` without checking anything. Kani does not yet detect this case;
see [#4757](https://github.com/model-checking/kani/issues/4757).

### Mined-invariant heuristic

This also applies only to "(ctor)" harnesses, where mined invariants are assumed for generated
values (see [Mined invariants](#mined-invariants)). The "asserted in ≥2 methods" filter is a
heuristic, not a proof of type-invariance. If two methods share a *precondition* that is not a
universal invariant, it is assumed for all generated values and may exclude otherwise-valid
inputs — a potential missed bug. This is acceptable only under the opt-in, under-approximating
`(ctor)` contract; see [#4763](https://github.com/model-checking/kani/issues/4763).

### Other limitations

The following under-approximations apply by default, and the summary table does not mark the
harnesses they affect:

- **Closures have no side effects.** The model used for `Fn`/`FnMut`/`FnOnce`-bounded type
  parameters returns a fresh nondeterministic value on every call but never modifies anything.
  It therefore does not cover closures that modify state the function can also observe, such as
  a `static` or a `Cell` that the caller also passes as an argument; see
  [#4994](https://github.com/model-checking/kani/issues/4994).
- **Raw pointers are well-behaved.** Generated raw pointers are always aligned, never point to
  deallocated memory, and point to an initialized value whenever they are valid, so bugs that
  need a misaligned or dangling pointer are not found (see [Raw Pointers](#raw-pointers)).
- **Formatting uses the default parameters and a sink that never fails.** Harnesses for `fmt`
  trait implementations only cover the default formatting parameters, not a non-default width,
  precision, fill, alignment, sign or the alternate flag, and never reach the error path of a
  failed write (see [Formatting Trait Implementations](#formatting-trait-implementations)). The
  `&Formatter` model used under `--bounded-arguments` also writes to a sink that never fails.
- **Specifications are assumed.** Function contract preconditions and the `is_safe()` predicate
  of types implementing `Invariant` are assumed for generated values, so a precondition or
  invariant that is stronger than necessary excludes inputs that a caller could pass (see
  [Contracts (requires)](#contracts-requires) and [Type Safety Invariants](#type-safety-invariants)).

## Architecture (for contributors)

<!-- TODO(#4979): architecture overview for contributors -->

## Request for comments
This feature is experimental and is therefore subject to change.
If you have ideas for improving the user experience of this feature,
please add them to [this GitHub issue](https://github.com/model-checking/kani/issues/3832).
