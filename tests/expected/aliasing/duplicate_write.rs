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

#[cfg(any(kani))]
extern crate kani;
#[cfg(any(kani))]
use kani::shadow::ShadowMem;
#[cfg(not(kani))]
use std::collections::HashMap;
#[cfg(not(kani))]
#[derive(Debug)]
struct ShadowMem<T>{
    mem: Option<HashMap<usize, T>>,
    default: T
}
#[cfg(not(kani))]
static mut FOUND_POINTERS: Option<HashMap<*const u8, usize>> = None;
#[cfg(not(kani))]
static mut FOUND_POINTERS_TOP: usize = 0;

#[cfg(not(kani))]
impl<T> ShadowMem<T> where T: Copy {
    #[inline(always)]
    const fn new(v: T) -> Self {
        ShadowMem { mem: None, default: v }
    }

    fn set<U>(&mut self, pointer: *const U, value: T) {
        unsafe {
            let map = FOUND_POINTERS.get_or_insert(HashMap::new());
            let e = map.entry(pointer as *const u8)
                .or_insert_with(|| { let top = FOUND_POINTERS_TOP; FOUND_POINTERS_TOP += 1; top } );
            let mem = self.mem.get_or_insert(HashMap::new());
            mem.insert(*e, value);
        }
    }

    fn get<U>(&self, pointer: *const U) -> T {
        unsafe {
            let map = FOUND_POINTERS.get_or_insert(HashMap::new());
            let pointer = pointer as *const u8;
            *(self.mem.as_ref().unwrap().get(map.get(&pointer).unwrap()).unwrap_or(&self.default))
        }
    }
}

#[inline(never)]
fn get_checked() -> AliasingChecked {
    static mut CHECKED: AliasingChecked = AliasingChecked { amount: 0 };
    unsafe { CHECKED.amount = CHECKED.amount.wrapping_add(1);  CHECKED }
}

#[cfg(any(kani))]
fn pointer_object<U>(pointer: *const U) -> usize {
    let checked = get_checked();
    let _ = checked;
    kani::mem::pointer_object(pointer)
}

#[cfg(not(kani))]
fn pointer_object<U>(pointer: *const U) -> usize {
    pointer as usize
}

#[cfg(any(kani))]
fn pointer_offset<U>(pointer: *const U) -> usize {
    let checked = get_checked();
    let _ = checked;
    kani::mem::pointer_offset(pointer)
}

#[cfg(not(kani))]
fn pointer_offset<U>(_pointer: *const U) -> usize {
    0
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

    #[cfg(not(kani))]
    pub fn debug() {
        unsafe {
            println!("tags & perms at this point: tags: {:?} perms: {:?}", sstate::TAGS, sstate::PERMS);
            self::monitors::debug_monitors();
        }
    }

    #[cfg(any(kani))]
    #[inline(always)]
    pub fn debug() {
        let checked = get_checked();
        let _ = checked;
        return;
    }

    pub(super) mod monitors {
        static mut STATE: bool = false;
        static mut OBJECT: usize = 0;
        static mut OFFSET: usize = 0;
        static mut STACK_TAGS: [u8; STACK_DEPTH] = [0; STACK_DEPTH];
        static mut STACK_PERMS: [u8; STACK_DEPTH] = [0; STACK_DEPTH];
        static mut STACK_TOP: usize = 0;

        #[cfg(not(kani))]
        pub fn debug_monitors() {
            unsafe {
                println!("monitors at this point state: {:?} object: {:?} offset: {:?} stags: {:?} sperms: {:?} stop: {:?}",
                         STATE, OBJECT, OFFSET, STACK_TAGS, STACK_PERMS, STACK_TOP);
            }
        }

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
                    OBJECT = pointer_object(pointer);
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
                use self::*;
                if STATE == MonitorState::INIT &&
                   OBJECT == pointer_object(pointer) &&
                   OFFSET == pointer_offset(pointer)
                {
                    STACK_TAGS[STACK_TOP] = tag;
                    STACK_PERMS[STACK_TOP] = perm;
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
                   OFFSET == pointer_offset(address) &&
                   OBJECT == pointer_object(address) {
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

    #[rustc_diagnostic_item = "KaniStackCheckPtr"]
    pub fn stack_check_ptr<U>(pointer_value: *const *mut U) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            let tag = TAGS.get(pointer_value);
            let perm = PERMS.get(pointer_value);
            let pointer = *pointer_value;
            for i in 0..std::mem::size_of::<U>() {
                for access in [false, true] {
                    if Permission::grants(access, perm) {
                        self::monitors::stack_check(tag, access, pointer.byte_add(i));
                    }
                }
            }
        }
    }

    #[rustc_diagnostic_item = "KaniStackCheckRef"]
    pub fn stack_check_ref<U>(pointer_value: *const &mut U) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        stack_check_ptr(pointer_value as *const *mut U);
    }

    /// Update the stacked borrows state for the case created: &mut T = &mut (referent:T)
    /// by associating the location of the created value, stored at pointer_to_created,
    /// with a new tag, and pushing the new tag to the created reference, stored at
    /// pointer_to_val.
    #[rustc_diagnostic_item = "KaniNewMutRefFromLocal"]
    pub fn new_mut_ref_from_value<T>(pointer_to_created: *const &mut T, pointer_to_val: *const T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // Then associate the lvalue and push it
            TAGS.set(pointer_to_created, NEXT_TAG);
            push(NEXT_TAG, Permission::SHAREDRW, pointer_to_val);
            NEXT_TAG += 1;
        }
    }

    /// Update the stacked borrows state for the case created = (reference: &mut T) as *mut T,
    /// associating the location of the created value, stored at pointer_to_created, with a new
    /// tag, running a stack check on the tag associated with the reference, accessed by
    /// pointer_to_ref, and pushing the tag to the original location.
    #[rustc_diagnostic_item = "KaniNewMutRawFromRef"]
    pub fn new_mut_raw_from_ref<T>(pointer_to_created: *const *mut T, pointer_to_ref: *const &mut T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // Then associate the lvalue and push it
            TAGS.set(pointer_to_created, NEXT_TAG);
            push(NEXT_TAG, Permission::SHAREDRW, *pointer_to_ref);
            NEXT_TAG += 1;
        }
    }

    /// Update the stacked borrows state for the case created = (reference: &mut T) as *mut T,
    /// associating the location of the created value, stored at pointer_to_created, with a new
    /// tag, running a stack check on the tag associated with the reference, accessed by
    /// pointer_to_ref, and pushing the tag to the original location.
    #[rustc_diagnostic_item = "KaniNewMutRefFromRaw"]
    pub fn new_mut_ref_from_raw<T>(pointer_to_created: *const &mut T, pointer_to_ref: *const *mut T) {
        let checked: AliasingChecked = get_checked();
        let _ = checked;
        unsafe {
            // Then associate the lvalue and push it
            TAGS.set(pointer_to_created, NEXT_TAG);
            push(NEXT_TAG, Permission::SHAREDRW, *pointer_to_ref);
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

#[cfg(not(kani))]
static mut LOCAL_COUNT: u32 = 0;

#[cfg(not(kani))]
fn initialize_local<T>(local: *const T) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    unsafe {
        println!("Initializing local {:?}", LOCAL_COUNT);
    }
    sstate::initialize_local(local);
    sstate::debug();
    unsafe { LOCAL_COUNT += 1 };
}

#[cfg(not(kani))]
fn new_mut_ref_from_value<T>(pointer_to_created: *const &mut T, pointer_to_val: *const T) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    println!("new mut ref from value");
    sstate::new_mut_ref_from_value(pointer_to_created, pointer_to_val);
    sstate::debug();
}

#[cfg(not(kani))]
pub fn new_mut_raw_from_ref<T>(pointer_to_created: *const *mut T, pointer_to_ref: *const &mut T) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    println!("new mut raw from ref");
    sstate::new_mut_raw_from_ref(pointer_to_created, pointer_to_ref);
    sstate::debug();
}

#[cfg(not(kani))]
pub fn new_mut_ref_from_raw<T>(pointer_to_created: *const &mut T, pointer_to_ref: *const *mut T) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    println!("new mut ref from raw");
    sstate::new_mut_ref_from_raw(pointer_to_created, pointer_to_ref);
    sstate::debug();
}

pub fn stack_check_ptr<U>(pointer_value: *const *mut U) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    println!("checking ptr on stack");
    sstate::stack_check_ptr(pointer_value);
    sstate::debug();
}

pub fn stack_check_ref<U>(pointer_value: *const &mut U) {
    let checked: AliasingChecked = get_checked();
    let _ = checked;
    println!("checking ref on stack");
    sstate::stack_check_ref(pointer_value);
    sstate::debug();
}

#[cfg_attr(any(kani), kani::proof)]
fn main() {
    let mut local: i32;
    let temp_ref: &mut i32;
    let raw_pointer: *mut i32;
    let ref_from_raw_1: &mut i32;
    let ref_from_raw_2: &mut i32;

    local = 0;
    #[cfg(not(kani))]
    initialize_local(std::ptr::addr_of!(local));
    temp_ref = &mut local;
    #[cfg(not(kani))]
    initialize_local(std::ptr::addr_of!(temp_ref));
    raw_pointer = temp_ref as *mut i32;
    #[cfg(not(kani))]
    initialize_local(std::ptr::addr_of!(raw_pointer));
    #[cfg(not(kani))]
    new_mut_ref_from_value(std::ptr::addr_of!(temp_ref),
                           temp_ref);
    #[cfg(not(kani))]
    stack_check_ref(std::ptr::addr_of!(temp_ref));
    #[cfg(not(kani))]
    new_mut_raw_from_ref(std::ptr::addr_of!(raw_pointer), std::ptr::addr_of!(temp_ref));
    unsafe {
        ref_from_raw_1 = &mut *raw_pointer;
        #[cfg(not(kani))]
        new_mut_ref_from_raw(std::ptr::addr_of!(ref_from_raw_1), std::ptr::addr_of!(raw_pointer));
        *ref_from_raw_1 = 0;
        #[cfg(not(kani))]
        stack_check_ref(std::ptr::addr_of!(ref_from_raw_1));
        ref_from_raw_2 = &mut *raw_pointer;
        #[cfg(not(kani))]
        stack_check_ptr(std::ptr::addr_of!(raw_pointer));
        #[cfg(not(kani))]
        new_mut_ref_from_raw(std::ptr::addr_of!(ref_from_raw_2), std::ptr::addr_of!(raw_pointer));
        *ref_from_raw_2 = 1;
        #[cfg(not(kani))]
        stack_check_ref(std::ptr::addr_of!(ref_from_raw_2));
        *ref_from_raw_1 = 2;
        #[cfg(not(kani))]
        stack_check_ref(std::ptr::addr_of!(ref_from_raw_1));
    }
}
