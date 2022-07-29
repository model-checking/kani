// kani-check-fail
// compile-flags: --edition 2021
#![allow(unused)]
// invalid suffixes

fn main() {
0invalidSuffix;

// uses numbers of the wrong base

123AFB43;
0b0102;
0o0581;

// integers too big for their type (they overflow)

128_i8;
256_u8;

// bin, hex, and octal literals must have at least one digit

0b_;
0b____;
}