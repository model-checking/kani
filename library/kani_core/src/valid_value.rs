// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// State for `-Z valid-value-checks`: which bytes may have been written without a type check.
// For full documentation, see the usage of `kani_core::kani_valid_value!(std);` in
// library/kani_core/src/lib.rs.

// Definitions in this module are not meant to be visible to the end user, only the compiler.
#![allow(dead_code)]

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! kani_valid_value {
    ($core:path) => {
        /// The byte the checks track, chosen nondeterministically when the harness starts, and
        /// whether its last write was untyped: a write of raw bytes (`write_bytes`, a copy) or a
        /// store of a type without validity requirements through a pointer. A typed read that
        /// covers the tracked byte checks the value it reads only if that byte was written
        /// untyped, so that writing bytes that are invalid for a type is only reported when they
        /// are read as that type, as the language specifies.
        struct UntypedWriteState {
            initialized: bool,
            tracked_object_id: usize,
            tracked_offset: usize,
            untyped: bool,
        }

        // Identified by its `fn_marker`, as `MEM_INIT_STATE` is.
        #[kanitool::fn_marker = "KaniUntypedWriteState"]
        static mut UNTYPED_WRITE_STATE: UntypedWriteState = UntypedWriteState {
            initialized: false,
            tracked_object_id: 0,
            tracked_offset: 0,
            untyped: false,
        };

        /// Choose the tracked byte, where the harness starts. A harness is an ordinary function
        /// that another harness can call, so only the first call chooses it: a later one would
        /// forget the untyped writes made before it.
        #[kanitool::fn_marker = "InitializeUntypedWriteStateModel"]
        fn initialize_untyped_write_state() {
            unsafe {
                if !UNTYPED_WRITE_STATE.initialized {
                    UNTYPED_WRITE_STATE.initialized = true;
                    UNTYPED_WRITE_STATE.tracked_object_id = super::any();
                    UNTYPED_WRITE_STATE.tracked_offset = super::any();
                    UNTYPED_WRITE_STATE.untyped = false;
                }
            }
        }

        /// Record whether the `count` values of `elem_size` bytes at `ptr` were written untyped.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "SetUntypedWriteModel"]
        fn set_untyped_write<T: ?Sized>(
            ptr: *const T,
            count: usize,
            elem_size: usize,
            untyped: bool,
        ) {
            let (ptr, _) = ptr.to_raw_parts();
            let ptr = ptr as *const u8;
            let obj = super::mem::pointer_object(ptr);
            let offset = super::mem::pointer_offset(ptr);
            unsafe {
                if UNTYPED_WRITE_STATE.tracked_object_id == obj
                    && UNTYPED_WRITE_STATE.tracked_offset >= offset
                    && UNTYPED_WRITE_STATE.tracked_offset - offset < count.saturating_mul(elem_size)
                {
                    UNTYPED_WRITE_STATE.untyped = untyped;
                }
            }
        }

        /// Whether `vtable` is the address of a vtable that the program can create for the trait
        /// object type `T` (or the trait object at the tail of `T`): one Kani generated for the
        /// trait, or for a trait whose vtables the program casts to it. The compiler generates
        /// the body once all the vtables are known.
        #[kanitool::fn_marker = "ValidVtableHook"]
        #[inline(never)]
        fn valid_vtable<T: ?Sized>(_vtable: *const u8) -> bool {
            super::kani_intrinsic()
        }

        /// Whether the trait object at the tail of the `T` at `ptr` is a valid value of its
        /// dynamic type, which the vtable in the metadata of `ptr` determines. The trait object
        /// is at `tail_offset` from `ptr`, rounded up to the alignment of its dynamic type. It is
        /// invalid if the vtable is not one `valid_vtable` accepts.
        #[kanitool::fn_marker = "ValidDynValueHook"]
        #[inline(never)]
        fn valid_dyn_value<T: ?Sized>(_ptr: *const T, _tail_offset: usize) -> bool {
            super::kani_intrinsic()
        }

        /// Whether one of the `size` bytes at `ptr` was written untyped.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "IsUntypedWriteModel"]
        fn is_untyped_write<T: ?Sized>(ptr: *const T, size: usize) -> bool {
            let (ptr, _) = ptr.to_raw_parts();
            let ptr = ptr as *const u8;
            let obj = super::mem::pointer_object(ptr);
            let offset = super::mem::pointer_offset(ptr);
            unsafe {
                UNTYPED_WRITE_STATE.untyped
                    && UNTYPED_WRITE_STATE.tracked_object_id == obj
                    && UNTYPED_WRITE_STATE.tracked_offset >= offset
                    && UNTYPED_WRITE_STATE.tracked_offset - offset < size
            }
        }
    };
}
