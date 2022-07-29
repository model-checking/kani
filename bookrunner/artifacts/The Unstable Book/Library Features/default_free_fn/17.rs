// compile-flags: --edition 2015
#![allow(unused)]
#![feature(default_free_fn)]
use std::default::default;

#[derive(Default)]
struct AppConfig {
    foo: FooConfig,
    bar: BarConfig,
}

#[derive(Default)]
struct FooConfig {
    foo: i32,
}

#[derive(Default)]
struct BarConfig {
    bar: f32,
    baz: u8,
}

fn main() {
    let options = AppConfig {
        foo: default(),
        bar: BarConfig {
            bar: 10.1,
            ..default()
        },
    };
}