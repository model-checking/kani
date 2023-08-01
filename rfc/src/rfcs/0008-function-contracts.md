- **Feature Name:** Function Contracts
- **Feature Request Issue:** [#2652](https://github.com/model-checking/kani/issues/2652)
- **RFC PR:** [#2620](https://github.com/model-checking/kani/pull/2620)
- **Status:** Under Review 
- **Version:** 0
- **Proof-of-concept:** [features/contracts](https://github.com/model-checking/kani/tree/features/contracts)
- **Gate:** `-Zcontracts`, enforced by compile time error[^gate]

[^gate]: Enforced gates means all uses of constructs (functions, annotations,
    macros) in this RFC are an error.

-------------------

## Summary

Function contracts are a mechanism for a [stubbing]-like abstraction of concrete
implementations but with a significantly reduced threat to soundness[^simple-unsoundness].

Lays the ground work for modular verification.
<!-- Shorter? -->

[stubbing]: https://model-checking.github.io/kani/rfc/rfcs/0002-function-stubbing.html

## User Impact

<!-- Is basically the pitch and addressing the user. -->

Function contracts provide an interface for verified abstraction, a mechanism
similar to [stubbing], but sound[^simple-unsoundness]. This kind of verified
stubbing allows for modular verification, which paves the way for the
following two ambitious goals.

[^simple-unsoundness]: The main remaining threat to soundness in the use of
    contracts, as defined in this proposal, is the reliance on user-supplied
    harnesses for contract checking (explained in bulled 2 of [user
    experience](#user-experience)). A more thorough discussion on the dangers
    and potential remedies can be found in the [future](#future-possibilities)
    section.

- **Scalability:** Function contracts are sound (over)abstractions of function
  behavior. By verifiying the contract against its implemetation and
  subsequently performing caller verification against the (cheaper) abstraction
  verification can be modularized, cached and thus scaled.
- **Unbounded Verification:** Contracts can be reasoned over inductively and
  thus verify recursive functions.

Enabling function contracts is a non-invasive change. While it contains a new
API, that API is strictly additive for users. All prior operations are unchanged.

### Caveats

We add function contract functionality, but do not add the inductive reasoning
support needed for many unbounded problems, such as ["decreases"
measures](https://diffblue.github.io/cbmc/contracts-decreases.html) and
inductive lemmas (for instance in [ACSL](https://frama-c.com/download/acsl.pdf)
section 2.6.3 "inductive predicates").

## User Experience

Function contract provide a verifiable way to specify function behavior. In
addition the specified behavior can subsequently be used as an abstraction of
the functions behavior at call sites.

The lifecycle of a contract is split roughly into three phases. Which we will
explore on this simple example:

```rs
fn my_div(dividend: u32, divisor: u32) -> u32 {
  dividend / divisor
}
```

1. In the first phase we **specify** the approximation. Kani provides two new
   annotations: `requires` (preconditions) to describe the expectations this
   function has as to the calling context and `ensures` (postconditions) which
   approximates function outputs in terms of function inputs.

   ```rs
   #[kani::requires(divisor != 0)]
   #[kani::ensures(result <= dividend)]
   fn my_div(dividend: u32, divisor: u32) -> u32 {
     dividend / divisor
   }
   ```
  
   `requires` here indicates this function expects its `divisor` input to never
   be 0, or it will not execute correctly (i.e. panic).

   `ensures` puts a bound on the output, relative to the `dividend` input.

   Conditions in contracts are plain Rust expressions which can reference the
   function arguments and, in case of `ensures`, the result of the function as a
   special `result`[^result-naming] variable. Syntactically Kani supports any
   Rust expression, including function calls, defining types etc, however they
   must be side-effect free[^side-effects]. Multiple `requires` and `ensures`
   clauses are allowed on the same function, they are implicitly logically
   conjoined.

   [^result-naming]: See [open questions](#open-questions) for a discussion
       about naming of the result variable.

2. Next Kani must make sure that the approximation we specified actually holds
   by **checking** the contract against the implementation. This is in contrast
   to ["stubbing"][stubbing], where the approximation is blindly trusted.

   The contract must always overapproximate the function to guarantee soundness.

   To facilitate the check Kani needs a suitable environment to verify the
   function in. For this proposal the environment must be provided by us (the
   users). See [future possibilities](#future-possibilities) for a discussion
   about the arising soundness issues and their remedies.

   We provide the checking environment for our contract with a special new
   `proof_for_contract` harness.

   ```rs
   #[kani::proof_for_contract(my_div)]
   fn my_div_harness() {
     my_div(kani::any(), kani::any())
   }
   ```

   Similar to a verification harness for any other function, we are supposed to
   create all possible input combinations the function can encounter, then call
   the function at least once with those abstract inputs. If we forget to call
   `my_div` Kani reports an error.
   
   Unlike a unit-test we can however elide any checks of the output and
   post-call state. Instead Kani uses the conditions we specified in the
   contract as checks. Preconditions (`requires`) are inserted as `kani::assume`
   *before* the call to `my_div`, limiting the generic inputs to those the
   function is actually defined for. Postconditions (`ensures`) are inserted as
   `kani::assert` checks *after* the call to `my_div`.

   The expanded version of our harness and function is equivalent to the following:

   ```rs
   #[kani::proof]
   fn my_div_harness() {
     let dividend = kani::any();
     let divisor = kani::any();
     kani::assume(divisor != 0);
     let result = my_div(dividend, divisor);
     kani::assert(result <= dividend);
   }
   ```

   This expanded harness is then verified like any other harness but also gives
   the green light for the next step: verified stubbing.

3. In the last phase the **verified** contract is ready for us to use to
   **stub** other harnesses.

   Unlike in regular stubbing, there has to be at least one associated
   `proof_for_contract` harness for each function to stub *and* it requires all
   such harnesses to pass verification before attempting verification of any
   harnesses that use it as a stub.

   A possible harness that uses our `my_div` contract could be the following:

   ```rs
   #[kani::proof]
   #[kani::stub_verified(my_div)]
   fn use_div() {
     let v = vec![...];
     let some_idx = my_div(v.len() - 1, 3);
     v[some_idx];
   }
   ```

   To use the contract as a stub Kani must first ensure the calling context is
   safe. It inserts a `kani::assert` for the preconditions (`requires`) before
   the call; then it replaces the result of `my_div` with a non-deterministic
   value. Finally the non-deterministic result is constrained by a
   `kani::assume` of the postconditions (`ensures`).
   
   Mutable memory is similarly made non-deterministic, discussed later in
   [havociing](#memory-predicates-and-havocing).

   Stubbing `my_div` expands it similar to this:
  
   ```rs
   fn my_div_stub(dividend: u32, divisor: u32) -> u32 {
     kani::assert(divisor != 0); // pre-condition
     kani::any_where(|result| { /* post-condition */ result <= dividend })
   }
   ```

   Notice that this performs no actual computation for `my_div` (other than the
   conditions) which allows us to avoid something potentially costly.

Also notice that Kani was able to express both contract checking and stubbing
with existing capabilities; the important feature is the enforcement. The
checking is, by construction, performed **against the same condition** that is
later used as stub, which ensures soundness (see discussion on lingering threats
to soundness in the [future](#future-possibilities) section) and guarding against
stubs diverging from their checks.

### History Variables

Kani's contract language contains additional support for reasoning about changes
to memory. One case where this is necessary is whenever `ensures` needs to
reason about state before the function call. By default it only has access to
state after the call completes, which will be different if the call mutates
memory.

Consider the `Vec::pop` function

```rs
impl<T> Vec<T> {
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

If we want to describe in which case the result is `Some`, we need to know
whether `self` is empty *before* `pop` is called. To do this Kani provides the
`old(EXPR)` pseudo function, which evaluates `EXPR` before the call (e.g. to
`pop`) and makes the result available to `ensures`. It is used like so:

```rs
impl<T> Vec<T> {
  #[kani::ensures(old(self.is_empty()) || result.is_some())]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

`old` allows evaluating any (side-effect free[^side-effects]) Rust expression.
The borrow checker enforces the result of `old` cannot observe the mutations
from e.g. `pop`, as that would defeat the purpose. If `your` expression in `old`
returns borrowed content, make a copy instead (using e.g. `clone()`).

Note also that `old` is syntax, not a function and implemented as an extraction
and lifting during code generation. It can reference e.g. `pop`'s arguments but
not local variables. Compare the following

**Invalid ❌:** `#[kani::ensures({ let x = self.is_empty(); old(x) } || result.is_some())]`</br>
**Valid ✅:** `#[kani::ensures(old({ let x = self.is_empty(); x }) || result.is_some())]`

And it will only be recognized as `old(...)`, not as `let old1 = old; old1(...)` etc.

### Memory Predicates and Havocing

The last new feature added are predicates to refine a function's access to heap
memory. A memory footprint is used by the verifier to perform "havocing" during
contract stubbing. Recall that stubbing replaces the result value with a
non-deterministic `kani::any()`, havocing is the equivalent memory regions
touched by the function. Any memory regions in the footprint are "havoced" by
the verifier, that is replaced by a non-deterministic value (subject to type
constraints).

By default Kani infers a memory footprint as all memory reachable from a `&mut`
input or any `static` global referenced, directly or transitively, by the
function. While the inferred footprint is sound and enough for successful
contract checking[^inferred-footprint] it can easily turn large section of
memory to non-deterministic values, invalidate invariants of your program and
cause the verification to fail when the contract is used as a stub.

[^inferred-footprint]: While inferred memory footprints are sound for both safe
    and unsafe rust certain features in unsafe rust (e.g. `RefCell`) get
    inferred incorrectly and will lead to a failing contract check.

To reduce the scope of havocing Kani provides the `#[kani::assigns(CONDITION,
ASSIGN_RANGE...)]` and `#[kani::frees(CONDITION, LVALUE...)]` attributes. When
these attributes are provided Kani will only havoc the location mentioned in
`ASSIGN_RANGE` and `LVALUE` instead of the inferred footprint. Additionally Kani
verifies during checking that only the mentioned memory regions are touched and
only under the specified `CONDITION`. The `CONDITION` is optional and defaults
to `true`.

`LVALUE` are simple expressions permissible on the left hand side of an
assignment. They compose of the name of one function argument and zero or more
projections (dereference `*`, field access `.x`, slice indexing `[1]`).

The `ASSIGN_RANGE` permits any `LVALUE` but additionally permits more complex
slice expressions as the last projection that applies to pointer values. `[..]`
denotes the entirety of an allocation, `[i..]`, `[..j]` and `[i..j]` are
ranges of pointer offsets. A slicing syntax `p[i..j]` only applies if `p` is a
`*mut T` and points to an array allocation. The slice indices are offsets with
sizing `T`, e.g. in Rust `p[i..j]` would be equivalent to
`std::slice::from_raw_parts(p.offset(i), i - j)`. `i` must be smaller or equal
than `j`.

Because lvalues are restricted to using projections only, Kani must break
encapsulation here. If need be we can reference fields that are usually hidden,
without an error from the compiler.

[^side-effects]: Code used in contracts is required to be side effect free which
    means it must not perform I/O, mutate memory (`&mut` vars and such) or
    (de)allocate heap memory. This is enforced by the verifier, see the
    discussion in the [future](#future-possibilities) section.


### Detailed Attribute Contraints Overview

Any violation of the following constraints constitutes a compile-time error.

- A function may have any number of `requires`, `ensures`, `assigns` and `frees`
  attributes. Any function with at least one such annotation is considered as
  "having a contract".

  Harnesses (general or for contract checking) may not have any such annotation.

- A harness may have up to one `proof_for_contract(TARGET)` annotation where `TARGET` must
  "have a contract". One or more `proof_for_contract` harnesses may have the
  same `TARGET`. All such harnesses must pass verification, before `TARGET` may
  be used as a verified stub.

  A `proof_for_contract` harness may use any harness attributes, including
  `stub` and `stub_verified`, though the `TARGET` may not appear in either. 

-  Kani checks that `TARGET` is reachable from the `proof_for_contract` harness,
  but it does not warn if stubbed functions use `TARGET`[^stubcheck].

-  A `proof_for_contract` function may not have the `kani::proof` attribute (it
  is already implied by `proof_for_contract`).

- A harness may have multiple `stub_verified(TARGET)` attributes. Each `TARGET`
  must "have a contract". No `TARGET` may appear twice.

- Harnesses may combine `stub(S_TARGET, ..)` and `stub_verified(V_TARGET)`
  annotations, though no target may occur in `S_TARGET`s and `V_TARGET`s
  simultaneously.

- For mutually recursive functions using `stub_verified` kani will check their
  contracts in non-deterministic order and assume each time the respective other
  check succeeded.

[^stubcheck]: Kani cannot report the occurrence of a contract function to check
    in stubbed functions as errors, because the mechanism is needed to verify
    mutually recursive functions.

## Detailed Design

<!-- For the implementors or the hackers -->

Kani implements the functionality of function contracts in two places.

1. Code generation in the `requires` and `ensures` macros (`kani_macros`).
2. GOTO level contracts using CBMC's contract language generated in
   `kani-compiler` for handling memory predicates.

With some additional plumbing in the compiler and the driver.

### Code generation in `kani_macros`

The `requires` and `ensures` macros perform code generation in the macro,
creating a `check` and a `replace` function which use `assert` and `assume` as
described in the [user experience](#user-experience) section. Both are attached
to the function they are checking/replacing by  `kanitool::checked_with` and
`kanitool::replaced_with` attributes respectively. See also the
[discussion](#rationale-and-alternatives) about why we decided to generate check
and replace functions like this.

The code generation in the macros is straightforward, save two aspects: `old`
and the borrow checker.

The special `old` builtin function is implemented as an AST rewrite. Consider
the below example:

```rs
impl<T> Vec<T> {
  #[kani::ensures(self.is_empty() || self.len() == old(self.len()) - 1)]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

The `ensures` macro performs an AST rewrite constiting of an extraction of the
expressions in `old` and a replacement with a fresh local variable, creating the
following:

```rs
impl<T> Vec<T> {
  fn check_pop(&mut self) -> Option<T> {
    let old_1 = self.len();
    let result = Self::pop(self);
    kani::assert(self.is_empty() || self.len() == old_1 - 1)
  }
}
```

Nested invocations of `old` are prohibited (Kani throws an error) and the
expression inside may only refer to the function arguments and not other local
variables in the contract (Rust will report those variables as not being in
scope). 

The borrow checker also ensures for us that none of the temporary variables
borrow in a way that would be able to observe the moditication in `pop` which
would occur for instance if the user wrote `old(self)`. Instead of borrowing
copies should be created (e.g. `old(self.clone())`). This is only enforced for
safe rust though.

The second part relevant for the implementation is how we deal with the borrow
checker for postconditions. They reference the arguments of the function after
the call which is problematic if part of an input is borrowed mutably in the
return value. For instance the `Vec::split_at_mut` function does this and a
sensible contract for it might look as follows:

```rs
impl<T> Vec<T> {
  #[ensures(self.len() == result.0.len() + result.1.len())]
  fn split_at_mut(&mut self, i: usize) -> (&mut [T], &mut [T]) {
    ...
  }
}
```

This contract refers simultaneously to `self` and the result. Since the method
however borrows `self` mutably, it would no longer be accessible in the
postcondition. To work around this we strategically break the borrowing rules
using a new hidden builtin `kani::unckecked_deref` with the type signature `for
<T> fn (&T) -> T` which is essentially a C-style dereference operation. Breaking
the borrow checker like this is safe for 2 reasons:

1. Postconditions are not allowed perform mutation[^side-effects] and
2. Post conditions are of type `bool`, meaning they cannot leak references to
   the arguments and cause race conditions.

The "copies" of arguments created by by `unsafe_deref` are stored as fresh local
variables and their occurrence in the postcondition is renamed.

### Changes to Other Components

Contract enforcement and replacement (`kani::proof_for_contract(f)`,
`kani::stub_verified(f)`) both dispatch to the stubbing logic, stubbing `f` with
the generated check and replace function respectively. If `f` has no contract,
an error is thrown.

For memory predicates Kani relies on CBMC. Generated memory predicates (whether
derived from types of from explicit clauses) are emitted from the compiler as
GOTO contracts in the artifact. Then the driver invokes `goto-instrument` with
the name of the GOTO-level function names to enforce or replace the memory
contracts. The compiler communicates the names of the function via harness
metadata.

<!-- 
This is the technical portion of the RFC. Please provide high level details of the implementation you have in mind:

- What are the main components that will be modified? (E.g.: changes to `kani-compiler`, `kani-driver`, metadata,
  installation...)
- How will they be modified? Any changes to how these components communicate?
- Will this require any new dependency?
- What corner cases do you anticipate? 
-->

## Rationale and alternatives

<!-- For Developers -->
<!-- `old` discussion here -->

<!-- 
- What are the pros and cons of this design?
- What is the impact of not doing this?
- What other designs have you considered? Why didn't you choose them? 
-->


- **Kani-side implementation vs CBMC** Instead of generating check and replace
  functions in Kani, we could use the contract instrumentation provided by CBMC.
  We tried this earlier but came up short, because it is difficult to implement,
  while supporting arbitrary Rust syntax. We exported the conditions into
  functions so that Rust would do the parsing/type checking/lowering for us and
  then call the lowered function in the CBMC contract. The trouble is that
  CBMC's `old` is only supported directly in the contract, not in functions
  called from the contract. This means we either need to inline the contract
  function body, which is brittle in the presence if control flow, or we must
  extract the `old` expressions, evaluate them in the contract directly and pass
  the results to the check function. However this means we must restrict the
  expressions in `old`, because we now need to lower those by hand and even if
  we could let rustc do it, CBMC's old has no support for function calls in its
  argument expression.
- **Expanding all contract macros at the same time** Instead of expanding
  contract macros one-at-a-atime and creating the onion layer structure we could
  expand all subsequent one's with the outermost one, creating only one check
  and replace function each. This is however brittle with respect to renaming.
  If a user does `use kani::requires as my_requires` and then does multiple
  `#[my_requires(condition)]` macro would not collect them properly since it can
  only mathc syntactically and it does not know about the `use` and neither can
  we restrict this kind if use or warn the user. By contrast the collection with
  `kanitool::checked_with` is safe, because that attribute is generated by our
  macro itself, so we can rely on the fact that it uses then canonical
  representation.
- **Generating nested functions instead of siblings** Instead of generating the
  `check` and `replace` functions as siblings to the contracted function we
  could nest them like so

  ```rs
  fn my_div(dividend: u32, divisor: u32) -> u32 {
    fn my_div_check_5e3713(dividend: u32, divisor: u32) -> u32 {
      ...
    }
    ...
  }
  ```

  This could be beneficial if we want to be able to allow contracts on trait
  impl items, in which case generating sibling functions is not allowed. The
  only thing required to make this work is an additional pass over the condition
  that replaces every `self` with a fresh identifier that now becomes the first
  argument of the function.
- **Explicit command line checking/substitution vs attributes**

## Open questions

<!-- For Developers -->

- Is it really correct to return `kani::any()` from the replacement copy, even
  if it can be a pointer?
- Our handling of `impl` in `reuqires` and `ensures` macros is brittle, though
  probably can't be improved. If the contracted function is an `impl` item, then
  the call to the next onion layer has to be `Self::<next fn>()` instead of
  `<next fn>()`. However we have no reliable way of knowing when we are in an
  `impl` fn. The macro uses a heuristic (is `self` or `Self` present) but in
  theory a user can declare an `impl` fn that never uses either `Self` or `self`
  in which case we generate broken code that throws cryptic error messages.
- Making result special. Should we use special syntax here like `@result` or
  `kani::result()`, though with the latter I worry that people may get confused
  because it is syntactic and not subject to usual `use` renaming and import
  semantics. Alternatively we can let the user pick the name with an additional
  argument to `ensures`, e.g. `ensures(my_result_var, CONDITION)`

  See [#2597](https://github.com/model-checking/kani/issues/2597)

 
<!-- 
- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design? 
-->

## Future possibilities

<!-- For Developers -->

- **Quantifiers:** Quantifiers are like logic-level loops and a powerful
  reasoning helper. CBMC has support for both `exists` and `forall`, but the
  code generation is difficult. The most ergonomic and easy way to implement
  quantifiers on the Rust side is as higher-order functions taking `Fn(T) ->
  bool`, where `T` is some arbitrary type that can be quantified over. This
  interface is familiar to developers, but the code generation is tricky, as
  CBMC level quantifiers only allow certain kinds of expressions. This
  necessiates a rewrite of the `Fn` closure to a compliant expression.
- **Side effect** freedom is currently enforced by CBMC. This means that the error
  originates there and is likely not legible. Intead Kani should perform a
  reachability analysis from the contract expressions and determine whether side
  effects are possible, throwing a graceful error.
- Letting the user supply the **harnesses for checking contracts** is a source of
  unsoundness, if corner cases are not adequately covered. Ideally Kani would
  generate the check harness automatically, but this is difficult both because
  heap datastructures are potentially infinite, and also because it must observe
  user-level invariants.
- What about mutable trait inputs (wrt memory access patters), e.g. a `mut impl AccessMe`
- **Trait contracts:** Ous proposal could be extended easily to handle simple
  trait contracts. The macros would generate new trait methods with default
  implementation, similar to the functions it generates today. Using sealed
  types we can prevent the user from overwriting the generated contract methods.
  Contracts for the trait and contracts on it's impls are combined by stubbing
  the original method depending on context. The occurrence inside the contract
  generated from the trait method is replaced by the impl contract. Any other
  occurrence is replaced by the just altered trait method contract.
- **Cross Session Verification Caching:** This proposal focuses on scalability
  benefits within a single verification session, but those verification results
  could be cached across sessions and speed up verification for large projects
  using contacts in the future.
- **Inductive Reasoning:** Describing recursive functions can require that the
  contract also recurse, describing a fixpoint logic. This is needed for
  instance for linked data structures like linked lists or trees.
- **Compositional Contracts:** The proposal in this document lacks a
  comprehensive handling of type parameters. Correctness of a function with a
  constrained type parameter (e.g. `T : Trait`) depends on `T` observing a
  contract as well. This is most striking in the case of higher order functions.