// compile-flags: -Zsave-analysis
// needs-asm-support
// Also test for #72960

use std::arch::asm;

fn main() {
    unsafe {
        asm!("", in("invalid") "".len());
        //~^ ERROR: invalid register `invalid`: unknown register
    }
}
