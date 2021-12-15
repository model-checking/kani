// FIXME(fee1-dead): this should have a better error message
#![feature(const_trait_impl)]
struct NonConstAdd(i32);

impl std::ops::Add for NonConstAdd {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        NonConstAdd(self.0 + rhs.0)
    }
}

trait Foo {
    type Bar: ~const std::ops::Add;
}

impl const Foo for NonConstAdd {
    type Bar = NonConstAdd;
    //~^ ERROR
}

trait Baz {
    type Qux: std::ops::Add;
}

impl const Baz for NonConstAdd {
    type Qux = NonConstAdd; // OK
}

fn main() {}
