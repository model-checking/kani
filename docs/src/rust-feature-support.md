# Rust feature support

The table below tries to summarize the current support in Kani for
the Rust language features according to the [Rust Reference](https://doc.rust-lang.org/stable/reference/).
We use the following values to indicate the level of support:
 * **Yes**: The feature is fully supported. We are not aware of any issue with it.
 * **Partial**: The feature is at least partially supported. We are aware of some issue with
 with it.
 * **No**: The feature is not supported. Some support may be available but analyses should not be trusted.

As with all software, bugs may be found anywhere regardless of the level of support. In such cases, we
would greatly appreciate that you [filed a bug report](https://github.com/model-checking/kani/issues/new?assignees=&labels=bug&template=bug_report.md).

Reference | Feature | Support | Notes |
--- | --- | --- | --- |
3.1 | Macros By Example | Yes | |
3.2 | Procedural Macros | Yes | |
4 | Crates and source files | Yes | |
5 | Conditional compilation | Yes | |
6.1 | Modules | Yes | |
6.2 | Extern crates | Yes | |
6.3 | Use declarations | Yes | |
6.4 | Functions | Yes | |
6.5 | Type aliases | Yes | |
6.6 | Structs | Yes | |
6.7 | Enumerations | Yes | |
6.8 | Unions | Yes | |
6.9 | Constant items | Yes | |
6.10 | Static items | Yes | |
6.11 | Traits | Yes | |
6.12 | Implementations | Yes | |
6.13 | External blocks | Yes | |
6.14 | Generic parameters | Yes | |
6.15 | Associated Items | Yes | |
7 | Attributes | Yes | |
8.1 | Statements | Yes | |
8.2.1 | Literal expressions | Yes | |
8.2.2 | Path expressions | Yes | |
8.2.3 | Block expressions | Yes | |
8.2.4 | Operator expressions | Yes | |
8.2.5 | Grouped expressions | Yes | |
8.2.6 | Array and index expressions | Yes | |
8.2.7 | Tuple and index expressions | Yes | |
8.2.8 | Struct expressions | Yes | |
8.2.9 | Call expressions | Yes | |
8.2.10 | Method call expressions | Yes | |
8.2.11 | Field access expressions | Yes | |
8.2.12 | Closure expressions | Yes | |
8.2.13 | Loop expressions | Yes | |
8.2.14 | Range expressions | Yes | |
8.2.15 | If and if let expressions | Yes | |
8.2.16 | Match expressions | Yes | |
8.2.17 | Return expressions | Yes | |
8.2.18 | Await expressions | No | See [Notes - Concurrency](#concurrency) |
9 | Patterns | Partial | [#707](https://github.com/model-checking/kani/issues/707) |
10.1.1 | Boolean type | Yes | |
10.1.2 | Numeric types | Yes | | See [Notes - Floats](#floating-point-operations)
10.1.3 | Textual types | Yes | |
10.1.4 | Never type | Yes | |
10.1.5 | Tuple types | Yes | |
10.1.6 | Array types | Yes | |
10.1.7 | Slice types | Yes | |
10.1.8 | Struct types | Yes | |
10.1.9 | Enumerated types | Yes | |
10.1.10 | Union types | Yes | |
10.1.11 | Function item types | Yes | |
10.1.12 | Closure types | Partial | See [Notes - Advanced features](#advanced-features) |
10.1.13 | Pointer types | Partial | See [Notes - Advanced features](#advanced-features) |
10.1.14 | Function pointer types | Partial | See [Notes - Advanced features](#advanced-features) |
10.1.15 | Trait object types | Partial | See [Notes - Advanced features](#advanced-features) |
10.1.16 | Impl trait type | Partial | See [Notes - Advanced features](#advanced-features) |
10.1.17 | Type parameters | Partial | See [Notes - Advanced features](#advanced-features) |
10.1.18 | Inferred type | Partial | See [Notes - Advanced features](#advanced-features) |
10.2 | Dynamically Sized Types | Partial | See [Notes - Advanced features](#advanced-features) |
10.3 | Type layout | Yes | |
10.4 | Interior mutability | Yes | |
10.5 | Subtyping and Variance | Yes | |
10.6 | Trait and lifetime bounds | Yes | |
10.7 | Type coercions | Partial | See [Notes - Advanced features](#advanced-features) |
10.8 | Destructors | Partial | |
10.9 | Lifetime elision | Yes | |
11 | Special types and traits | Partial | |
| | `Box<T>` | Yes | |
| | `Rc<T>` | Yes | |
| | `Arc<T>` | Yes | |
| | `Pin<T>` | Yes | |
| | `UnsafeCell<T>` | Partial | |
| | `PhantomData<T>` | Partial | |
| | Operator Traits | Partial | |
| | `Deref` and `DerefMut` | Yes | |
| | `Drop` | Partial | |
| | `Copy` | Yes | |
| | `Clone` | Yes | |
14 | Linkage | Yes | |
15.1 | Unsafe functions | Yes | |
15.2 | Unsafe blocks | Yes | |
15.3 | Behavior considered undefined | Partial | |
| | Data races | No | See [Notes - Concurrency](#concurrency) |
| | Dereferencing dangling raw pointers | Yes | |
| | Dereferencing unaligned raw pointers | No | |
| | Breaking pointer aliasing rules | No | |
| | Mutating immutable data | No | |
| | Invoking undefined behavior via compiler intrinsics | Partial | See [Notes - Intrinsics](#intrinsics) |
| | Executing code compiled with platform features that the current platform does not support | No | |
| | Producing an invalid value, even in private fields and locals | No | |

## Notes on partially or unsupported features

### Code generation for unsupported features

Kani aims to be an industrial verification tool. Most industrial crates may
include unsupported features in parts of their code that do not need to be
verified. In general, this should not prevent users using Kani to verify their code.

Because of that, the general rule is that Kani generates an `assert(false)`
statement followed by an `assume(false)` statement when compiling any
unsupported feature. `assert(false)` will cause verification to fail if the
statement is reachable during the verification stage, while `assume(false)` will
block any further exploration of the path. However, the analysis will not be
affected if the statement is not reachable from the code under verification, so
users can still verify components of their code that do not use unsupported
features.

In a few cases, Kani aborts execution if the analysis could be affected in
some way because of an unsupported feature (e.g., global ASM).

### Assembly

Kani does not support assembly code for now. We may add it in the future but at
present there are no plans to do so.

Check out the tracking issues for [inline assembly (`asm!`
macro)](https://github.com/model-checking/kani/issues/2) and [global assembly
(`asm_global!` macro)](https://github.com/model-checking/kani/issues/316) to know
more about the current status.

### Concurrency

Concurrent features are currently out of scope for Kani. In general, the
verification of concurrent programs continues to be an open research problem
where most tools that analyze concurrent code lack support for other features.
Because of this, Kani emits a warning whenever it encounters concurrent code and
compiles as if it was sequential code.

### Standard library functions

Kani [overrides](./overrides.md) a few common functions
(e.g., print macros) to provide a more verification friendly implementation.

### Advanced features

The semantics around some advanced features (traits, types, etc.) from Rust are
not formally defined which makes it harder to ensure that we can properly model
all their use cases.

We are aware of a lack of sanity checking the `Variant` type in projections
[#448](https://github.com/model-checking/kani/issues/448).
If you become aware of other issues concerning
these features, please [file a bug
report](https://github.com/model-checking/kani/issues/new?assignees=&labels=bug&template=bug_report.md).

### Panic strategies

Rust has two different strategies when a panic occurs:
 1. Stack unwinding (default): Walks back the stack cleaning up the data from
    each function it encounters.
 2. Abortion: Immediately ends the program without cleaning up.

Currently, Kani does not support stack unwinding. This has some implications
regarding memory safety since programs sometimes rely on the unwinding logic to
ensure there is no resource leak or persistent data inconsistency. Check out
[this issue](https://github.com/model-checking/kani/issues/692) for updates on
stack unwinding support.

### Uninitialized memory

Reading uninitialized memory is
[considered undefined behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html#behavior-considered-undefined) in Rust.
Kani has partial, experimental support for detecting access to uninitialized memory with the `-Z uninit-checks` option.
See [this issue](https://github.com/model-checking/kani/issues/3300) for more details.

### Destructors

At present, we are aware of some issues with destructors, in particular those
related to [advanced features](#advanced-features).

### Intrinsics

Please refer to [Intrinsics](rust-feature-support/intrinsics.md) for information
on the current support in Kani for Rust compiler intrinsics.

### Floating point operations

Kani supports floating point numbers, but some supported operations on floats are "over-approximated."
These are the trigonometric functions like `sin` and `cos` and the `sqrt` function as well.
This means the verifier can raise errors that cannot actually happen when the code is run normally.
For instance, ([#1342](https://github.com/model-checking/kani/issues/1342)) the `sin`/`cos` functions basically return a nondeterministic value between -1 and 1.
In other words, they largely ignore their input and give very conservative answers.
This range certainly includes the "real" value, so proof soundness is still preserved, but it means Kani could raise spurious errors that cannot actually happen.
This makes Kani unsuitable for verifying some kinds of properties (e.g. precision) about numerical algorithms.
Proofs that fail because of this problem can sometimes be repaired by introducing "stubs" for these functions that return a more acceptable approximation.
However, note that the actual behavior of these functions can vary by platform/os/architecture/compiler, so introducing an "overly precise" approximation may introduce unsoundness: actual system behavior may produce different values from the stub's approximation.
