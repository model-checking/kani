- **Feature Name:** Function Contracts
- **Feature Request Issue:** [#2652](https://github.com/model-checking/kani/issues/2652) and [Milestone](https://github.com/model-checking/kani/milestone/31)
- **RFC PR:** [#2620](https://github.com/model-checking/kani/pull/2620)
- **Status:** Unstable
- **Version:** 1
- **Proof-of-concept:** [features/contracts](https://github.com/model-checking/kani/tree/features/contracts)
- **Feature Gate:** `-Zfunction-contracts`, enforced by compile time error[^gate]

-------------------

## Summary

Function contracts are a means to specify and check function behavior. On top of
that the specification can then be used as a sound[^simple-unsoundness]
abstraction to replace the concrete implementation, similar to [stubbing].


This allows for a modular verification.
<!-- Shorter? -->

[stubbing]: https://model-checking.github.io/kani/rfc/rfcs/0002-function-stubbing.html

## User Impact

<!-- Is basically the pitch and addressing the user. -->

Function contracts provide an interface for a verified,
sound[^simple-unsoundness] function abstraction. This is similar to [stubbing]
but with verification of the abstraction instead of blind trust. This allows for
modular verification, which paves the way for the following two ambitious goals.

- **Scalability:** A function contract is an abstraction (sound
  overapproximation) of a function's behavior. After verifying the contract
  against its implementation we can subsequently use the (cheaper) abstraction
  instead of the concrete implementation when analyzing its callers.
  Verification is thus modularized and even cacheable.
- **Unbounded Verification:** Contracts enable inductive reasoning for recursive
  functions where the first call is checked against the contract and recursive
  calls are stubbed out using the abstraction.

Function contracts are completely optional with no user impact if unused. This
RFC proposes the addition of new attributes, and functions, that shouldn't
interfere with existing functionalities.  


## User Experience

A function contract specifies the behavior of a function as a predicate that
can be checked against the function implementation and also used as an
abstraction of the implementation at the call sites.

The lifecycle of a contract is split into three phases: specification,
verification and call abstraction, which we will explore on this example:

```rs
fn my_div(dividend: u32, divisor: u32) -> u32 {
  dividend / divisor
}
```

1. In the first phase we **specify** the contract. Kani provides two new
   annotations: `requires` (preconditions) to describe the expectations this
   function has as to the calling context and `ensures` (postconditions) which
   approximates function outputs in terms of function inputs.

   ```rs
   #[kani::requires(divisor != 0)]
   #[kani::ensures(|result : &u32| *result <= dividend)]
   fn my_div(dividend: u32, divisor: u32) -> u32 {
     dividend / divisor
   }
   ```
  
   `requires` here indicates this function expects its `divisor` input to never
   be 0, or it will not execute correctly (for instance panic or cause undefined
   behavior).

   `ensures` puts a bound on the output, relative to the `dividend` input.

   Conditions in contracts are Rust expressions which reference the
   function arguments and, in case of `ensures`, the return value of the
   function. The return value is passed into the ensures closure statement by reference. Syntactically
   Kani supports any Rust expression, including function calls, defining types
   etc. However they must be side-effect free (see also side effects
   [here](#changes-to-other-components)) or Kani will throw a compile error.

   Multiple `requires` and `ensures` clauses are allowed on the same function,
   they are implicitly logically conjoined.


2. Next, Kani ensures that the function implementation respects all the conditions specified in its contract.

   To perform this check Kani needs a suitable harness to verify the function
   in. The harness is mainly responsible for providing the function arguments
   but also set up a valid heap that pointers may refer to and properly
   initialize `static` variables.
   
   Kani demands of us, as the user, to provide this harness; a limitation of
   this proposal. See also [future possibilities](#future-possibilities) for a
   discussion about the arising soundness issues and their remedies.

   Harnesses for checking contract are defined with the
   `proof_for_contract(TARGET)` attribute which references `TARGET`, the
   function for which the contract is supposed to be checked.

   ```rs
   #[kani::proof_for_contract(my_div)]
   fn my_div_harness() {
     my_div(kani::any(), kani::any())
   }
   ```

   Similar to a verification harness for any other function, we are supposed to
   create all possible input combinations the function can encounter, then call
   the function at least once with those abstract inputs. If we forget to call
   `my_div` Kani reports an error. Unlike other harnesses we only need to create
   suitable data structures but we don't need to add any checks as Kani will
   use the conditions we specified in the contract. 
   
   Kani inserts preconditions (`requires`) as `kani::assume` *before* the call
   to `my_div`, limiting inputs to those the function is actually defined for.
   It inserts postconditions (`ensures`) as `kani::assert` checks *after* the
   call to `my_div`, enforcing the contract.

   The expanded version of our harness that Kani generates looks roughly like
   this:

   ```rs
   #[kani::proof]
   fn my_div_harness() {
     let dividend = kani::any();
     let divisor = kani::any();
     kani::assume(divisor != 0); // requires
     let result_kani_internal = my_div(dividend, divisor);
     kani::assert((|result : &u32| *result <= dividend)(result_kani_internal)); // ensures
   }
   ```

   Kani verifies the expanded harness like any other harness, giving the
   green light for the next step: call abstraction.

3. In the last phase the **verified** contract is ready for us to use to
   abstract the function at its call sites.

   Kani requires that there has to be at least one associated
   `proof_for_contract` harness for each abstracted function, otherwise an error is
   thrown. In addition, by default, it requires all `proof_for_contract`
   harnesses to pass verification before attempting verification of any
   harnesses that use the contract as a stub.

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

   At a call site where the contract is used as an abstraction Kani
   `kani::assert`s the preconditions (`requires`) and produces a
   nondeterministic value (`kani::any`) which satisfies the postconditions.
   
   Mutable memory is similarly made non-deterministic, discussed later in
   [havocking](#memory-predicates-and-havocking).

   An expanded stubbing of `my_div` looks like this:
  
   ```rs
   fn my_div_stub(dividend: u32, divisor: u32) -> u32 {
     kani::assert(divisor != 0); // pre-condition
     kani::any_where(|result| { /* post-condition */ result <= dividend })
   }
   ```

   Notice that this performs no actual computation for `my_div` (other than the
   conditions) which allows us to avoid something potentially costly.

Also notice that Kani was able to express both contract checking and abstracting
with existing capabilities; the important feature is the enforcement. The
checking is, by construction, performed **against the same condition** that is
later used as the abstraction, which ensures soundness (see discussion on
lingering threats to soundness in the [future](#future-possibilities) section)
and guarding against abstractions diverging from their checks.

### Write Sets and Havocking

Functions can have side effects on data reachable through mutable references or
pointers. To overapproximate all such modifications a function could apply to
pointed-to data, the verifier "havocs" those regions, essentially replacing
their content with non-deterministic values.

Let us consider a simple example of a `pop` method.

```rs
impl<T> Vec<T> {
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

This function can, in theory, modify any memory behind `&mut self`, so this is
what Kani will assume it does by default. It infers the "write set", that is the
set of memory locations a function may modify, from the type of the function
arguments. As a result, any data pointed to by a mutable reference or pointer is
considered part of the write set[^write-set-recursion]. In addition, a static
analysis of the source code discovers any `static mut` variables the function or
it's dependencies reference and adds all pointed-to data to the write set also.

During havocking the verifier replaces all locations in the write set with
non-deterministic values. Kani emits a set of automatically generated
postconditions which encode the expectations from the Rust type system and
`assume`s them for the havocked locations to ensure they are valid. This
encompasses both limits as to what values are acceptable for a given type, such
as `char` or the possible values of an enum discriminator, as well as lifetime
constraints.

While the inferred write set is sound and enough for successful contract
checking[^inferred-footprint] in many cases this inference is too coarse
grained. In the case of `pop` every value in this vector will be made
non-deterministic.

To address this the proposal also adds a `modifies` and `frees` clause which
limits the scope of havocking. Both clauses represent an assertion that the
function will modify only the specified memory regions. Similar to
requires/ensures the verifier enforces the assertion in the checking stage to
ensure soundness. When the contract is used as an abstraction, the `modifies`
clause is used as the write set to havoc.

In our `pop` example the only modified memory location is the last element and
only if the vector was not already empty, which would be specified thusly.

```rs
impl<T> Vec<T> {
  #[modifies(if !self.is_empty() => (*self).buf.ptr.pointer.pointer[self.len])]
  #[modifies(if self.is_empty())]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

The `#[modifies(when = CONDITION, targets = { MODIFIES_RANGE, ... })]` consists
of a `CONDITION` and zero or more, comma separated `MODIFIES_RANGE`s which are
essentially a place expression.

Place expressions describe a position in the abstract program memory. You may
think of it as what goes to the left of an assignment. They compose of the name
of one function argument (or static variable) and zero or more projections
(dereference `*`, field access `.x` and slice indexing `[1]`[^slice-exprs]).

If no `when` is provided the condition defaults to `true`, meaning the modifies
ranges apply to all invocations of the function. If `targets` is omitted it
defaults to `{}`, e.g. an empty set of targets meaning under this condition the
function modifies no mutable memory.

Because place expressions are restricted to using projections only, Kani must
break Rusts `pub`/no-`pub` encapsulation here[^assigns-encapsulation-breaking].
If need be we can reference fields that are usually hidden, without an error
from the compiler.

In addition to a place expression, a `MODIFIES_RANGE` can also be terminated
with more complex *slice* expressions as the last projection. This only applies
to `*mut` pointers to arrays. For instance this is needed for `Vec::truncate`
where all of the latter section of the allocation is assigned (dropped).

```rs
impl<T> Vec<T> {
  #[modifies(self.buf.ptr.pointer.pointer[len..])]
  fn truncate(&mut self, len: usize) {
    ...
  }
}
```

`[..]` denotes the entirety of an allocation, `[i..]`, `[..j]` and `[i..j]` are
ranges of pointer offsets[^slice-exprs]. The slice indices are offsets with sizing `T`, e.g.
in Rust `p[i..j]` would be equivalent to
`std::slice::from_raw_parts(p.offset(i), i - j)`. `i` must be smaller or equal
than `j`.

A `#[frees(when = CONDITION, targets = { PLACE, ... })]` clause works similarly
to `modifies` but denotes memory that is deallocated. Like `modifies` it applies
only to pointers but unlike modifies it does not admit slice syntax, only
place expressions, because the whole allocation has to be freed.

### History Expressions

Kani's contract language contains additional support to reason about changes of
mutable memory. One case where this is necessary is whenever `ensures` needs to
refer to state before the function call. By default variables in the ensures
clause are interpreted in the post-call state whereas history expressions are
interpreted in the pre-call state.

Returning to our `pop` function from before we may wish to describe in which
case the result is `Some`. However that depends on whether `self` is empty
*before* `pop` is called. To do this Kani provides the `old(EXPR)` pseudo
function (see [this section](#open-questions) about a discussion on naming),
which evaluates `EXPR` before the call (e.g. to `pop`) and makes the result
available to `ensures`. It is used like so:

```rs
impl<T> Vec<T> {
  #[kani::ensures(|result : &Option<T>| old(self.is_empty()) || result.is_some())]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

`old` allows evaluating any Rust expression in the pre-call context, so long as
it is free of side-effects. See also [this
explanation](#changes-to-other-components). The borrow checker enforces that the
mutations performed by e.g. `pop` cannot be observed by the history expression, as
that would defeat the purpose. If you wish to return borrowed content from
`old`, make a copy instead (using e.g. `clone()`).

Note also that `old` is syntax, not a function and implemented as an extraction
and lifting during code generation. It can reference e.g. `pop`'s arguments but
not local variables. Compare the following

**Invalid ❌:** `#[kani::ensures(|result : &Option<T>| { let x = self.is_empty(); old(x) } || result.is_some())]`</br>
**Valid ✅:** `#[kani::ensures(|result : &Option<T>| old({ let x = self.is_empty(); x }) || result.is_some())]`

And it will only be recognized as `old(...)`, not as `let old1 = old; old1(...)` etc.

### Workflow and Attribute Constraints Overview

1. By default `kani` or `cargo kani` first verifies all contract harnesses
   (`proof_for_contract`) reachable from the file or in the local workspace
   respectively.
2. Each contract (from the local
   crate[^external-contract-checking-expectations]) that is used in a
   `stub_verified` is required to have at least one associated contract harness.
   Kani reports any missing contract harnesses as errors.
3. Kani verifies all regular harnesses *if* their `stub_verified` contracts
   passed step 1 and 2.

When specific harnesses are selected (with `--harness`) contracts are not
verified.

Kani reports a compile time error if any of the following constraints are violated:

- A function may have any number of `requires`, `ensures`, `modifies` and `frees`
  attributes. Any function with at least one such annotation is considered as
  "having a contract".

  Harnesses (general or for contract checking) may not have any such annotation.

- A harness may have up to one `proof_for_contract(TARGET)` annotation where `TARGET` must
  "have a contract". One or more `proof_for_contract` harnesses may have the
  same `TARGET`. 

  A `proof_for_contract` harness may use any harness attributes, including
  `stub` and `stub_verified`, though the `TARGET` may not appear in either. 

-  Kani checks that `TARGET` is reachable from the `proof_for_contract` harness,
  but it does not warn if abstracted functions use `TARGET`[^stubcheck].

-  A `proof_for_contract` function may not have the `kani::proof` attribute (it
  is already implied by `proof_for_contract`).

- A harness may have multiple `stub_verified(TARGET)` attributes. Each `TARGET`
  must "have a contract". No `TARGET` may appear twice. Each local `TARGET` is
  expected to have at least one associated `proof_for_contract` harness which
  passes verification, see also the discussion on when to check contracts in
  [open questions](#open-questions).

- Harnesses may combine `stub(S_TARGET, ..)` and `stub_verified(V_TARGET)`
  annotations, though no target may occur in `S_TARGET`s and `V_TARGET`s
  simultaneously.

- For mutually recursive functions using `stub_verified`, Kani will check their
  contracts in non-deterministic order and assume each time the respective other
  check succeeded.

## Detailed Design

<!-- For the implementors or the hackers -->

Kani implements the functionality of function contracts in three places.

1. Code generation in the `requires` and `ensures` macros (`kani_macros`).
2. GOTO level contracts using CBMC's contract language generated in
   `kani-compiler` for `modifies` clauses.
3. Dependencies and ordering among harnesses in `kani-driver` to enforce
   contract checking before replacement. Also plumbing between compiler and
   driver for enforcement of assigns clauses.

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
  #[kani::ensures(|result : &Option<T>| self.is_empty() || self.len() == old(self.len()) - 1)]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

The `ensures` macro performs an AST rewrite consisting of an extraction of the
expressions in `old` and a replacement with a fresh local variable, creating the
following:

```rs
impl<T> Vec<T> {
  fn check_pop(&mut self) -> Option<T> {
    let old_1 = self.len();
    let result_kani_internal = Self::pop(self);
    kani::assert((|result : &Option<T>| self.is_empty() || self.len() == old_1 - 1)(result_kani_internal))
  }
}
```

Nested invocations of `old` are prohibited (Kani throws an error) and the
expression inside may only refer to the function arguments and not other local
variables in the contract (Rust will report those variables as not being in
scope). 

The borrow checker also ensures for us that none of the temporary variables
borrow in a way that would be able to observe the modification in `pop` which
would occur for instance if the user wrote `old(self)`. Instead of borrowing
copies should be created (e.g. `old(self.clone())`). This is only enforced for
safe Rust though.

The second part relevant for the implementation is how we deal with the borrow
checker for postconditions. They reference the arguments of the function after
the call which is problematic if part of an input is borrowed mutably in the
return value. For instance the `Vec::split_at_mut` function does this and a
sensible contract for it might look as follows:

```rs
impl<T> Vec<T> {
  #[ensures(|result : &(&mut [T], &mut [T])| self.len() == result.0.len() + result.1.len())]
  fn split_at_mut(&mut self, i: usize) -> (&mut [T], &mut [T]) {
    ...
  }
}
```

This contract refers simultaneously to `self` and the result. Since the method
however borrows `self` mutably, it would no longer be accessible in the
postcondition. To work around this we strategically break the borrowing rules
using a new hidden builtin `kani::unchecked_deref` with the type signature `for
<T> fn (&T) -> T` which is essentially a C-style dereference operation. Breaking
the borrow checker like this is safe for 2 reasons:

1. Postconditions are not allowed perform mutation and
2. Post conditions are of type `bool`, meaning they cannot leak references to
   the arguments and cause the race conditions the Rust type system tries to
   prevent.

The "copies" of arguments created by `unsafe_deref` are stored as fresh local
variables and their occurrence in the postcondition is renamed. In addition a
`mem::forget` is emitted for each copy to avoid a double free.

### Recursion

Kani verifies contracts for recursive functions inductively. Reentry of the
function is detected with a function-specific static variable. Upon detecting
reentry we use the replacement of the contract instead of the function body.

Kani generates an additional wrapper around the function to add the detection.
The additional wrapper is there so we can place the `modifies` contract on
`check_pop` and `replace_pop` instead of `recursion_wrapper` which prevents CBMC
from triggering its recursion induction as this would skip our replacement checks.

```rs
#[checked_with = "recursion_wrapper"]
#[replaced_with = "replace_pop"]
fn pop(&mut self) { ... }

fn check_pop(&mut self) { ... }

fn replace_pop(&mut self) { ... }

fn recursion_wrapper(&mut self) { 
  static mut IS_ENTERED: bool = false;

  if unsafe { IS_ENTERED } {
    replace_pop(self)
  } else {
    unsafe { IS_ENTERED = true; }
    let result = check_pop(self);
    unsafe { IS_ENTERED = false; }
    result
  };
}
```

Note that this is insufficient to verify all types of recursive functions, as
the contract specification language has no support for inductive lemmas (for
instance in [ACSL](https://frama-c.com/download/acsl.pdf) section 2.6.3
"inductive predicates"). Inductive lemmas are usually needed for recursive
data structures.

### Changes to Other Components

Contract enforcement and replacement (`kani::proof_for_contract(f)`,
`kani::stub_verified(f)`) both dispatch to the **stubbing logic**, stubbing `f`
with the generated check and replace function respectively. If `f` has no
contract, Kani throws an error.

For **write sets** Kani relies on CBMC. `modifies` clauses (whether derived from
types or from explicit clauses) are emitted from the compiler as GOTO contracts
in the artifact. Then the driver invokes `goto-instrument` with the name of the
GOTO-level function names to enforce or replace the memory contracts. The
compiler communicates the names of the function via harness metadata.


Code used in contracts is required to be **side effect** free which means it
must not perform I/O, mutate memory (`&mut` vars and such) or (de)allocate heap
memory. This is enforced in two layers. First with an MIR traversal over all
code reachable from a contract expression. An error is thrown if known
side-effecting actions are performed such as `ptr::write`, `malloc`, `free` or
functions which we cannot check, such as e.g. `extern "C"`, with the exception
of known side effect free functions in e.g. the standard library.

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


### Kani-side implementation vs CBMC 

Instead of generating check and replace functions in Kani, we could use the contract instrumentation provided by CBMC.

We tried this earlier but came up short, because it is difficult to implement,
while supporting arbitrary Rust syntax. We exported the conditions into
functions so that Rust would do the parsing/type checking/lowering for us and
then call the lowered function in the CBMC contract. 

The trouble is that CBMC's `old` is only supported directly in the contract, not
in functions called from the contract. This means we either need to inline the
contract function body, which is brittle in the presence of control flow, or we
must extract the `old` expressions, evaluate them in the contract directly and
pass the results to the check function. However this means we must restrict the
expressions in `old`, because we now need to lower those by hand and even if we
could let `rustc` do it, CBMC's `old` has no support for function calls in its
argument expression.

### Expanding all contract macros at the same time 

Instead of expanding contract macros one-at-a-time and layering the checks we
could expand all subsequent one's with the outermost one in one go.

This is however brittle with respect to renaming. If a user does `use
kani::requires as my_requires` and then does multiple
`#[my_requires(condition)]` macro would not collect them properly since it can
only match syntactically and it does not know about the `use` and neither can we
restrict this kind of use or warn the user. By contrast, the collection with
`kanitool::checked_with` is safe, because that attribute is generated by our
macro itself, so we can rely on the fact that it uses the canonical
representation.

### Generating nested functions instead of siblings 

Instead of generating the `check` and `replace` functions as siblings to the
contracted function we could nest them like so

```rs
fn my_div(dividend: u32, divisor: u32) -> u32 {
  fn my_div_check_5e3713(dividend: u32, divisor: u32) -> u32 {
    ...
  }
  ...
}
```

This could be beneficial if we want to be able to allow contracts on trait impl
items, in which case generating sibling functions is not allowed. On the other
hand this makes it harder to implement contracts on *trait definitions*,
because there is no body available which we could nest the function into.
Ultimately we may require both so that we can support both.


What is required to make this work is an additional pass over the condition that
replaces every `self` with a fresh identifier that now becomes the first
argument of the function. In addition there are open questions as to how to
resolve the nested name inside the compiler.

### Explicit command line checking/substitution vs attributes: 

Instead of
  adding a new special `proof_for_contact` attributes we could have instead done:

  1. **Check contracts on the command line** like CBMC does. This makes contract
     checking a separate `kani` invocation with something like a
     `--check-contract` flag that directs the system to instrument the function.
     This is a very flexible design, but also easily used incorrectly.
     Specifically nothing in the source indicates which harnesses are supposed
     to be used for which contract, users must remember to invoke the check and
     are also responsible for ensuring they really do verify *all* contacts they
     will later be replacing and lastly.
  2. **Check contracts with a `#[kani::proof]` harness.** This would have used
     e.g. a `#[kani::for_contract]` attributes on a `#[kani::proof]`. Since
     `#[kani::for_contract]` is *only* valid on a proof, we decided to just
     imply it and save the user some headache. Contract checking harnesses are
     not meant to be reused for other purposes anyway and if the user *really*
     wants to the can just factor out the actual contents of the harness to
     reuse it.

### Polymorphism during contract checking

A current limitation with how contracts are enforced means that if the target of
a `proof_for_contract` is polymorphic, only one monomorphization is permitted to
occur in the harness. This does not limit the target to a single occurrence,
*but* to a single instantiation of its generic parameters.

This is because we rely on CBMC for enforcing the `modifies` contract. At the
GOTO level all monomorphized instances are distinct functions *and* CBMC only
allows checking one function contract at a time, hence this restriction.

### User supplied harnesses

We make the user supply the harnesses for checking contracts. This is our major
source of unsoundness, if corner cases are not adequately covered. Having Kani
generate the harnesses automatically is a non-trivial task (because heaps are
hard) and will be the subject of [future improvements](#future-possibilities). 

In limited cases we could generate harnesses, for instance if only bounded types
(integers, booleans, enums, tuples, structs, references and their combinations)
were used. We could restrict the use of contracts to cases where only such types
are involved in the function inputs and outputs, however this would drastically
limit the applicability, as even simple heap data structures such as `Vec`,
`String` and even `&[T]` and `&str` (slices) would be out of scope. These data
structures however are ubiquitous and users can avoid the unsoundness with
relative confidence by overprovisioning (generating inputs that are several
times larger than what they expect the function will touch).


## Open questions

<!-- For Developers -->

- Returning **`kani::any()` in a replacement isn't great**, because it wouldn't work
  for references as they can't have an `Arbitrary` implementation. Plus the
  soundness then relies on a correct implementation of `Arbitrary`. Instead it
  may be better to allow for the user to specify type invariants which can the
  be used to generate correct values in replacement but also be checked as part
  of the contract checking.
- Making result special. Should we use special syntax here like `@result` or
  `kani::result()`, though with the latter I worry that people may get confused
  because it is syntactic and not subject to usual `use` renaming and import
  semantics. Alternatively we can let the user pick the name with an additional
  argument to `ensures`, e.g. `ensures(my_result_var, CONDITION)`

  Similar concerns apply to `old`, which may be more appropriate to be special
  syntax, e.g. `@old`.

  See [#2597](https://github.com/model-checking/kani/issues/2597)
- How to **check the right contracts at the right time**. By default `kani` and
  `cargo kani` check all contracts in a crate/workspace. This represents the
  safest option for the user but may be too costly in some cases.

  The user should be provided with options to disable contract checking for the
  sake of efficiency. Such options may look like this:

  - **By default** (`kani`/`cargo kani`) all local contracts are checked,
    harnesses are only checked if the contracts they depend on succeeded their check.
  - **With harness selection** (`--harness`) only those contracts which the
    selected harnesses depend on are checked.
  - **For high assurance** passing a `--paranoid` flag also checks contracts for
    dependencies (other crates) when they are used in abstractions.
  - **Per harness** the users can disable the checking for specific contracts
    via attribute, like `#[stub_verified(TARGET, trusted)]` or
    `#[stub_unverified(TARGET)]`. This also plays nicely with `cfg_attr`.
  - **On the command line** users can similarly disable contract checks by
    passing (multiple times) `--trusted TARGET` to skip checking those
    contracts.
  - **The bold** (or naïve) user can skip all contracts with `--all-trusted`.
  - **For the lawyer** that is only interested in checking contracts and nothing
    else a `--litigate` flag checks only contract harnesses.

  Aside: I'm obviously having some fun here with the names, happy to change,
  it's really just about the semantics.
- **Can `old` accidentally break scope?** The `old` function cannot reference local
  variables. For instance `#[ensures({let x = ...; old(x)})]` cannot work as an
  AST rewrite because the expression in `old` is lifted out of it's context into
  one where the only bound variables are the function arguments (see also
  [history expressions](#history-expressions)). In most cases this will be a
  compiler error complaining that `x` is unbound, however it is possible that
  *if* there is also a function argument `x`, then it may silently succeed the
  code generation but confusingly fail verification. For instance `#[ensures({
  let x = 1; old(x) == x })]` on a function that has an argument named `x` would
  *not* hold.

  To handle this correctly we would need an extra check that detects if `old`
  references local variables. That would also enable us to provide a better
  error message than the default "cannot find value `x` in this scope".
- **Can panicking be expected behavior?** Usually preconditions are used to rule
  out panics but it is conceivable that a user would want to specify that a
  function panics under certain conditions. Specifying this would require an
  extension to the current interface.
- **UB checking.** With unsafe rust it is possible to break the type system
  guarantees in Rust without causing immediate errors. Contracts must be
  cognizant of this and enforce the guarantees as part of the contract *or*
  require users to explicitly defer such checks to use sites. The latter case
  requires dedicated support because the potential UB must be reflected in the
  havoc.
- **`modifies` clauses over patterns.** Modifies clauses mention values bound in
  the function header and as a user I would expect that if I use a pattern in
  the function header then I can use the names bound in that pattern as base
  variables in the `modifies` clause. However `modifies` clauses are implemented
  as `assigns` clauses in CBMC which does not have a notion of function header
  patterns. Thus it is necessary to project any `modifies` ranges deeper by the
  fields used in the matched pattern.

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
  necessitates a rewrite of the `Fn` closure to a compliant expression.
- Letting the user supply the **harnesses for checking contracts** is a source of
  unsoundness, if corner cases are not adequately covered. Ideally Kani would
  generate the check harness automatically, but this is difficult both because
  heap data structures are potentially infinite, and also because it must observe
  user-level type invariants.

  A complete solution for this is not known to us but there are ongoing
  investigations into harness generation mechanisms in CBMC.

  Function inputs that are non-inductive could be created from the type as the
  safe Rust type constraints describe a finite space.

  For dealing with pointers one applicable mechanism could be *memory
  predicates* to declaratively describe the state of the heap both before and
  after the function call. 
  
  In CBMC's implementation memory predicates are part of the pre/postconditions.
  This does not easily translate to Kani, since we handle pre/postconditions
  manually and mainly in proc-macros. There are multiple ways to bridge this
  gap, perhaps the easiest being to add memory predicates *separately* to Kani
  instead of as part of pre/postconditions, so they can be handled by forwarding
  them to CBMC. However this is also tricky, because memory predicates are used
  to describe pointers and pointers only. Meaning that if they are encapsulated
  in a structure (such as `Vec` or `RefCell`) there is no way of specifying the
  target of the predicate without breaking encapsulation (similar to
  `modifies`). In addition there are limitations also on the pointer predicates
  in CBMC itself. For instance they cannot be combined with quantifiers.
  
  A better solution would be for the data structure to declare its own
  invariants at definition site which are automatically swapped in on every
  contract that uses this type.
- What about mutable trait inputs (wrt memory access patters), e.g. a `mut impl AccessMe`
- **Trait contracts:** Our proposal could be extended easily to handle simple
  trait contracts. The macros would generate new trait methods with default
  implementations, similar to the functions it generates today. Using sealed
  types we can prevent the user from overwriting the generated contract methods.
  Contracts for the trait and contracts on it's `impl`s are combined by abstracting
  the original method depending on context. The occurrence inside the contract
  generated from the trait method is replaced by the `impl` contract. Any other
  occurrence is replaced by the just altered trait method contract.
- **Cross Session Verification Caching:** This proposal focuses on scalability
  benefits within a single verification session, but those verification results
  could be cached across sessions and speed up verification for large projects
  using contacts in the future.
- **Inductive Reasoning:** Describing recursive functions can require that the
  contract also recurse, describing a fixpoint logic. This is needed for
  instance for linked data structures like linked lists or trees. Consider for
  instance a reachability predicate for a linked list:

  ```rs
  struct LL<T> { head: T, next: *const LL<T> }

  fn reachable(list: &LL<T>, t: &T) -> bool {
      list.head == t
      || unsafe { next.as_ref() }.map_or(false, |p| reachable(p, t))
  }

  ```
- **Compositional Contracts:** The proposal in this document lacks a
  comprehensive handling of type parameters. Contract checking harnesses require
  monomorphization. However this means the contract is only checked against a
  finite number of instantiations of any type parameter (at most as many as
  contract checking harnesses were defined). There is nothing preventing the
  user from using different instantiations of the function's type parameters.

  A function (`f()`) can only interact with its type parameters `P` through the
  traits (`T`) they are constrained over. We can require `T` to carry contracts
  on each method `T::m()`. During checking we can use a synthetic type that
  abstracts `T::m()` with its contract. This way we check `f()` against `T`s
  contract. Then we later abstract `f()` we can ensure any instantiations of `P`
  have passed verification of the contract of `T::m()`. This makes the
  substitution safe even if the particular type has not been used in a checking
  harness.

  For higher order functions this gets a bit more tricky, as closures are ad-hoc
  defined types. Here the contract for the closure could be attached to `f()`
  and then checked for each closure that may be provided. However this does not
  work so long as the user has to provide the harnesses, as they cannot recreate
  the closure type.

---

[^gate]: Enforced gates means all uses of constructs (functions, annotations,
    macros) in this RFC are an error.

[^simple-unsoundness]: The main remaining threat to soundness in the use of
    contracts, as defined in this proposal, is the reliance on user-supplied
    harnesses for contract checking (explained in item 2 of [user
    experience](#user-experience)). A more thorough discussion on the dangers
    and potential remedies can be found in the [future](#future-possibilities)
    section.

[^write-set-recursion]: For inductively defined types the write set inference
    will only add the first "layer" to the write set. If you wish to modify
    deeper layers of a recursive type an explicit `modifies` clause is required.

[^inferred-footprint]: While inferred memory footprints are sound for both safe
    and unsafe Rust certain features in unsafe rust (e.g. `RefCell`) get
    inferred incorrectly and will lead to a failing contract check.

[^slice-exprs]: Slice indices can be place expressions referencing function
    arguments, constants and integer arithmetic expressions. Take for example
    this `Vec` method (places simplified vs. actual implementation in `std`):
    `fn truncate(&mut self, len: usize)`. A relatively precise contract for this
    method can be achieved with slice indices like so:
    `#[modifies(self.buf[len..self.len], self.len)]`

[^assigns-encapsulation-breaking]: Breaking the `pub` encapsulation has
    unfortunate side effects because it means the contract depends on non-public
    elements which are not expected to be stable and can drastically change even
    in minor versions. For instance if your project depends on crate `a` which
    in turn depends on crate `b`, and `a::foo` has a contract that takes as
    input a pointer data structure `b::Bar` then `a::foo`s `assigns` contract
    must reference internal fields of `b::Bar`. Say your project depends on the
    *replacement* of `a::foo`, if `b` changes the internal representation of
    `Bar` in a minor version update cargo could bump your version of `b`,
    breaking the contract of `a::foo` (it now crashes because it e.g. references
    non-existent fields).
    You cannot easily update the contract for `a::foo`, since it is a
    third-party crate; in fact even the author of `a` could not properly update
    to the new contract since their old version specification would still admit
    the new, broken version of `b`. They would have to yank the old version and
    explicitly nail down the exact minor version of `b` which defeats the whole
    purpose of semantic versioning.

[^external-contract-checking-expectations]: Contracts for functions from
    external crates (crates from outside the workspace, which is not quite the
    definition of `extern crate` in Rust) are not checked by default. The
    expectation is that the library author providing the contract has performed
    this check. See also [open question](#open-questions) for a discussion on
    defaults and checking external contracts.

[^stubcheck]: Kani cannot report the occurrence of a contract function to check
    in abstracted functions as errors, because the mechanism is needed to verify
    mutually recursive functions.

