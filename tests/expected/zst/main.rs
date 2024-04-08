#[repr(C)]
#[derive(Copy, Clone)]
struct Z(i8, i64);

struct Y;

#[kani::proof]
fn test_z() -> Z {
    let m = Y;
    let n = Y;
    let zz = Z(1, -1);

    let ptr: *const Z = if &n as *const _ == &m as *const _ { std::ptr::null() } else { &zz };
    unsafe { *ptr }
}
