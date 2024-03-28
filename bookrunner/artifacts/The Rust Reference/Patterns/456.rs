// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
let c = 'f';
let valid_variable = match c {
    'a'..='z' => true,
    'A'..='Z' => true,
    'α'..='ω' => true,
    _ => false,
};

let ph = 10;
println!("{}", match ph {
    0..=6 => "acid",
    7 => "neutral",
    8..=14 => "base",
    _ => unreachable!(),
});

let uint: u32 = 5;
match uint {
    0 => "zero!",
    1.. => "positive number!",
};

// using paths to constants:
const TROPOSPHERE_MIN : u8 = 6;
const TROPOSPHERE_MAX : u8 = 20;

const STRATOSPHERE_MIN : u8 = TROPOSPHERE_MAX + 1;
const STRATOSPHERE_MAX : u8 = 50;

const MESOSPHERE_MIN : u8 = STRATOSPHERE_MAX + 1;
const MESOSPHERE_MAX : u8 = 85;

let altitude = 70;

println!("{}", match altitude {
    TROPOSPHERE_MIN..=TROPOSPHERE_MAX => "troposphere",
    STRATOSPHERE_MIN..=STRATOSPHERE_MAX => "stratosphere",
    MESOSPHERE_MIN..=MESOSPHERE_MAX => "mesosphere",
    _ => "outer space, maybe",
});

pub mod binary {
    pub const MEGA : u64 = 1024*1024;
    pub const GIGA : u64 = 1024*1024*1024;
}
let n_items = 20_832_425;
let bytes_per_item = 12;
if let size @ binary::MEGA..=binary::GIGA = n_items * bytes_per_item {
    println!("It fits and occupies {} bytes", size);
}

trait MaxValue {
    const MAX: u64;
}
impl MaxValue for u8 {
    const MAX: u64 = (1 << 8) - 1;
}
impl MaxValue for u16 {
    const MAX: u64 = (1 << 16) - 1;
}
impl MaxValue for u32 {
    const MAX: u64 = (1 << 32) - 1;
}
// using qualified paths:
println!("{}", match 0xfacade {
    0 ..= <u8 as MaxValue>::MAX => "fits in a u8",
    0 ..= <u16 as MaxValue>::MAX => "fits in a u16",
    0 ..= <u32 as MaxValue>::MAX => "fits in a u32",
    _ => "too big",
});
}