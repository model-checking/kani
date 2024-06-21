// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Kani implementation of function contracts.
//!
//! Function contracts are still under development. Using the APIs therefore
//! requires the unstable `-Zfunction-contracts` flag to be passed. You can join
//! the discussion on contract design by reading our
//! [RFC](https://model-checking.github.io/kani/rfc/rfcs/0009-function-contracts.html)
//! and [commenting on the tracking
//! issue](https://github.com/model-checking/kani/issues/2652).
//!
//! The function contract API is expressed as proc-macro attributes, and there
//! are two parts to it.
//!
//! 1. [Contract specification attributes](#specification-attributes-overview):
//!    [`requires`][macro@requires] and [`ensures`][macro@ensures].
//! 2. [Contract use attributes](#contract-use-attributes-overview):
//!    [`proof_for_contract`][macro@proof_for_contract] and
//!    [`stub_verified`][macro@stub_verified].
//!
//! ## Step-by-step Guide
//!
//! Let us explore using a workflow involving contracts on the example of a
//! simple division function `my_div`:
//!
//! ```
//! fn my_div(dividend: u32, divisor: u32) -> u32 {
//!   dividend / divisor
//! }
//! ```
//!
//! With the contract specification attributes we can specify the behavior of
//! this function declaratively. The [`requires`][macro@requires] attribute
//! allows us to declare constraints on what constitutes valid inputs to our
//! function. In this case we would want to disallow a divisor that is `0`.
//!
//! ```ignore
//! #[requires(divisor != 0)]
//! ```
//!
//! This is called a precondition, because it is enforced before (pre-) the
//! function call. As you can see attribute has access to the functions
//! arguments. The condition itself is just regular Rust code. You can use any
//! Rust code, including calling functions and methods. However you may not
//! perform I/O (like [`println!`]) or mutate memory (like [`Vec::push`]).
//!
//! The [`ensures`][macro@ensures] attribute on the other hand lets us describe
//! the output value in terms of the inputs. You may be as (im)precise as you
//! like in the [`ensures`][macro@ensures] clause, depending on your needs. One
//! approximation of the result of division for instance could be this:
//!
//! ```
//! #[ensures(|result : &u32| *result <= dividend)]
//! ```
//!
//! This is called a postcondition and it also has access to the arguments and
//! is expressed in regular Rust code. The same restrictions apply as did for
//! [`requires`][macro@requires]. In addition to the postcondition is expressed
//! as a closure where the value returned from the function is passed to this
//! closure by reference.
//!
//! You may combine as many [`requires`][macro@requires] and
//! [`ensures`][macro@ensures] attributes on a single function as you please.
//! They all get enforced (as if their conditions were `&&`ed together) and the
//! order does not matter. In our example putting them together looks like this:
//!
//! ```
//! #[kani::requires(divisor != 0)]
//! #[kani::ensures(|result : &u32| *result <= dividend)]
//! fn my_div(dividend: u32, divisor: u32) -> u32 {
//!   dividend / divisor
//! }
//! ```
//!
//! Once we are finished specifying our contract we can ask Kani to check it's
//! validity. For this we need to provide a proof harness that exercises the
//! function. The harness is created like any other, e.g. as a test-like
//! function with inputs and using `kani::any` to create arbitrary values.
//! However we do not need to add any assertions or assumptions about the
//! inputs, Kani will use the pre- and postconditions we have specified for that
//! and we use the [`proof_for_contract`][macro@proof_for_contract] attribute
//! instead of [`proof`](crate::proof) and provide it with the path to the
//! function we want to check.
//!
//! ```
//! #[kani::proof_for_contract(my_div)]
//! fn my_div_harness() {
//!     my_div(kani::any(), kani::any()) }
//! ```
//!
//! The harness is checked like any other by running `cargo kani` and can be
//! specifically selected with `--harness my_div_harness`.
//!
//! Once we have verified that our contract holds, we can use perhaps it's
//! coolest feature: verified stubbing. This allows us to use the conditions of
//! the contract *instead* of it's implementation. This can be very powerful for
//! expensive implementations (involving loops for instance).
//!
//! Verified stubbing is available to any harness via the
//! [`stub_verified`][macro@stub_verified] harness attribute. We must provide
//! the attribute with the path to the function to stub, but unlike with
//! [`stub`](crate::stub) we do not need to provide a function to replace with,
//! the contract will be used automatically.
//!
//! ```
//! #[kani::proof]
//! #[kani::stub_verified(my_div)]
//! fn use_div() {
//!   let v = vec![...];
//!   let some_idx = my_div(v.len() - 1, 3);
//!   v[some_idx];
//! }
//! ```
//!
//! In this example the contract is sufficient to prove that the element access
//! in the last line cannot be out-of-bounds.
//!
//! ## Specification Attributes Overview
//!
//! The basic two specification attributes available for describing
//! function behavior are [`requires`][macro@requires] for preconditions and
//! [`ensures`][macro@ensures] for postconditions. Both admit arbitrary Rust
//! expressions as their bodies which may also reference the function arguments
//! but must not mutate memory or perform I/O. The postcondition may
//! additionally reference the return value of the function as the variable
//! `result`.
//!
//! In addition Kani provides the [`modifies`](macro@modifies) attribute. This
//! works a bit different in that it does not contain conditions but a comma
//! separated sequence of expressions that evaluate to pointers. This attribute
//! constrains to which memory locations the function is allowed to write. Each
//! expression can contain arbitrary Rust syntax, though it may not perform side
//! effects and it is also currently unsound if the expression can panic. For more
//! information see the [write sets](#write-sets) section.
//!
//! During verified stubbing the return value of a function with a contract is
//! replaced by a call to `kani::any`. As such the return value must implement
//! the `kani::Arbitrary` trait.
//!
//! In Kani, function contracts are optional. As such a function with at least
//! one specification attribute is considered to "have a contract" and any
//! absent specification type defaults to its most general interpretation
//! (`true`). All functions with not a single specification attribute are
//! considered "not to have a contract" and are ineligible for use as the target
//! of a [`proof_for_contract`][macro@proof_for_contract] of
//! [`stub_verified`][macro@stub_verified] attribute.
//!
//! ## Contract Use Attributes Overview
//!
//! Contract are used both to verify function behavior and to leverage the
//! verification result as a sound abstraction.
//!
//! Verifying function behavior currently requires the designation of at least
//! one checking harness with the
//! [`proof_for_contract`](macro@proof_for_contract) attribute. A harness may
//! only have one `proof_for_contract` attribute and it may not also have a
//! `proof` attribute.
//!
//! The checking harness is expected to set up the arguments that `foo` should
//! be called with and initialized any `static mut` globals that are reachable.
//! All of these should be initialized to as general value as possible, usually
//! achieved using `kani::any`. The harness must call e.g. `foo` at least once
//! and if `foo` has type parameters, only one instantiation of those parameters
//! is admissible. Violating either results in a compile error.
//!
//! If any inputs have special invariants you *can* use `kani::assume` to
//! enforce them but this may introduce unsoundness. In general all restrictions
//! on input parameters should be part of the [`requires`](macro@requires)
//! clause of the function contract.
//!
//! Once the contract has been verified it may be used as a verified stub. For
//! this the [`stub_verified`](macro@stub_verified) attribute is used.
//! `stub_verified` is a harness attribute, like
//! [`unwind`](macro@crate::unwind), meaning it is used on functions that are
//! annotated with [`proof`](macro@crate::proof). It may also be used on a
//! `proof_for_contract` proof.
//!
//! Unlike `proof_for_contract` multiple `stub_verified` attributes are allowed
//! on the same proof harness though they must target different functions.
//!
//! ## Inductive Verification
//!
//! Function contracts by default use inductive verification to efficiently
//! verify recursive functions. In inductive verification a recursive function
//! is executed once and every recursive call instead uses the contract
//! replacement. In this way many recursive calls can be checked with a
//! single verification pass.
//!
//! The downside of inductive verification is that the return value of a
//! contracted function must implement `kani::Arbitrary`. Due to restrictions to
//! code generation in proc macros, the contract macros cannot determine reliably
//! in all cases whether a given function with a contract is recursive. As a
//! result it conservatively sets up inductive verification for every function
//! and requires the `kani::Arbitrary` constraint for contract checks.
//!
//! If you feel strongly about this issue you can join the discussion on issue
//! [#2823](https://github.com/model-checking/kani/issues/2823) to enable
//! opt-out of inductive verification.
//!
//! ## Write Sets
//!
//! The [`modifies`](macro@modifies) attribute is used to describe which
//! locations in memory a function may assign to. The attribute contains a comma
//! separated series of expressions that reference the function arguments.
//! Syntactically any expression is permissible, though it may not perform side
//! effects (I/O, mutation) or panic. As an example consider this super simple
//! function:
//!
//! ```
//! #[kani::modifies(ptr, my_box.as_ref())]
//! fn a_function(ptr: &mut u32, my_box: &mut Box<u32>) {
//!     *ptr = 80;
//!     *my_box.as_mut() = 90;
//! }
//! ```
//!
//! Because the function performs an observable side-effect (setting both the
//! value behind the pointer and the value pointed-to by the box) we need to
//! provide a `modifies` attribute. Otherwise Kani will reject a contract on
//! this function.
//!
//! An expression used in a `modifies` clause must return a pointer to the
//! location that you would like to allow to be modified. This can be any basic
//! Rust pointer type (`&T`, `&mut T`, `*const T` or `*mut T`). In addition `T`
//! must implement [`Arbitrary`](super::Arbitrary). This is used to assign
//! `kani::any()` to the location when the function is used in a `stub_verified`.
//!
//! ## History Expressions
//!
//! Additionally, an ensures clause is allowed to refer to the state of the function arguments before function execution and perform simple computations on them
//! via an `old` monad. Any instance of `old(computation)` will evaluate the
//! computation before the function is called. It is required that this computation
//! is effect free and closed with respect to the function arguments.
//!
//! For example, the following code passes kani tests:
//!
//! ```
//! #[kani::modifies(a)]
//! #[kani::ensures(|result| old(*a).wrapping_add(1) == *a)]
//! #[kani::ensures(|result : &u32| old(*a).wrapping_add(1) == *result)]
//! fn add1(a : &mut u32) -> u32 {
//!     *a=a.wrapping_add(1);
//!     *a
//! }
//! ```
//!
//! Here, the value stored in `a` is precomputed and remembered after the function
//! is called, even though the contents of `a` changed during the function execution.
//!
pub use super::{ensures, modifies, modifies_slice, proof_for_contract, requires, stub_verified};
