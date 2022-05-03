# Intrinsics

The table below tries to summarize the current support in Kani for Rust
compiler intrinsics. We define the level of support similar to how we
indicate [Rust feature support](../rust-feature-support.md):
 * **Yes**: The intrinsic is fully supported. We are not aware of any issue with it.
 * **Partial**: The intrinsic is at least partially supported. We are aware of some issue with
 with it.
 * **No**: The intrinsic is not supported.

In general, code generation for unsupported intrinsics follows the rule
described in [Rust feature support - Code generation for unsupported
features](../rust-feature-support.md#code-generation-for-unsupported-features).

Name | Support | Notes |
--- | --- | --- |
abort | Yes | |
add_with_overflow | Yes | |
arith_offset | No | |
assert_inhabited | | |
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
offset | Yes | |
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
