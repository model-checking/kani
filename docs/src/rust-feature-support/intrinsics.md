# Intrinsics

The tables below try to summarize the current support in Kani for Rust intrinsics.
We define the level of support similar to how we indicate [Rust feature support](../rust-feature-support.md):
 * **Yes**: The intrinsic is fully supported. We are not aware of any issue with it.
 * **Partial**: The intrinsic is at least partially supported. We are aware of some issue with
 with it.
 * **No**: The intrinsic is not supported.

In general, code generation for unsupported intrinsics follows the rule
described in [Rust feature support - Code generation for unsupported
features](../rust-feature-support.md#code-generation-for-unsupported-features).

Any intrinsic not appearing in the tables below is considered not supported.
Please [open a feature request](https://github.com/model-checking/kani/issues/new?assignees=&labels=%5BC%5D+Feature+%2F+Enhancement&template=feature_request.md&title=)
if your code depends on an unsupported intrinsic.

### Compiler intrinsics

Name | Support | Notes |
--- | --- | --- |
abort | Yes | |
add_with_overflow | Yes | |
arith_offset | Yes | |
assert_inhabited | Yes | |
assert_uninit_valid | Yes | |
assert_zero_valid | Yes | |
assume | Yes | |
atomic_and_seqcst | Partial | See [Atomics](#atomics) |
atomic_and_acquire | Partial | See [Atomics](#atomics) |
atomic_and_acqrel | Partial | See [Atomics](#atomics) |
atomic_and_release | Partial | See [Atomics](#atomics) |
atomic_and_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_acqrel_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchg_acqrel_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_acqrel_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchg_acquire_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchg_acquire_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_acquire_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchg_relaxed_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchg_relaxed_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_relaxed_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchg_release_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchg_release_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_release_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchg_seqcst_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchg_seqcst_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchg_seqcst_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acqrel_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acqrel_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acqrel_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acquire_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acquire_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_acquire_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_relaxed_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_relaxed_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_relaxed_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_release_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_release_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_release_seqcst | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_seqcst_acquire | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_seqcst_relaxed | Partial | See [Atomics](#atomics) |
atomic_cxchgweak_seqcst_seqcst | Partial | See [Atomics](#atomics) |
atomic_fence_seqcst | Partial | See [Atomics](#atomics) |
atomic_fence_acquire | Partial | See [Atomics](#atomics) |
atomic_fence_acqrel | Partial | See [Atomics](#atomics) |
atomic_fence_release | Partial | See [Atomics](#atomics) |
atomic_load_seqcst | Partial | See [Atomics](#atomics) |
atomic_load_acquire | Partial | See [Atomics](#atomics) |
atomic_load_relaxed | Partial | See [Atomics](#atomics) |
atomic_load_unordered | Partial | See [Atomics](#atomics) |
atomic_max_seqcst | Partial | See [Atomics](#atomics) |
atomic_max_acquire | Partial | See [Atomics](#atomics) |
atomic_max_acqrel | Partial | See [Atomics](#atomics) |
atomic_max_release | Partial | See [Atomics](#atomics) |
atomic_max_relaxed | Partial | See [Atomics](#atomics) |
atomic_min_seqcst | Partial | See [Atomics](#atomics) |
atomic_min_acquire | Partial | See [Atomics](#atomics) |
atomic_min_acqrel | Partial | See [Atomics](#atomics) |
atomic_min_release | Partial | See [Atomics](#atomics) |
atomic_min_relaxed | Partial | See [Atomics](#atomics) |
atomic_nand_seqcst | Partial | See [Atomics](#atomics) |
atomic_nand_acquire | Partial | See [Atomics](#atomics) |
atomic_nand_acqrel | Partial | See [Atomics](#atomics) |
atomic_nand_release | Partial | See [Atomics](#atomics) |
atomic_nand_relaxed | Partial | See [Atomics](#atomics) |
atomic_or_seqcst | Partial | See [Atomics](#atomics) |
atomic_or_acquire | Partial | See [Atomics](#atomics) |
atomic_or_acqrel | Partial | See [Atomics](#atomics) |
atomic_or_release | Partial | See [Atomics](#atomics) |
atomic_or_relaxed | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_seqcst | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_acquire | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_acqrel | Partial | See [Atomics](#atomics) |
atomic_singlethreadfence_release | Partial | See [Atomics](#atomics) |
atomic_store_seqcst | Partial | See [Atomics](#atomics) |
atomic_store_release | Partial | See [Atomics](#atomics) |
atomic_store_relaxed | Partial | See [Atomics](#atomics) |
atomic_store_unordered | Partial | See [Atomics](#atomics) |
atomic_umax_seqcst | Partial | See [Atomics](#atomics) |
atomic_umax_acquire | Partial | See [Atomics](#atomics) |
atomic_umax_acqrel | Partial | See [Atomics](#atomics) |
atomic_umax_release | Partial | See [Atomics](#atomics) |
atomic_umax_relaxed | Partial | See [Atomics](#atomics) |
atomic_umin_seqcst | Partial | See [Atomics](#atomics) |
atomic_umin_acquire | Partial | See [Atomics](#atomics) |
atomic_umin_acqrel | Partial | See [Atomics](#atomics) |
atomic_umin_release | Partial | See [Atomics](#atomics) |
atomic_umin_relaxed | Partial | See [Atomics](#atomics) |
atomic_xadd_seqcst | Partial | See [Atomics](#atomics) |
atomic_xadd_acquire | Partial | See [Atomics](#atomics) |
atomic_xadd_acqrel | Partial | See [Atomics](#atomics) |
atomic_xadd_release | Partial | See [Atomics](#atomics) |
atomic_xadd_relaxed | Partial | See [Atomics](#atomics) |
atomic_xchg_seqcst | Partial | See [Atomics](#atomics) |
atomic_xchg_acquire | Partial | See [Atomics](#atomics) |
atomic_xchg_acqrel | Partial | See [Atomics](#atomics) |
atomic_xchg_release | Partial | See [Atomics](#atomics) |
atomic_xchg_relaxed | Partial | See [Atomics](#atomics) |
atomic_xor_seqcst | Partial | See [Atomics](#atomics) |
atomic_xor_acquire | Partial | See [Atomics](#atomics) |
atomic_xor_acqrel | Partial | See [Atomics](#atomics) |
atomic_xor_release | Partial | See [Atomics](#atomics) |
atomic_xor_relaxed | Partial | See [Atomics](#atomics) |
atomic_xsub_seqcst | Partial | See [Atomics](#atomics) |
atomic_xsub_acquire | Partial | See [Atomics](#atomics) |
atomic_xsub_acqrel | Partial | See [Atomics](#atomics) |
atomic_xsub_release | Partial | See [Atomics](#atomics) |
atomic_xsub_relaxed | Partial | See [Atomics](#atomics) |
blackbox | Yes | |
bitreverse | Yes | |
breakpoint | Yes | |
bswap | Yes | |
caller_location | No | |
ceilf32 | Yes | |
ceilf64 | Yes | |
copy | Yes | |
copy_nonoverlapping | Yes | |
copysignf32 | Yes | |
copysignf64 | Yes | |
cosf32 | Partial | Results are overapproximated; [this test](https://github.com/model-checking/kani/blob/main/tests/kani/Intrinsics/Math/Trigonometry/cosf32.rs) explains how |
cosf64 | Partial | Results are overapproximated; [this test](https://github.com/model-checking/kani/blob/main/tests/kani/Intrinsics/Math/Trigonometry/cosf64.rs) explains how |
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
floorf32 | Yes | |
floorf64 | Yes | |
fmaf32 | Partial | Results are overapproximated |
fmaf64 | Partial | Results are overapproximated |
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
maxnumf32 | Yes | |
maxnumf64 | Yes | |
min_align_of | Yes | |
min_align_of_val | Yes | |
minnumf32 | Yes | |
minnumf64 | Yes | |
move_val_init | No | |
mul_with_overflow | Yes | |
nearbyintf32 | Yes | |
nearbyintf64 | Yes | |
needs_drop | Yes | |
nontemporal_store | No | |
offset | Partial | Doesn't check [all UB conditions](https://doc.rust-lang.org/std/primitive.pointer.html#safety-2) |
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
ptr_offset_from | Partial | Doesn't check [all UB conditions](https://doc.rust-lang.org/std/primitive.pointer.html#safety-4) |
raw_eq | Partial | Cannot detect [uninitialized memory](#uninitialized-memory) |
rintf32 | Yes | |
rintf64 | Yes | |
rotate_left | Yes | |
rotate_right | Yes | |
roundf32 | Yes | |
roundf64 | Yes | |
rustc_peek | No | |
saturating_add | Yes | |
saturating_sub | Yes | |
sinf32 | Partial | Results are overapproximated; [this test](https://github.com/model-checking/kani/blob/main/tests/kani/Intrinsics/Math/Trigonometry/sinf32.rs) explains how |
sinf64 | Partial | Results are overapproximated; [this test](https://github.com/model-checking/kani/blob/main/tests/kani/Intrinsics/Math/Trigonometry/sinf64.rs) explains how |
size_of | Yes | |
size_of_val | Yes | |
sqrtf32 | No | |
sqrtf64 | No | |
sub_with_overflow | Yes | |
transmute | Partial | Doesn't check [all UB conditions](https://doc.rust-lang.org/nomicon/transmutes.html) |
truncf32 | Yes | |
truncf64 | Yes | |
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
volatile_load | Partial | See [Notes - Concurrency](#concurrency) |
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

### Platform intrinsics

Intrinsics from [the `platform_intrinsics` feature](https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html#operations).

Name | Support | Notes |
--- | --- | --- |
`simd_add` | Yes | |
`simd_and`  | Yes | |
`simd_div`  | Yes | |
`simd_eq`  | Yes | |
`simd_extract`  | Yes | |
`simd_ge`  | Yes | |
`simd_gt`  | Yes | |
`simd_insert`  | Yes | |
`simd_le`  | Yes | |
`simd_lt`  | Yes | |
`simd_mul`  | Yes | |
`simd_ne`  | Yes | |
`simd_or`  | Yes | |
`simd_rem`  | Yes | Doesn't check for floating point overflow [#2669](https://github.com/model-checking/kani/issues/2669) |
`simd_shl`  | Yes | |
`simd_shr`  | Yes | |
`simd_shuffle*`  | Yes | |
`simd_sub`  | Yes | |
`simd_xor`  | Yes | |
