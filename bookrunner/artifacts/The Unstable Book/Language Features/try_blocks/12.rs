// compile-flags: --edition 2018
#![allow(unused)]
#![feature(try_blocks)]

fn main() {
use std::num::ParseIntError;

let result: Result<i32, ParseIntError> = try {
    "1".parse::<i32>()?
        + "2".parse::<i32>()?
        + "3".parse::<i32>()?
};
assert_eq!(result, Ok(6));

let result: Result<i32, ParseIntError> = try {
    "1".parse::<i32>()?
        + "foo".parse::<i32>()?
        + "3".parse::<i32>()?
};
assert!(result.is_err());
}