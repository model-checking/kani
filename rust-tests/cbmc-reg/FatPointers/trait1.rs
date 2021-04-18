/// Cast a concrete ref to a trait ref.

pub trait Subscriber {
    fn process(&mut self);
}

struct DummySubscriber {}

impl DummySubscriber {
    fn new() -> Self {
        DummySubscriber {}
    }
}

impl Subscriber for DummySubscriber {
    fn process(&mut self) {}
}

fn main() {
    let _d = DummySubscriber::new();
    let _s = &_d as &dyn Subscriber;
}
