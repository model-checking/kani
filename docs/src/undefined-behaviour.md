# Undefined Behaviour

## The Effect of Undefined Behaviour on Program Verification
Rust had a broad definition of [undefined behaviour](https://doc.rust-lang.org/reference/behavior-considered-undefined.html) (UB).
The [Rust documentation warns](https://doc.rust-lang.org/reference/behavior-considered-undefined.html) that UB can have unexpected, non-local effects:


> Note: Undefined behavior affects the entire program. For example, calling a function in C that exhibits undefined behavior of C means your entire program contains undefined behaviour that can also affect the Rust code. And vice versa, undefined behavior in Rust can cause adverse affects on code executed by any FFI calls to other languages.

If a program has UB, the semantics of the rest of the program are **undefined**.
If the program under verification does contain UB, then in theory the MIR evaluated by the verifier **has no semantics**, and hence could do anything, including violating the guarantees checked be a verifier. 
This means that verification results are subject to the proviso that the program under verification does not contain UB.

## What forms of Undefined Behaviour can Rust Exhibit

Rust’s [definition of UB](https://doc.rust-lang.org/reference/behavior-considered-undefined.html) is so broad that Rust has the following warning:

> **Warning**
> The following list is not exhaustive. There is no formal model of Rust's semantics for what is and is not allowed in unsafe code, so there may be more behavior considered unsafe. The following list is just what we know for sure is undefined behavior. Please read the Rustonomicon (https://doc.rust-lang.org/nomicon/index.html) before writing unsafe code.


Given the lack of a formal semantics for UB, and given Kani's focus on memory safety, there are classes of UB which Kani does not detect.
A non-exhaustive list of these, based on the the non-exhaustive list from the [Rust documentation](https://doc.rust-lang.org/reference/behavior-considered-undefined.html), is:

* Data races. 
    * Kani focuses on sequential code
* Breaking the pointer aliasing rules (http://llvm.org/docs/LangRef.html#pointer-aliasing-rules). 
    * Kani can detect if misuse of pointers causes memory safety or assertion violations, but does not not track reference lifetimes.
* Mutating immutable data.
    * Kani can detect if modification of immutable data causes memory safety or assertion violations, but does not not track reference lifetimes.
* Invoking undefined behavior via compiler intrinsics.
    * Kani makes a best effort attempt to check the preconditions of compiler intrinsics, but does not guarantee to do so in all cases
* Executing code compiled with platform features that the current platform does not support (see target_feature (https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute)).
    * Kani relies on the rustc compiler to check for this case
* Calling a function with the wrong call ABI or unwinding from a function with the wrong unwind ABI.
    * Kani relies on the rustc compiler to check for this case
* Producing an invalid value, even in private fields and locals. 
    * Kani provides a mechanism is_valid() which users can use to check validity of objects, but it does not currently apply to all types.
* Incorrect use of inline assembly.
    * Kani does not support inline assembly

Kani makes a best-effort attempt to detect some cases classes of UB:
* Evaluating a dereference expression (*expr) on a raw pointer that is dangling or unaligned
    * Kani can detect invalid dereferences, but may not detect them in [place expression context](https://doc.rust-lang.org/reference/expressions.html#place-expressions-and-value-expressions)
* Invoking undefined behavior via compiler intrinsics.
    * See [current support for Rust features](./rust-feature-support.md)
* Producing an invalid value, even in private fields and locals. 
    * Kani provides a mechanism is_valid() which users can use to check validity of objects, but it does not currently apply to all types.


