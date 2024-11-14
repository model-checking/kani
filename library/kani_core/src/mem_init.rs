// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides instrumentation for tracking memory initialization of raw pointers.
//!
//! Currently, memory initialization is tracked on per-byte basis, so each byte of memory pointed to
//! by raw pointers could be either initialized or uninitialized. Padding bytes are always
//! considered uninitialized when read as data bytes. Each type has a type layout to specify which
//! bytes are considered to be data and which -- padding. This is determined at compile time and
//! statically injected into the program (see `Layout`).
//!
//! Compiler automatically inserts calls to `is_xxx_initialized` and `set_xxx_initialized` at
//! appropriate locations to get or set the initialization status of the memory pointed to.
//!
//! Note that for each harness, tracked object and tracked offset are chosen non-deterministically,
//! so calls to `is_xxx_initialized` should be only used in assertion contexts.

// Definitions in this module are not meant to be visible to the end user, only the compiler.
#![allow(dead_code)]

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! kani_mem_init {
    ($core:path) => {
        /// Global object for tracking memory initialization state.
        #[kanitool::fn_marker = "MemoryInitializationStateModel"]
        static mut MEM_INIT_STATE: MemoryInitializationState = MemoryInitializationState::new();

        /// Global object for tracking union initialization state across function boundaries.
        static mut ARGUMENT_BUFFER: Option<ArgumentBuffer> = None;

        /// Bytewise mask, representing which bytes of a type are data and which are padding.
        /// For example, for a type like this:
        /// ```
        /// #[repr(C)]
        /// struct Foo {
        ///     a: u16,
        ///     b: u8,
        /// }
        /// ```
        /// the layout would be [true, true, true, false];
        type Layout<const LAYOUT_SIZE: usize> = [bool; LAYOUT_SIZE];

        /// Currently tracked non-deterministically chosen memory initialization state.
        struct MemoryInitializationState {
            pub tracked_object_id: usize,
            pub tracked_offset: usize,
            pub value: bool,
        }

        impl MemoryInitializationState {
            /// This is a dummy initialization function -- the values will be eventually overwritten by a
            /// call to `initialize_memory_initialization_state`.
            pub const fn new() -> Self {
                Self { tracked_object_id: 0, tracked_offset: 0, value: false }
            }

            /// Return currently tracked memory initialization state if `ptr` points to the currently
            /// tracked object and the tracked offset lies within `LAYOUT_SIZE` bytes of `ptr`. Return
            /// `true` otherwise.
            ///
            /// Such definition is necessary since both tracked object and tracked offset are chosen
            /// non-deterministically.
            #[kanitool::disable_checks(pointer)]
            pub fn get<const LAYOUT_SIZE: usize>(
                &mut self,
                ptr: *const u8,
                layout: Layout<LAYOUT_SIZE>,
            ) -> bool {
                let obj = super::mem::pointer_object(ptr);
                let offset = super::mem::pointer_offset(ptr);
                if self.tracked_object_id == obj
                    && self.tracked_offset >= offset
                    && self.tracked_offset < offset + LAYOUT_SIZE
                {
                    !layout[self.tracked_offset - offset] || self.value
                } else {
                    true
                }
            }

            /// Set currently tracked memory initialization state if `ptr` points to the currently tracked
            /// object and the tracked offset lies within `LAYOUT_SIZE` bytes of `ptr`. Do nothing
            /// otherwise.
            ///
            /// Such definition is necessary since both tracked object and tracked offset are chosen
            /// non-deterministically.
            #[kanitool::disable_checks(pointer)]
            pub fn set<const LAYOUT_SIZE: usize>(
                &mut self,
                ptr: *const u8,
                layout: Layout<LAYOUT_SIZE>,
                value: bool,
            ) {
                let obj = super::mem::pointer_object(ptr);
                let offset = super::mem::pointer_offset(ptr);
                if self.tracked_object_id == obj
                    && self.tracked_offset >= offset
                    && self.tracked_offset < offset + LAYOUT_SIZE
                {
                    self.value = layout[self.tracked_offset - offset] && value;
                }
            }

            /// Copy memory initialization state by non-deterministically switching the tracked object and
            /// adjusting the tracked offset.
            #[kanitool::disable_checks(pointer)]
            pub fn copy<const LAYOUT_SIZE: usize>(
                &mut self,
                from_ptr: *const u8,
                to_ptr: *const u8,
                num_elts: usize,
            ) {
                let from_obj = super::mem::pointer_object(from_ptr);
                let from_offset = super::mem::pointer_offset(from_ptr);

                let to_obj = super::mem::pointer_object(to_ptr);
                let to_offset = super::mem::pointer_offset(to_ptr);

                if self.tracked_object_id == from_obj
                    && self.tracked_offset >= from_offset
                    && self.tracked_offset < from_offset + num_elts * LAYOUT_SIZE
                {
                    let should_reset: bool = super::any();
                    if should_reset {
                        self.tracked_object_id = to_obj;
                        self.tracked_offset += to_offset - from_offset;
                        // Note that this preserves the value.
                    }
                } else {
                    self.bless::<LAYOUT_SIZE>(to_ptr, 1);
                }
            }

            /// Set currently tracked memory initialization state to `true` if `ptr` points to the
            /// currently tracked object and the tracked offset lies within `LAYOUT_SIZE * num_elts`
            /// bytes of `ptr`.
            #[kanitool::disable_checks(pointer)]
            pub fn bless<const LAYOUT_SIZE: usize>(&mut self, ptr: *const u8, num_elts: usize) {
                let obj = super::mem::pointer_object(ptr);
                let offset = super::mem::pointer_offset(ptr);

                if self.tracked_object_id == obj
                    && self.tracked_offset >= offset
                    && self.tracked_offset < offset + num_elts * LAYOUT_SIZE
                {
                    self.value = true;
                }
            }

            /// Return currently tracked memory initialization state if `ptr` points to the currently
            /// tracked object and the tracked offset lies within `LAYOUT_SIZE * num_elts` bytes of `ptr`.
            /// Return `true` otherwise.
            ///
            /// Such definition is necessary since both tracked object and tracked offset are chosen
            /// non-deterministically.
            #[kanitool::disable_checks(pointer)]
            pub fn get_slice<const LAYOUT_SIZE: usize>(
                &mut self,
                ptr: *const u8,
                layout: Layout<LAYOUT_SIZE>,
                num_elts: usize,
            ) -> bool {
                let obj = super::mem::pointer_object(ptr);
                let offset = super::mem::pointer_offset(ptr);
                if self.tracked_object_id == obj
                    && self.tracked_offset >= offset
                    && self.tracked_offset < offset + num_elts * LAYOUT_SIZE
                {
                    !layout[(self.tracked_offset - offset) % LAYOUT_SIZE] || self.value
                } else {
                    true
                }
            }

            /// Set currently tracked memory initialization state if `ptr` points to the currently tracked
            /// object and the tracked offset lies within `LAYOUT_SIZE * num_elts` bytes of `ptr`. Do
            /// nothing otherwise.
            ///
            /// Such definition is necessary since both tracked object and tracked offset are chosen
            /// non-deterministically.
            #[kanitool::disable_checks(pointer)]
            pub fn set_slice<const LAYOUT_SIZE: usize>(
                &mut self,
                ptr: *const u8,
                layout: Layout<LAYOUT_SIZE>,
                num_elts: usize,
                value: bool,
            ) {
                let obj = super::mem::pointer_object(ptr);
                let offset = super::mem::pointer_offset(ptr);
                if self.tracked_object_id == obj
                    && self.tracked_offset >= offset
                    && self.tracked_offset < offset + num_elts * LAYOUT_SIZE
                {
                    self.value = layout[(self.tracked_offset - offset) % LAYOUT_SIZE] && value;
                }
            }
        }

        /// Set tracked object and tracked offset to a non-deterministic value.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "InitializeMemoryInitializationStateModel"]
        fn initialize_memory_initialization_state() {
            unsafe {
                MEM_INIT_STATE.tracked_object_id = super::any();
                MEM_INIT_STATE.tracked_offset = super::any();
                MEM_INIT_STATE.value = false;
            }
        }

        /// Get initialization state of `num_elts` items laid out according to the `layout` starting at address `ptr`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "IsPtrInitializedModel"]
        fn is_ptr_initialized<const LAYOUT_SIZE: usize, T>(
            ptr: *const T,
            layout: Layout<LAYOUT_SIZE>,
        ) -> bool {
            if LAYOUT_SIZE == 0 {
                return true;
            }
            let (ptr, _) = ptr.to_raw_parts();
            unsafe { MEM_INIT_STATE.get(ptr as *const u8, layout) }
        }

        /// Set initialization state to `value` for `num_elts` items laid out according to the `layout` starting at address `ptr`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "SetPtrInitializedModel"]
        fn set_ptr_initialized<const LAYOUT_SIZE: usize, T>(
            ptr: *const T,
            layout: Layout<LAYOUT_SIZE>,
            value: bool,
        ) {
            if LAYOUT_SIZE == 0 {
                return;
            }
            let (ptr, _) = ptr.to_raw_parts();
            unsafe {
                MEM_INIT_STATE.set(ptr as *const u8, layout, value);
            }
        }

        /// Get initialization state of `num_elts` items laid out according to the `layout` starting at address `ptr`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "IsSliceChunkPtrInitializedModel"]
        fn is_slice_chunk_ptr_initialized<const LAYOUT_SIZE: usize, T>(
            ptr: *const T,
            layout: Layout<LAYOUT_SIZE>,
            num_elts: usize,
        ) -> bool {
            if LAYOUT_SIZE == 0 {
                return true;
            }
            let (ptr, _) = ptr.to_raw_parts();
            unsafe { MEM_INIT_STATE.get_slice(ptr as *const u8, layout, num_elts) }
        }

        /// Set initialization state to `value` for `num_elts` items laid out according to the `layout` starting at address `ptr`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "SetSliceChunkPtrInitializedModel"]
        fn set_slice_chunk_ptr_initialized<const LAYOUT_SIZE: usize, T>(
            ptr: *const T,
            layout: Layout<LAYOUT_SIZE>,
            num_elts: usize,
            value: bool,
        ) {
            if LAYOUT_SIZE == 0 {
                return;
            }
            let (ptr, _) = ptr.to_raw_parts();
            unsafe {
                MEM_INIT_STATE.set_slice(ptr as *const u8, layout, num_elts, value);
            }
        }

        /// Get initialization state of the slice, items of which are laid out according to the `layout` starting at address `ptr`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "IsSlicePtrInitializedModel"]
        fn is_slice_ptr_initialized<const LAYOUT_SIZE: usize, T>(
            ptr: *const [T],
            layout: Layout<LAYOUT_SIZE>,
        ) -> bool {
            if LAYOUT_SIZE == 0 {
                return true;
            }
            let (ptr, num_elts) = ptr.to_raw_parts();
            unsafe { MEM_INIT_STATE.get_slice(ptr as *const u8, layout, num_elts) }
        }

        /// Set initialization state of the slice, items of which are laid out according to the `layout` starting at address `ptr` to `value`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "SetSlicePtrInitializedModel"]
        fn set_slice_ptr_initialized<const LAYOUT_SIZE: usize, T>(
            ptr: *const [T],
            layout: Layout<LAYOUT_SIZE>,
            value: bool,
        ) {
            if LAYOUT_SIZE == 0 {
                return;
            }
            let (ptr, num_elts) = ptr.to_raw_parts();
            unsafe {
                MEM_INIT_STATE.set_slice(ptr as *const u8, layout, num_elts, value);
            }
        }

        /// Get initialization state of the string slice, items of which are laid out according to the `layout` starting at address `ptr`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "IsStrPtrInitializedModel"]
        fn is_str_ptr_initialized<const LAYOUT_SIZE: usize>(
            ptr: *const str,
            layout: Layout<LAYOUT_SIZE>,
        ) -> bool {
            if LAYOUT_SIZE == 0 {
                return true;
            }
            let (ptr, num_elts) = ptr.to_raw_parts();
            unsafe { MEM_INIT_STATE.get_slice(ptr as *const u8, layout, num_elts) }
        }

        /// Set initialization state of the string slice, items of which are laid out according to the `layout` starting at address `ptr` to `value`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "SetStrPtrInitializedModel"]
        fn set_str_ptr_initialized<const LAYOUT_SIZE: usize>(
            ptr: *const str,
            layout: Layout<LAYOUT_SIZE>,
            value: bool,
        ) {
            if LAYOUT_SIZE == 0 {
                return;
            }
            let (ptr, num_elts) = ptr.to_raw_parts();
            unsafe {
                MEM_INIT_STATE.set_slice(ptr as *const u8, layout, num_elts, value);
            }
        }

        /// Copy initialization state of `size_of::<T> * num_elts` bytes from one pointer to the other. Note
        /// that in this case `LAYOUT_SIZE == size_of::<T>`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "CopyInitStateModel"]
        fn copy_init_state<const LAYOUT_SIZE: usize, T>(
            from: *const T,
            to: *const T,
            num_elts: usize,
        ) {
            if LAYOUT_SIZE == 0 {
                return;
            }
            let (from_ptr, _) = from.to_raw_parts();
            let (to_ptr, _) = to.to_raw_parts();
            unsafe {
                MEM_INIT_STATE.copy::<LAYOUT_SIZE>(
                    from_ptr as *const u8,
                    to_ptr as *const u8,
                    num_elts,
                );
            }
        }

        /// Copy initialization state of `size_of::<T>` bytes from one pointer to the other. Note that in
        /// this case `LAYOUT_SIZE == size_of::<T>`.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "CopyInitStateSingleModel"]
        fn copy_init_state_single<const LAYOUT_SIZE: usize, T>(from: *const T, to: *const T) {
            copy_init_state::<LAYOUT_SIZE, T>(from, to, 1);
        }

        /// Information about currently tracked argument, used for passing union initialization
        /// state across function boundaries. This struct is written to by the caller and read from
        /// by the callee.
        #[derive(Clone, Copy)]
        struct ArgumentBuffer {
            selected_argument: usize,
            saved_address: *const (),
            layout_size: usize,
        }

        /// Non-deterministically store information about currently tracked argument in the argument
        /// buffer.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "StoreArgumentModel"]
        fn store_argument<const LAYOUT_SIZE: usize, T>(from: *const T, selected_argument: usize) {
            let (from_ptr, _) = from.to_raw_parts();
            let should_store: bool = super::any();
            if should_store {
                unsafe {
                    ARGUMENT_BUFFER = Some(ArgumentBuffer {
                        selected_argument,
                        saved_address: from_ptr,
                        layout_size: LAYOUT_SIZE,
                    })
                }
            }
        }

        /// Load information from the argument buffer (if the argument position matches) via copying
        /// the memory initialization information from an address in the caller to an address in the
        /// callee. Otherwise, mark that the argument as initialized, as it will be checked by
        /// another non-deterministic branch. Reset the argument buffer after loading from it.
        #[kanitool::disable_checks(pointer)]
        #[kanitool::fn_marker = "LoadArgumentModel"]
        fn load_argument<const LAYOUT_SIZE: usize, T>(to: *const T, selected_argument: usize) {
            let (to_ptr, _) = to.to_raw_parts();
            if let Some(buffer) = unsafe { ARGUMENT_BUFFER } {
                if buffer.selected_argument == selected_argument {
                    assert!(buffer.layout_size == LAYOUT_SIZE);
                    copy_init_state_single::<LAYOUT_SIZE, ()>(buffer.saved_address, to_ptr);
                    unsafe {
                        ARGUMENT_BUFFER = None;
                    }
                    return;
                }
            }
            unsafe {
                MEM_INIT_STATE.bless::<LAYOUT_SIZE>(to_ptr as *const u8, 1);
            }
        }
    };
}
