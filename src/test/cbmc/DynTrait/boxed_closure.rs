fn main() {
    // Create a boxed once-callable closure
    let f: Box<dyn FnOnce()> = Box::new(|| {
    });

    // Call it
    f();
}