fn modify_check_6e1104(ptr: &mut Box<u32>) {
    let _wrapper_arg_0 = kani::untracked_deref(&*ptr.as_ref());
    kani::assume(**ptr < 100);
    let result: () = modify_wrapper_6e1104(ptr, _wrapper_arg_0);
    result
}
fn modify_replace_6e1104(ptr: &mut Box<u32>) {
    kani::assert(false, "Replacement with modifies is not supported yet.")
}
fn modify_wrapper_6e1104<'_wrapper_arg_0>(ptr: &mut Box<u32>,
    _wrapper_arg_0: &'_wrapper_arg_0 impl kani::Arbitrary) {
    *ptr.as_mut() += 1;
}
fn main() { let mut i = Box::new(kani::any()); modify_check_6e1104(&mut i); }