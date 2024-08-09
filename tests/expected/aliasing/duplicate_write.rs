// kani-flags: -Zghost-state -Zaliasing
#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(vec_into_raw_parts)]
// Copyright Jacob Salzberg
// SPDX-License-Identifier: Apache-2.0

// Basic test from the stacked borrows paper
#![allow(non_snake_case)]
#![feature(const_trait_impl)]
#![cfg_attr(not(kani), feature(register_tool))]
#![cfg_attr(not(kani), register_tool(kani))]

#[derive(Copy, Clone)]
#[rustc_diagnostic_item = "KaniAliasingChecked"]
struct AliasingChecked { amount: usize }

const STACK_DEPTH: usize = 15;
type PointerTag = u8;

extern crate kani;
use kani::shadow::ShadowMem;

#[inline(never)]
fn get_checked() -> AliasingChecked {
    static mut CHECKED: AliasingChecked = AliasingChecked { amount: 0 };
    unsafe { CHECKED.amount = CHECKED.amount.wrapping_add(1);  CHECKED }
}

#[cfg(any(kani))]
fn assume(b: bool) {
    let checked = get_checked();
    let _ = checked;
    kani::assume(b);
}


#[cfg(not(kani))]
fn assume(b: bool) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    assert!(b);
}

/// The stacked borrows state.
pub mod sstate {
    use super::*;
    /// Associate every pointer object with a tag
    static mut TAGS: ShadowMem<PointerTag> = ShadowMem::new(0);
    /// Next pointer id: the next pointer id in sequence
    static mut NEXT_TAG: PointerTag = 0;

    #[non_exhaustive]
    struct Access;
    #[allow(unused)]
    impl Access {
        pub(self) const READ: bool = false;
        pub(self) const WRITE: bool = true;
    }

    #[non_exhaustive]
    struct Permission;
    impl Permission {
        pub(self) const UNIQUE:   u8 = 0;
        pub(self) const SHAREDRW: u8 = 1;
        pub(self) const SHAREDRO: u8 = 2;
        pub(self) const DISABLED: u8 = 3;

        pub(self) fn grants(access: bool, tag: u8) -> bool {
            let checked: AliasingChecked = get_checked();
            let _ = checked;
            tag != Self::DISABLED && (access != Access::WRITE || tag != Self::SHAREDRO)
        }
    }

    /// Associate every pointer object with a permission
    static mut PERMS: ShadowMem<u8> = ShadowMem::new(0);

    pub(super) mod monitors {
        static mut STATE: bool = false;
        static mut OBJECT: usize = 0;
        static mut OFFSET: usize = 0;
        static mut STACK_TAGS: [u8; STACK_DEPTH] = [0; STACK_DEPTH];
        static mut STACK_PERMS: [u8; STACK_DEPTH] = [0; STACK_DEPTH];
        static mut STACK_TOP: usize = 0;

        #[non_exhaustive]
        struct MonitorState;
        impl MonitorState {
            pub(self) const UNINIT: bool = false;
            pub(self) const INIT: bool = true;
        }

        use super::*;
        // pub fn get_objects() -> *mut usize {
        //     unsafe { OBJECTS as *mut usize }
        // }

        // fn get_offsets() -> *mut usize {
        //     unsafe { OFFSETS as *mut usize }
        // }

        // fn get_states() -> *mut bool {
        //     unsafe { STATES as *mut bool }
        // }

        // fn get_stack_tops() -> *mut usize {
        //     unsafe { STACK_TOPS as *mut usize }
        // }

        // fn get_stack_ids() -> *mut [PointerTag; STACK_DEPTH] {
        //     unsafe { STACK_TAGS as *mut [PointerTag; STACK_DEPTH] }
        // }

        // fn get_stack_permissions() -> *mut [u8; STACK_DEPTH] {
        //     unsafe { STACK_TAGS as *mut [u8; STACK_DEPTH] }
        // }

        /// Monitors:
        /// If there are K bytes in the address space,
        /// every stacked borrows instrumentation has
        /// between 0 and K monitors.
        /// These monitors track a single byte of the program,
        /// associating it with a stack of pointer values
        /// (represented by tags).
        /// Whenever a pointer borrows an object containing
        /// the byte, its tag is pushed to the stack;
        /// when a read or write is performed through this pointer,
        /// writes from pointers above its location on the stack
        /// are disabled.
        /// This function prepares N monitors,
        /// writes them to global heap memory, then
        /// stores them in pointers.
        /// An N+1th monitor is allocated as a "garbage"
        /// area to be used when no monitor is picked.
        pub fn prepare_monitors() {
            let checked: AliasingChecked = get_checked();
            let _ = checked;
            unsafe {
                OBJECT = 0usize;
                    // vec![0usize; size].into_raw_parts().0 as *const ();
                OFFSET = 0usize;
                    // vec![0usize; size].into_raw_parts().0 as *const ();
                STATE = MonitorState::UNINIT;
                    // vec![MonitorState::UNINIT; size].into_raw_parts().0 as *const ();
                STACK_TAGS = [NEXT_TAG; STACK_DEPTH];
                    // vec![[NEXT_TAG; STACK_DEPTH]; size].into_raw_parts().0 as *const ();
                STACK_PERMS = [Permission::UNIQUE; STACK_DEPTH];
                    // vec![[Permission::UNIQUE; STACK_DEPTH]; size].into_raw_parts().0 as *const ();
                STACK_TOP = 0usize;
                    // vec![0usize; size].into_raw_parts().0 as *const ();
            }
        }

        /// Initialize local when track local is true, picking a monitor,
        /// and setting its object and offset to within pointer.
        pub(super) unsafe fn track_local<U>(tag: u8, pointer: *const U) {
            let checked: AliasingChecked = get_checked();
            let _ = checked;
            // Decide whether to initialize the stacks
            // for location:location+size_of(U).
            // Offset has already been picked earlier.
            unsafe {
                // Pick a monitor nondeterministically
                // use self::*;
                // let states      = get_states();
                // let objects     = get_objects();
                // let offsets     = get_offsets();
                // let stack_ids   = get_stack_ids();
                // let stack_perms = get_stack_permissions();
                // let tops        = get_stack_tops();

                // let mut i = sstate_config::MONITORS.try_into().unwrap();
                // while i > 0 {
                //     i -= 1;
                //     if demonic_nondet() && *states.offset(i) == MonitorState::UNINIT {
                //         let top = *tops.offset(i);
                //         *states.offset(i) = MonitorState::INIT;
                //         *objects.offset(i) = pointer_object(pointer);
                //         assume(*offsets.offset(i) == 0 ||
                //                *offsets.offset(i) < std::mem::size_of::<U>());
                //         (*stack_ids.offset(i))[0] = tag;
                //         (*stack_perms.offset(i))[0] = Permission::UNIQUE;
                //     }
                // }
                if demonic_nondet() && STATE == MonitorState::UNINIT {
                    STATE = MonitorState::INIT;
                    OBJECT =  kani::mem::pointer_object(pointer);
                    assume(OFFSET < std::mem::size_of::<U>());
                    STACK_TAGS[STACK_TOP] = tag;
                    STACK_PERMS[STACK_TOP] = Permission::UNIQUE;
                    STACK_TOP += 1;
                }
            }
        }

        /// Push a tag with a permission perm at pointer
        pub(super) fn push<U>(tag: u8, perm: u8, pointer: *const U) {
            let checked: AliasingChecked = get_checked();
            let _ = checked;
            // Decide whether to initialize the stacks
            // for location:location+size_of(U).
            // Offset has already been picked earlier.
            unsafe {
                // Pick a monitor nondeterministically
                use self::*;
                // let states      = get_states();
                // let objects     = get_objects();
                // let offsets     = get_offsets();
                // let stack_ids   = get_stack_ids();
                // let stack_perms = get_stack_permissions();
                // let tops        = get_stack_tops();

                // let mut i = sstate_config::MONITORS.try_into().unwrap();
                if STATE == MonitorState::INIT &&
                   OBJECT == kani::mem::pointer_object(pointer) &&
                   OFFSET == kani::mem::pointer_offset(pointer)
                {
                    STACK_TAGS[STACK_TOP + 1] = tag;
                    STACK_PERMS[STACK_TOP + 1] = perm;
                    STACK_TOP += 1;
                }
            }
        }

        pub(super) fn stack_check<U>(tag: u8, access: bool, address: *const U) {
            let checked: AliasingChecked = get_checked();
            let _ = checked;
            unsafe {
                use self::*;
                // let states      = get_states();
                // let objects     = get_objects();
                // let offsets     = get_offsets();
                // let stack_ids   = get_stack_ids();
                // let stack_perms = get_stack_permissions();
                // let tops        = get_stack_tops();
                // let mut i = sstate_config::MONITORS.try_into().unwrap();
                if STATE == MonitorState::INIT &&
                   OFFSET == kani::mem::pointer_offset(address) &&
                   OBJECT == kani::mem::pointer_object(address) {
                   let mut found = false;
                   let mut j = 0;
                   let mut new_top = 0;
                   assert!(STACK_TOP < STACK_DEPTH);
                   while j < STACK_DEPTH {
                       if j < STACK_TOP {
                           let id = STACK_TAGS[j];
                           let kind = STACK_PERMS[j];
                           if Permission::grants(access, kind) && id == tag {
                               new_top = j + 1;
                               found = true;
                           }
                       }
                       j += 1;
                   }
                   STACK_TOP = new_top;
                   assert!(found, "Stack violated.");
                }
                // while i > 0 {
                //     {
                //         let top = *tops.offset(i);
                //         let mut found = false;
                //         let mut j = STACK_DEPTH;
                //         let mut new_top = 0;
                //         while j > 0 {
                //         }
                //         assert!(found, "Stack violated.");
                //         *tops.offset(i) = new_top;
                //     }
                // }
            }
        }
    }

    #[rustc_diagnostic_item = "KaniInitializeSState"]
    pub fn initialize() {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        self::monitors::prepare_monitors();
    }

    /// Run a stack check on the pointer value at the given location.
    pub fn stack_check<U>(tag: u8, access: bool, address: *const U) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        self::monitors::stack_check(tag, access, address)
    }

    /// Push the permissions at the given location
    pub fn push<U>(tag: u8, perm: u8, address: *const U) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        self::monitors::push(tag, perm, address)
    }

    /// Initialize the local stored at reference if initialized is set to false,
    /// and track it using a monitor when using demonic nondeterminism.
    ///
    /// Every function call in the source program stack-allocates
    /// the local variables that it uses; references to these
    /// variables are only valid after these variables are initialized (first written).
    /// Therefore this function can be used by supplying an initialized flag
    /// set to true after the first write, a track flag set to the value
    /// of a query to a demonic nondeterminism oracle (when this feature is used)
    /// and a reference to the stack location.
    #[rustc_diagnostic_item = "KaniInitializeLocal"]
    pub fn initialize_local<U>(pointer: *const U) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            let tag = NEXT_TAG;
            TAGS.set(pointer, tag);
            PERMS.set(pointer, Permission::UNIQUE);
            NEXT_TAG += 1;
            monitors::track_local(tag, pointer);
        }
    }

    pub fn use_2<T>(ptr: *const T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            let tag = TAGS.get(ptr);
            // let perm = PERMS[pointer_object(ptr)][pointer_offset(ptr)];
            for i in 0..std::mem::size_of::<T>() {
                stack_check(tag, Access::WRITE, ptr.byte_add(i));
            }
        }
    }

    /// Make a new mutable reference at the rvalue.
    /// Associate the tag with the lvalue.
    pub fn new_mut_ref<T>(lvalue: *const &mut T, rvalue: &mut T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // use_2 the rvalue
            use_2(rvalue as *const T);
            // Then associate the lvalue and push it
            push(NEXT_TAG, Permission::UNIQUE, lvalue);
            // TAGS[pointer_object(lvalue)][pointer_offset(lvalue)] = NEXT_TAG;
            // PERMS[pointer_object(lvalue)][pointer_offset(lvalue)] = Permission::UNIQUE;
            NEXT_TAG += 1;
        }
    }

    /// Make a raw mutable reference at the rvalue.
    /// Associate the tag with the lvalue.
    pub fn new_mut_raw<T>(lvalue: *const *mut T, rvalue: *mut T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // use_2 the rvalue
            use_2(rvalue as *const T);
            // Then associate the lvalue and push it
            push(NEXT_TAG, Permission::SHAREDRW, lvalue);
            // TAGS[pointer_object(lvalue)][pointer_offset(lvalue)] = NEXT_TAG;
            // PERMS[pointer_object(lvalue)][pointer_offset(lvalue)] = Permission::SHAREDRW;
            NEXT_TAG += 1;
        }
    }
}



#[cfg(any(kani))]
fn demonic_nondet() -> bool {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    kani::any::<bool>()
}

#[cfg(not(kani))]
fn demonic_nondet() -> bool {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    true
}

// #[rustc_diagnostic_item = "KaniPushUnique"]
// fn push_unique<U>(pointer: *const U, kind: &mut usize, tag: &mut usize) {
//     push(
//         pointer,
//         &mut POINTER_PERMISSIONS[SSTATE_MONITOR_OBJECT][SSTATE_MONITOR_OFFSET],
//         &mut POINTER_TAGS[SSTATE_MONITOR_OBJECT][SSTATE_MONITOR_OBJECT],
//         KIND_UNIQUE,
//     );
// }

// pub fn push<U>(pointer: *const U, kind: &mut usize, tag: &mut usize, create: PointerValueKind) {
//     unsafe {
//         *tag = 0;
//         if monitored(pointer) {
//             if create == KIND_SHARED_RW {
//                 *tag = SSTATE_NEXT_TAG;
//                 SSTATE_NEXT_TAG += 1;
//             }
//             *kind = create;
//             let top = STATE_STACK_TOPS;
//             assert!(top < STACK_DEPTH);
//             SSTATE_STACK_PERMS[top] = *kind;
//             SSTATE_STACK_TAGS[top] = *tag;
//             SSTATE_STACK_TOPS += 1;
//         }
//     }
// }

// #[rustc_diagnostic_item = "KaniUse2"]
// fn use_2<U>(pointer: *const U) {
//     unsafe {
//         if monitored(pointer) {
//             let top = SSTATE_STACK_TOPS;
//             let mut found = false;
//             assert!(kind != KIND_UNINITIALIZED);
//             let needle_kind = POINTER_PERMISSIONS[pointer_object(pointer)][pointer_offset(pointer)];
//             let needle_id = POINTER_IDS[pointer_object(pointer)][pointer_offset(pointer)];
//             let mut i = 0;
//             let mut new_top = 0;
//             while (i < STACK_DEPTH) && (i < top) {
//                 if SSTATE_STACK_PERMS[i] == to_find && SSTATE_STACK_TAGS[i] == id {
//                     new_top = i + 1;
//                     found = true;
//                 }
//                 i += 1;
//             }
//             SSTATE_STACK_TOPS = new_top;
//             if kind != KIND_UNINITIALIZED {
//             } else {
//                 let mut i = 0;
//                 let mut new_top = 0;
//                 while (i < STACK_DEPTH) && (i < top) {
//                     if SSTATE_STACK_PERMS[i] == KIND_SHARED_RW {
//                         new_top = i + 1;
//                         found = true;
//                     }
//                     i += 1;
//                 }
//                 SSTATE_STACK_TOPS = new_top;
//             }
//             assert!(found, "Stack violated.");
//         }
//     }
// }

// #[rustc_diagnostic_item = "KaniNewMutableRef"]
// fn new_mut_ref<U, T>(reference: *const U, referent: *const T) {
//     use_2(referent);
//     assert!(
//         std::mem::size_of_val(unsafe { &*reference })
//             < std::mem::size_of_val(unsafe { &*referent })
//     );
//     for i in 0..std::mem::size_of_val(unsafe { &*reference }) {
//         push_shared(pointer.byte_offset(i as isize), kind, tag, KIND_SHARED_RW);
//     }
// }

// #[rustc_diagnostic_item = "KaniNewMutableRaw"]
// fn new_mutable_raw<U, T>(pointer: *const U, pointee: *const T) {
//     use_2(pointee);
//     for i in 0..std::mem::size_of_val(unsafe { &*pointer }) {
//         push_shared(pointer.byte_offset(i as isize), kind, tag, KIND_SHARED_RW);
//     }
// }


#[kani::proof]
fn main() {
    let mut local: i32;
    let temp_ref: &mut i32;
    let raw_pointer: *mut i32;
    let ref_from_raw_1: &mut i32;
    let ref_from_raw_2: &mut i32;

    local = 0;
    temp_ref = &mut local;
    raw_pointer = temp_ref as *mut i32;
    unsafe {
        ref_from_raw_1 = &mut *raw_pointer;
        *ref_from_raw_1 = 0;
        ref_from_raw_2 = &mut *raw_pointer;
        *ref_from_raw_2 = 1;
        *ref_from_raw_1 = 2;
    }
}
