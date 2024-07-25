# Interior Mutability

In Rust, if a struct is immutable, all fields within it are immutable. However, we can still mutate some of these fields via an interface that allows us to do so: https://doc.rust-lang.org/std/cell/.

In Goto, there is no distinction between immutable and mutable fields, thus breaking this encapsulation that Rust provides. We can leverage this encapsulation breaking mechanic to write verified proofs of Rust interior mutability, as long as we preserve the overall properties that the encapsulation would have given us.

It is possible to represent the abstraction breaking mechanics within unsafe Rust for `UnsafeCell` and `Cell` as explained by the memory layout properies ensured during compilation: https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html#memory-layout. While we cannot access private fields in Rust, we can interpret the fields as an arbitrary pointer to the type of the field via a transmute if we know the data layout of the field. It is thus possible to safely verify code for `UnsafeCell` and `Cell` within Kani by breaking the encapsulation with unsafe rust.

`OnceCell` follows the pattern of having only one field within the struct, thus access to this struct via an unsafe transmute is predictable within the Kani compiler, although this behavior is not explicitly explained in Rust documentation as it is for `UnsafeCell` and `Cell`. We can verify code for `OnceCell` within Kani, but we need to be more careful in assessing whether this is UB or not.

This is much harder for `RefCell` as it is UB to rely on the particular order of fields in a struct, and there might be inconsistent padding through compilation. It should be possible to attach a compilation tool within Kani to expose these private fields after they have been compiled to Goto, but this still creates risk that these private fields could be modified or accessed in a way that is UB to do so from the Rust standpoint.

We focus on single threaded computation, so we avoid verifying `RwLock`, `Mutex` and similar other constructs for now.