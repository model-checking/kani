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
10.1.2 | Numeric types | Yes | |
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

At present, Kani is able to link in functions from the standard library but the
generated code will not contain them unless they are generic, intrinsics,
inlined or macros. Missing functions are treated in a similar way to [unsupported
features](#code-generation-for-unsupported-features).
This results in verification failures if a call to one of these missing functions
is reachable.

We've done some experiments to embed the standard library into the generated
code, but this causes verification times to increase significantly. As of now,
we've not been able to find a simple solution for [this
issue](https://github.com/model-checking/kani/issues/581), but we have some
ideas for future work in this direction. At present, Kani
[overrides](./overrides.md) a few common functions (e.g., print macros) as
a workaround.

### Advanced features

The semantics around some advanced features (traits, types, etc.) from Rust are
not formally defined which makes it harder to ensure that we can properly model
all their use cases.

In particular, there are some outstanding issues to note here:
 * Unimplemented `PointerCast::ClosureFnPointer` in
   [#274](https://github.com/model-checking/kani/issues/274) and `Variant` case
   in projections type in
   [#448](https://github.com/model-checking/kani/issues/448).
 * Unexpected fat pointer results in
   [#82](https://github.com/model-checking/kani/issues/82),
   [#277](https://github.com/model-checking/kani/issues/277),
   [#327](https://github.com/model-checking/kani/issues/327),
   [#378](https://github.com/model-checking/kani/issues/378) and
   [#676](https://github.com/model-checking/kani/issues/676).

We are particularly interested in bug reports concerning
these features, so please [file a bug
report](https://github.com/model-checking/kani/issues/new?assignees=&labels=bug&template=bug_report.md)
if you're aware of one.

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
At the moment, Kani cannot detect if memory is uninitialized, but in practice
this is mitigated by the fact that all memory is initialized with
nondeterministic values.
Therefore, any code that depends on uninitialized data will exhibit nondeterministic behavior.
See [this issue](https://github.com/model-checking/kani/issues/920) for more details.

### Destructors

At present, we are aware of some issues with destructors, in particular those
related to [advanced features](#advanced-features).

### Intrinsics

The table below tries to summarize the current support in Kani for Rust
intrinsics.

In general, code generation for unsupported intrinsics follows the rule
described in [Code generation for unsupported
features](#code-generation-for-unsupported-features).

Name | Support | Notes |
--- | --- | --- |
abort | Yes | |
add_with_overflow | Yes | |
arith_offset | No | |
assert_inhabited | Partial | [#751](https://github.com/model-checking/kani/issues/751) |
assert_uninit_valid | Yes | |
assert_zero_valid | Yes | |
assume | Yes | |
atomic_and | Partial | See [Atomics](#atomics) |
atomic_and_acq | Partial | See [Atomics](#atomics) |
atomic_and_acqrel | Partial | See [Atomics](#atomics) |
atomic_and_rel | Partial | See [Atomics](#atomics) |
atomic_and_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg | Partial | See [Atomics](#atomics) |
atomic_cxchg_acq | Partial | See [Atomics](#atomics) |
atomic_cxchg_acq_failrelaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_acqrel | Partial | See [Atomics](#atomics) |
atomic_cxchg_acqrel_failrelaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_failacq | Partial | See [Atomics](#atomics) |
atomic_cxchg_failrelaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_rel | Partial | See [Atomics](#atomics) |
atomic_cxchg_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acq | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acq_failrelaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acqrel | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acqrel_failrelaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_failacq | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_failrelaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_rel | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_relaxed | Partial | See [Atomics](#atomics) |
atomic_fence | Partial | See [Atomics](#atomics) |
atomic_fence_acq | Partial | See [Atomics](#atomics) |
atomic_fence_acqrel | Partial | See [Atomics](#atomics) |
atomic_fence_rel | Partial | See [Atomics](#atomics) |
atomic_load | Partial | See [Atomics](#atomics) |
atomic_load_acq | Partial | See [Atomics](#atomics) |
atomic_load_relaxed | Partial | See [Atomics](#atomics) |
atomic_load_unordered | Partial | See [Atomics](#atomics) |
atomic_max | No | See [Atomics](#atomics) |
atomic_max_acq | No | See [Atomics](#atomics) |
atomic_max_acqrel | No | See [Atomics](#atomics) |
atomic_max_rel | No | See [Atomics](#atomics) |
atomic_max_relaxed | No | See [Atomics](#atomics) |
atomic_min | No | See [Atomics](#atomics) |
atomic_min_acq | No | See [Atomics](#atomics) |
atomic_min_acqrel | No | See [Atomics](#atomics) |
atomic_min_rel | No | See [Atomics](#atomics) |
atomic_min_relaxed | No | See [Atomics](#atomics) |
atomic_nand | Partial | See [Atomics](#atomics) |
atomic_nand_acq | Partial | See [Atomics](#atomics) |
atomic_nand_acqrel | Partial | See [Atomics](#atomics) |
atomic_nand_rel | Partial | See [Atomics](#atomics) |
atomic_nand_relaxed | Partial | See [Atomics](#atomics) |
atomic_or | Partial | See [Atomics](#atomics) |
atomic_or_acq | Partial | See [Atomics](#atomics) |
atomic_or_acqrel | Partial | See [Atomics](#atomics) |
atomic_or_rel | Partial | See [Atomics](#atomics) |
atomic_or_relaxed | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_acq | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_acqrel | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_rel | Partial | See [Atomics](#atomics) |
atomic_store | Partial | See [Atomics](#atomics) |
atomic_store_rel | Partial | See [Atomics](#atomics) |
atomic_store_relaxed | Partial | See [Atomics](#atomics) |
atomic_store_unordered | Partial | See [Atomics](#atomics) |
atomic_umax | No | See [Atomics](#atomics) |
atomic_umax_acq | No | See [Atomics](#atomics) |
atomic_umax_acqrel | No | See [Atomics](#atomics) |
atomic_umax_rel | No | See [Atomics](#atomics) |
atomic_umax_relaxed | No | See [Atomics](#atomics) |
atomic_umin | No | See [Atomics](#atomics) |
atomic_umin_acq | No | See [Atomics](#atomics) |
atomic_umin_acqrel | No | See [Atomics](#atomics) |
atomic_umin_rel | No | See [Atomics](#atomics) |
atomic_umin_relaxed | No | See [Atomics](#atomics) |
atomic_xadd | Partial | See [Atomics](#atomics) |
atomic_xadd_acq | Partial | See [Atomics](#atomics) |
atomic_xadd_acqrel | Partial | See [Atomics](#atomics) |
atomic_xadd_rel | Partial | See [Atomics](#atomics) |
atomic_xadd_relaxed | Partial | See [Atomics](#atomics) |
atomic_xchg | Partial | See [Atomics](#atomics) |
atomic_xchg_acq | Partial | See [Atomics](#atomics) |
atomic_xchg_acqrel | Partial | See [Atomics](#atomics) |
atomic_xchg_rel | Partial | See [Atomics](#atomics) |
atomic_xchg_relaxed | Partial | See [Atomics](#atomics) |
atomic_xor | Partial | See [Atomics](#atomics) |
atomic_xor_acq | Partial | See [Atomics](#atomics) |
atomic_xor_acqrel | Partial | See [Atomics](#atomics) |
atomic_xor_rel | Partial | See [Atomics](#atomics) |
atomic_xor_relaxed | Partial | See [Atomics](#atomics) |
atomic_xsub | Partial | See [Atomics](#atomics) |
atomic_xsub_acq | Partial | See [Atomics](#atomics) |
atomic_xsub_acqrel | Partial | See [Atomics](#atomics) |
atomic_xsub_rel | Partial | See [Atomics](#atomics) |
atomic_xsub_relaxed | Partial | See [Atomics](#atomics) |
blackbox | Yes | |
bitreverse | Yes | |
breakpoint | Yes | |
bswap | Yes | |
caller_location | No | |
ceilf32 | No | |
ceilf64 | No | |
copy | No | |
copy_nonoverlapping | No | |
copysignf32 | No | |
copysignf64 | No | |
cosf32 | Yes | |
cosf64 | Yes | |
ctlz | Yes | |
ctlz_nonzero | Yes | |
ctpop | Yes | |
cttz | Yes | |
cttz_nonzero | Yes | |
discriminant_value | Yes | |
drop_in_place | No | |
exact_div | Yes | |
exp2f32 | No | |
exp2f64 | No | |
expf32 | No | |
expf64 | No | |
fabsf32 | Yes | |
fabsf64 | Yes | |
fadd_fast | Yes | |
fdiv_fast | Partial | [#809](https://github.com/model-checking/kani/issues/809) |
float_to_int_unchecked | No | |
floorf32 | No | |
floorf64 | No | |
fmaf32 | No | |
fmaf64 | No | |
fmul_fast | Partial | [#809](https://github.com/model-checking/kani/issues/809) |
forget | Yes | |
frem_fast | No | |
fsub_fast | Yes | |
likely | Yes | |
log10f32 | No | |
log10f64 | No | |
log2f32 | No | |
log2f64 | No | |
logf32 | No | |
logf64 | No | |
maxnumf32 | No | |
maxnumf64 | No | |
min_align_of | Yes | |
min_align_of_val | Yes | |
minnumf32 | No | |
minnumf64 | No | |
move_val_init | No | |
mul_with_overflow | Yes | |
nearbyintf32 | No | |
nearbyintf64 | No | |
needs_drop | Yes | |
nontemporal_store | No | |
offset | Partial | Missing undefined behavior checks |
powf32 | No | |
powf64 | No | |
powif32 | No | |
powif64 | No | |
pref_align_of | Yes | |
prefetch_read_data | No | |
prefetch_read_instruction | No | |
prefetch_write_data | No | |
prefetch_write_instruction | No | |
ptr_guaranteed_eq | Yes | |
ptr_guaranteed_ne | Yes | |
ptr_offset_from | Partial | Missing undefined behavior checks |
raw_eq | Partial | Cannot detect [uninitialized memory](#uninitialized-memory) |
rintf32 | No | |
rintf64 | No | |
rotate_left | Yes | |
rotate_right | Yes | |
roundf32 | No | |
roundf64 | No | |
rustc_peek | No | |
saturating_add | Yes | |
saturating_sub | Yes | |
sinf32 | Yes | |
sinf64 | Yes | |
size_of | Yes | |
size_of_val | Yes | |
sqrtf32 | No | |
sqrtf64 | No | |
sub_with_overflow | Yes | |
transmute | Yes | |
truncf32 | No | |
truncf64 | No | |
try | No | [#267](https://github.com/model-checking/kani/issues/267) |
type_id | Yes | |
type_name | Yes | |
unaligned_volatile_load | No | See [Notes - Concurrency](#concurrency) |
unaligned_volatile_store | No | See [Notes - Concurrency](#concurrency) |
unchecked_add | Yes | |
unchecked_div | Yes | |
unchecked_mul | Yes | |
unchecked_rem | Yes | |
unchecked_shl | Yes | |
unchecked_shr | Yes | |
unchecked_sub | Yes | |
unlikely | Yes | |
unreachable | Yes | |
variant_count | No | |
volatile_copy_memory | No | See [Notes - Concurrency](#concurrency) |
volatile_copy_nonoverlapping_memory | No | See [Notes - Concurrency](#concurrency) |
volatile_load | No | See [Notes - Concurrency](#concurrency) |
volatile_set_memory | No | See [Notes - Concurrency](#concurrency) |
volatile_store | Partial | See [Notes - Concurrency](#concurrency) |
wrapping_add | Yes | |
wrapping_mul | Yes | |
wrapping_sub | Yes | |
write_bytes | Yes | |

#### Atomics

All atomic intrinsics are compiled as an atomic block where the operation is
performed. But as noted in [Notes - Concurrency](#concurrency), Kani support for
concurrent verification is limited and not used by default. Verification on code
containing atomic intrinsics should not be trusted given that Kani assumes the
code to be sequential.

#### SIMD instructions

While Kani is capable of generating code for SIMD instructions, unfortunately, it
does not provide support for the verification of some operations like vector
comparison (e.g., `simd_eq`).
