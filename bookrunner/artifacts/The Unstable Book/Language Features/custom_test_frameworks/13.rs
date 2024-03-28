// compile-flags: --edition 2015
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(my_runner)]

fn main() {
fn my_runner(tests: &[&i32]) {
    for t in tests {
        if **t == 0 {
            println!("PASSED");
        } else {
            println!("FAILED");
        }
    }
}

#[test_case]
const WILL_PASS: i32 = 0;

#[test_case]
const WILL_FAIL: i32 = 4;
}