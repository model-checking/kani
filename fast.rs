#![feature(core_intrinsics)]

fn main() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();
    kani::assume(x.is_finite());
    kani::assume(y.is_finite());
    kani::assume(x + y < f64::MAX);
    kani::assume(x + y > f64::MIN);

    let z = unsafe { std::intrinsics::fadd_fast(x, y) };
    let z2 = x + y;
}