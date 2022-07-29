// compile-flags: --edition 2015
#![allow(unused)]
#![feature(doc_notable_trait)]

fn main() {
#[doc(notable_trait)]
pub trait MyTrait {}

pub struct MyStruct;
impl MyTrait for MyStruct {}

/// The docs for this function will have a button that displays a dialog about
/// `MyStruct` implementing `MyTrait`.
pub fn my_fn() -> MyStruct { MyStruct }
}