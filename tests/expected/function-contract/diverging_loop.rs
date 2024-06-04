#[kani::ensures(result == 1)]
fn foo() -> i32 {
    loop {}
    2
}

#[kani::proof_for_contract(foo)]
#[kani::unwind(1)]
fn check_foo() {
    let _ = foo();
}
