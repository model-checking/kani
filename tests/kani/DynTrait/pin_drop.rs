use std::pin::Pin;

trait Trait {
    // Note: This also fails with Box.
    fn dummy(self: Pin<&mut Self>);
}

struct UnimplementedTrait {}

impl Trait for UnimplementedTrait {
    fn dummy(self: Pin<&mut Self>) {
        unimplemented!();
    }
}

#[kani::proof]
fn check_drop_in_poll() {
    let _future1 = UnimplementedTrait {};
}

pub struct LocalTraitObj<'a> {
    pub future: &'a dyn Trait,
}

impl<'a> Drop for LocalTraitObj<'a> {
    fn drop(&mut self) {}
}
