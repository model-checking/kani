// compile-flags: --edition 2021
#![allow(unused)]
// bad: the block's type is i32, not ()
// Error: expected `()` because of default return type
// if true {
//   1
// }

// good: the block's type is i32
fn main() {
if true {
  1
} else {
  2
};
}