// compile-flags: --edition 2018
#![allow(unused)]
fn main() {
use std::sync::Arc;

#[derive(Clone)]
struct Container<T>(Arc<T>);

fn clone_containers<T>(foo: &Container<i32>, bar: &Container<T>) {
    let foo_cloned = foo.clone();
    let bar_cloned = bar.clone();
}
}