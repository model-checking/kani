// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
use std::rc::Rc;
use std::sync::Arc;
use std::pin::Pin;
// Examples of object safe methods.
trait TraitMethods {
    fn by_ref(self: &Self) {}
    fn by_ref_mut(self: &mut Self) {}
    fn by_box(self: Box<Self>) {}
    fn by_rc(self: Rc<Self>) {}
    fn by_arc(self: Arc<Self>) {}
    fn by_pin(self: Pin<&Self>) {}
    fn with_lifetime<'a>(self: &'a Self) {}
    fn nested_pin(self: Pin<Arc<Self>>) {}
}
struct S;
impl TraitMethods for S {}
let t: Box<dyn TraitMethods> = Box::new(S);
}