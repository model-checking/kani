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
harness: **up to 16 elements** for slices and **up to 8 bytes** for strings. Strings cover all
valid UTF-8 contents up to the bound (the generated string is the longest valid-UTF-8 prefix of
nondeterministic bytes, the same approach as `String`'s `BoundedArbitrary` implementation); the
smaller bound reflects the cost of reasoning about UTF-8 for symbolic execution. The bounds are
chosen to stay below the default loop-unwinding bound of 20, so that loops over the slice can
be fully unwound by default.

Additionally (also requiring `--bounded-arguments`), for arguments whose type implements
[`BoundedArbitrary`](https://model-checking.github.io/kani/reference/experimental/bounded-arbitrary.html)
(e.g. `Vec<T>`, `String`, or user types deriving it), the harness generates a bounded
nondeterministic value with **bound 4** (via `kani::bounded_any`). The same caveat applies:
verification results only hold up to the bound. The smaller bound reflects that these values are
heap allocated and, for `String`, involve UTF-8 reasoning, both of which are costly for symbolic
execution.

Nested slice references (e.g. `&&[u8]`) and slices inside user-defined types remain unsupported.

## Limitations
### Arguments Implementing Arbitrary
Kani will only generate an automatic harness for a function if it can represent each of its arguments nondeterministically, without bounds.
In technical terms, each of the arguments needs to implement the `Arbitrary`
trait or be capable of deriving it, or be a reference (mutable or immutable)
where any of the prior requirements is fulfilled by the referenced type.
Kani will detect if a struct or enum could implement `Arbitrary` and derive it automatically.
Note that this automatic derivation feature is only available for autoharness.

### Generic Functions
The current implementation does not generate harnesses for generic functions.
For example, given:
```rust
fn foo<T: Eq>(x: T, y: T) {
    if x == y {
        panic!("x and y are equal");
    }
}
```
Kani would report that no functions were eligible for automatic harness generation.

If, however, some caller of `foo` is eligible for an automatic harness, then a monomorphized version of `foo` may still be reachable during verification.
For instance, if we add `main`:
```rust
fn main() {
    let x: u8 = 2;
    let y: u8 = 2;
    foo(x, y);
}
```
and run the autoharness subcommand, we get:
```
Autoharness: Checking function main against all possible inputs...

Failed Checks: x and y are equal
 File: "src/lib.rs", line 3, in foo::<u8>

VERIFICATION:- FAILED
```
