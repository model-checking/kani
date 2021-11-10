// run-rustfix

#![allow(unreachable_code)]
#![allow(unused_macros)]
#![allow(unused_variables)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::eq_op)]
#![allow(clippy::print_literal)]
#![warn(clippy::to_string_in_format_args)]

use std::io::{stdout, Write};
use std::ops::Deref;
use std::panic::Location;

struct Somewhere;

impl ToString for Somewhere {
    fn to_string(&self) -> String {
        String::from("somewhere")
    }
}

struct X(u32);

impl Deref for X {
    type Target = u32;

    fn deref(&self) -> &u32 {
        &self.0
    }
}

struct Y<'a>(&'a X);

impl<'a> Deref for Y<'a> {
    type Target = &'a X;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct Z(u32);

impl Deref for Z {
    type Target = u32;

    fn deref(&self) -> &u32 {
        &self.0
    }
}

impl std::fmt::Display for Z {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Z")
    }
}

macro_rules! my_macro {
    () => {
        // here be dragons, do not enter (or lint)
        println!("error: something failed at {}", Location::caller().to_string());
    };
}

macro_rules! my_other_macro {
    () => {
        Location::caller().to_string()
    };
}

fn main() {
    let x = &X(1);
    let x_ref = &x;

    let _ = format!("error: something failed at {}", Location::caller().to_string());
    let _ = write!(
        stdout(),
        "error: something failed at {}",
        Location::caller().to_string()
    );
    let _ = writeln!(
        stdout(),
        "error: something failed at {}",
        Location::caller().to_string()
    );
    print!("error: something failed at {}", Location::caller().to_string());
    println!("error: something failed at {}", Location::caller().to_string());
    eprint!("error: something failed at {}", Location::caller().to_string());
    eprintln!("error: something failed at {}", Location::caller().to_string());
    let _ = format_args!("error: something failed at {}", Location::caller().to_string());
    assert!(true, "error: something failed at {}", Location::caller().to_string());
    assert_eq!(0, 0, "error: something failed at {}", Location::caller().to_string());
    assert_ne!(0, 0, "error: something failed at {}", Location::caller().to_string());
    panic!("error: something failed at {}", Location::caller().to_string());
    println!("{}", X(1).to_string());
    println!("{}", Y(&X(1)).to_string());
    println!("{}", Z(1).to_string());
    println!("{}", x.to_string());
    println!("{}", x_ref.to_string());
    // https://github.com/rust-lang/rust-clippy/issues/7903
    println!("{foo}{bar}", foo = "foo".to_string(), bar = "bar");
    println!("{foo}{bar}", foo = "foo", bar = "bar".to_string());
    println!("{foo}{bar}", bar = "bar".to_string(), foo = "foo");
    println!("{foo}{bar}", bar = "bar", foo = "foo".to_string());

    // negative tests
    println!("error: something failed at {}", Somewhere.to_string());
    // The next two tests are negative because caching the string might be faster than calling `<X as
    // Display>::fmt` twice.
    println!("{} and again {0}", x.to_string());
    println!("{foo}{foo}", foo = "foo".to_string());
    my_macro!();
    println!("error: something failed at {}", my_other_macro!());
    // https://github.com/rust-lang/rust-clippy/issues/7903
    println!("{foo}{foo:?}", foo = "foo".to_string());
}
