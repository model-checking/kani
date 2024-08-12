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
                OFFSET = 0usize;
                STATE = MonitorState::UNINIT;
                STACK_TAGS = [NEXT_TAG; STACK_DEPTH];
                STACK_PERMS = [Permission::UNIQUE; STACK_DEPTH];
                STACK_TOP = 0usize;
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

    #[rustc_diagnostic_item = "KaniWriteThroughPointer"]
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

    /// Make a new mutable reference at the rvalue (pointer_value).
    /// Associate the tag with the lvalue (location).
    #[rustc_diagnostic_item = "KaniNewMutRef"]
    pub fn new_mut_ref<T>(location: *const &mut T, pointer_value: &mut T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // use_2 the rvalue in the case it is set
            use_2(pointer_value as *const T);
            // Then associate the lvalue and push it
            TAGS.set(location, NEXT_TAG);
            push(NEXT_TAG, Permission::UNIQUE, location);
            NEXT_TAG += 1;
        }
    }

    /// Make a raw mutable reference at the rvalue (pointer_value).
    /// Associate the tag with the lvalue (location).
    #[rustc_diagnostic_item = "KaniNewMutRaw"]
    pub fn new_mut_raw<T>(location: *const *mut T, pointer_value: *mut T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // use_2 the rvalue
            use_2(pointer_value);
            // Then associate the lvalue and push it
            TAGS.set(location, NEXT_TAG);
            push(NEXT_TAG, Permission::SHAREDRW, location);
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
