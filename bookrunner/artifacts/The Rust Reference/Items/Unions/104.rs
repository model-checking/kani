// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
union MyUnion { f1: u32, f2: f32 }

fn f(u: MyUnion) {
    unsafe {
        match u {
            MyUnion { f1: 10 } => { println!("ten"); }
            MyUnion { f2 } => { println!("{}", f2); }
        }
    }
}
}