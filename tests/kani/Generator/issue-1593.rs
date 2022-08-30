use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

#[kani::proof]
fn issue_1593() {
    let x = Arc::new(AtomicI64::new(0));
    let x2 = x.clone();
    let gen = async move {
        async {}.await;
        x2.fetch_add(1, Ordering::Relaxed);
    };
    assert_eq!(std::mem::size_of_val(&gen), 16);
    let pinbox = Box::pin(gen); // check that vtables work
}
