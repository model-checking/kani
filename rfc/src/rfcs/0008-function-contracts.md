- **Feature Name:** Function Contracts
- **Feature Request Issue:** *Link to issue*
- **RFC PR:** *Link to original PR*
- **Status:** Under Review 
- **Version:** 0 [0-9]\* *Increment this version whenever you open a new PR to update the RFC (not at every revision).
  Start with 0.*
- **Proof-of-concept:** [features/contracts](https://github.com/model-checking/kani/tree/features/contracts)

-------------------

## Summary

Contracts are a powerful tool for verification. They are both a convenient way
to write specifications as well as allowing users to soundly approximate the
behavior of units of code. The verification tool then leverages these
approximations for modular verification which affords both scalability, but also
allows for verifying unbounded loops and recursion.

<!-- Shorter? -->

## User Impact

<!-- Is basically the pitch and addressing the user. -->

Function contracts are a mechanism for a stubbing-like abstraction of concrete
implementations but with a significantly reduced threat to soundness. It also
lays the ground work for the following two ambitious goals.

- **Scalability:** Function contracts are sound (over)abstractions of function
  behavior. By verifiying the contract against its implemetation and
  subsequently performing caller verification against the (cheaper) abstraction
  verification can be modularized, cached and thus scaled.
- **Unbounded Verification:** Contracts can be reasoned over inductively and
  thus verify recursive functions.

Enabling function contracts is a non-invasive change. While it contains a new
API, that API is strictly additive for users. All prior operations are unchanged.

A `-Zcontracts` guard will be added wich is necessary to access any of the
contract functionality (attributes) described below. Any use of such attributes
without `-Zcontracts` is a compile time error.

### Caveats

This proposal focuses on scalability benefits within a single verification session,
caching strategies for cross-session speedup are left to future work.

We add function contract functionality, but do not add the indictive reasoning
support needed for many unbounded problems, such as "deacreases" measures and
inductive lemmas.

## User Experience

Function contracts provide a way to approximate function behavior, verify the
approximation and subsequently use the approximation instead if the (possibly
costly) implementation. The lifecycle of a contract is split roughly into three
phases. Which we will explore on this simple example:

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
   function arguments and, in case of `ensures`, the `result`[^result-naming]
   result of the function. Syntactically Kani supports any Rust expression,
   including function calls, defining types etc, however they must be
   side-effect free[^side-effects].

   [^result-naming]: See [open questions](#open-questions) for a discussion
       about naming of the result variable.

2. Checking the Contract

   Next Kani must make sure that the approximation we specified actually holds.
   This is in opposition to the related "stubbing" mechanism, where the
   approximation is not checked but always trusted.

   Kani must check that contract overapproximates the function to guarantee
   soundness. Specifically the domain (inputs) of the function described (by the
   `requires` clause) must be *at most* as large as the input space for which
   the function's behavior is well defined, and the codomain (outputs, described
   by `ensures`) must be *at least* as large as the actual space of outputs the
   function may produce.

   For example in our case it would be permissible to use
   `#[kani::requires(divisor > 100)]` (smaller permissible input domain) or
   `#[kani::ensures(result < dividend + divisor)]` (larger possible output
   domain), but e.g. `#[kani::ensures(result < dividend)]` (small output) is not
   allowed.

   To facilitate the check Kani needs a suitable environment to verify our
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

   Similar to a unit-test harness for any other function we are supposed to
   create inputs for the function that are as generic/abstract as possible to
   make sure we catch all edge cases, then call the function at least once with
   those abstract inputs. If we forget to call `my_div` Kani would report an
   error.
   
   Unlike a unit-test we can however elide any checks of the output and
   post-call state. Instead Kani uses the conditions we specified in the
   contract as checks. It inserts the preconditions (`requires`) of `my_div` as
   `kani::assume` *before* the call to `my_div`, to ensure it only tests the
   function on inputs it is actually defined for. Postconditions (`ensures`) are
   inserted as `kani::assert` checks *after* the call to `my_div`. 

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
   **stub** in other harnesses.

   Unlike in regular stubbing Kani there to be at least one associated
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
   safe. It a `kani::assert` for the preconditions (`requires`) of `my_div`
   before the call. Then it replaces the result of `my_div` with a
   non-deterministic value (see [havocing](#memory-predicates-and-havocing) for
   the equivalent for mutable memory), constrained by a `kani::assume` of
   `'my_div`'s postconditions (`ensures`).

   And the expanded version is equivalent to this:
  
   ```rs
   #[kani::proof]
   #[kani::stub_verified(my_div)]
   fn use_div() {
     let v = vec![...];
     let dividend = v.len() - 1;
     let divisor = 3;
     kani::assert(divisor != 0);
     let result = kani::any();
     kani::assume(result <= dividend);
     let some_idx = result;
     v[some_idx];
   }
   ```

   Notice that this performs no actual computation for `my_div` (other than the
   conditions) which allows us to avoid something potentially costly.

Also notice that Kani was able to express both contract checking and stubbing
with existing capabilities, the important feature is the enforcement. The
checking is, by construction, performed **against the same condition** that is
later used as stub, which ensures soundness (see discussion on lingering threats
to soundness in the [future](#future-possibilities) section) and guards against
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
`pop`) and makes the result available to `ensures`. it would be used like so

```rs
impl<T> Vec<T> {
  #[kani::ensures(old(self.is_empty()) || result.is_some())]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

`old` allows evaluating any (side-effect free[^side-effects]) Rust expression
but Rust enforces that it does not borrow so as to observe the mutations from
e.g. `pop`, as that would defeat the purpose. Instead Kani encourages us to make
copies (using e.g. `clone()`) if necessary.

Note also that `old` is syntax, not a function and implemented as an extraction
and lifting during code generation. It can reference e.g. `pop`'s arguments but
not local variables. Compare the following

**Invalid ❌:** `#[kani::ensures({ let x = self.is_empty(); old(x) } || result.is_some())]`
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
only under the mentioned conditions.

`CONDITION`s limit when the clause applies, may reference the arguments of the
function and can be omitted.

`LVALUE` are simple expressions permissible on the left hand side of an
assignment. They compose of the name of one function argument and zero or more
projections (dereference `*`, field access `.x`, slice indexing `[1]`).

The `ASSIGN_RANGE` permits any `LVALUE` but additionally permits more complex
slice expressions as the last projection and applies to pointer values. `[..]`
denotes the entirety of an allocation, `[i..]`, `[..j]` and `[i..j]` are
ranges of pointer offsets. A slicing syntax `p[i..j]` only applies if `p` is a
`*mut T` and points to an array allocation. The slice indices are offsets with
sizing `T`, e.g. in Rust `p[i..j]` would be equivalent to
`std::slice::from_raw_parts(p.offset(i), i - j)`. `i` must be smaller or equal
than `j`.

Because lvalues are restricted to using projections only Kani must break
encapsulation here. If need be we may reference fields that are usually hidden,
without an error from the compiler.

See also discussion on conditions in assigns clauses in

[^side-effects]: Code used in contracts is required to be side effect free which
    means it must not perform I/O, mutate memory (`&mut` vars and such) and
    (de)allocate heap memory. This is enforced by the verifier, see the
    discussion in the [future](#future-possibilities) section.

What is the scope of this RFC? Which use cases do you have in mind? Explain how users will interact with it. Also
please include:

## Detailed Design

<!-- For the implementors or the hackers -->


### Code generation in `kani_macros`

The `requires` and `ensures` macros generate new sibling functions to e.g. `my_div`
(see also discussion in [alternatives](#rationale-and-alternatives)). One
function is generated which corresponds to checking the contract holds for the
implementation. One function is generated which corresponds to approximating the
function behavior when called with the same arguments.

The complete code generated for the example is shown below and followed by an explanation of each component.

```rs
fn my_div_check_copy_965916(dividend: u32, divisor: u32) -> u32 { dividend / divisor }
fn my_div_replace_copy_965916(dividend: u32, divisor: u32) -> u32 { kani::any() }

#[kanitool::checked_with = "my_div_check_5e3713"]
#[kanitool::replaced_with = "my_div_replace_5e3713"]
fn my_div(dividend: u32, divisor: u32) -> u32 { dividend / divisor }

fn my_div_check_5e3713(dividend: u32, divisor: u32) -> u32 {
    let dividend_renamed = kani::untracked_deref(&dividend);
    let divisor_renamed = kani::untracked_deref(&divisor);
    let result = my_div_check_965916(dividend, divisor);
    kani::assert(result <= dividend_renamed, "result <= dividend");
    result
}

fn my_div_replace_5e3713(dividend: u32, divisor: u32) -> u32 {
    let dividend_renamed = kani::untracked_deref(&dividend);
    let divisor_renamed = kani::untracked_deref(&divisor);
    let result = my_div_replace_965916(dividend, divisor);
    kani::assume(result <= dividend_renamed);
    result
}

fn my_div_check_965916(dividend: u32, divisor: u32) -> u32 {
    kani::assume(divisor != 0);
    my_div_copy_965916(dividend, divisor)
}

fn my_div_replace_965916(dividend: u32, divisor: u32) -> u32 {
    kani::assert(divisor != 0, "divisor != 0");
    my_div_replace_copy_965916(dividend, divisor)
}
```


To support mutiple clauses while performing all code generation at macro
expansion time each clause separately generates both a checking and a
replacement function, wrapping like onion layers around any prior checks. Both
the generated check and replace function is attached to the annotated function
using `kanitool::{checked,replaced}_with` annotations. When the item is
reemitted from the clause macro, the  `kanitool` annotation is placed last in
the attribute sequence, so that clauses expanded later can see it. Those
subsequently expanded clauses use the `kanittol` annotations to determine which
function to call inside them next. If no prior `kanitool` annotation is present
then the check function calls a copy of `my_div`instead. The copy is called in
case of the `check` function, since the compiler will later substitute all
occurrences of `my_div` with the `check` function which would also apply here
and cause an infinite recursion and make the original `my_div` body
inaccessible. The replace function also makes a copy, the body of which is a
`kani::any()` non-determinstic value and this copy carries any memory predicates
which will be havoced by CBMC.

Each generated function is named
`<original_name>_{replace,check,reaplace_copy,check_copy}_<hash>`, where `hash`
is a hash of the original "function item" ast, in this case `my_div`, including
any attributes, such as `#[kanitool::checked_with = "my_div_check_5e3713"]` from
clauses expanded earlier, which guarantees the hash is unique for each clause
expansion.

Type signatures of the generated functions are always identical to the type
signature of the contracted function, including type parameters and lifetimes.


### Dealing with mutable borrows

Preconditions (`requires`) are emitted as-is into the generated function,
providing access to the function arguments directly. This is safe because they
are required to be side-effect free[^side-effects]. 

Postconditions (`ensures`) have to be handled specially. They can reference the
arguments of the function, though not modify them. However this is problematic
even without modification if part of an input is borrowed mutably as would be
the case in the following example of the `Vec::split_at_mut` function.

```rs
impl<T> Vec<T> {
  #[ensures(self.len() == result.0.len() + result.1.len())]
  fn split_at_mut(&mut self, i: usize) -> (&mut [T], &mut [T]) {
    ...
  }
}
```

This contract refers simultaneously to `self` and the result. Since the method however borrows `self` mutably, the borrow checker would not allow the following simplistic code generation:

```rs
impl<T> Vec<T> {
  fn split_at_mut_check_<hash>(&mut self, i: usize) -> (&mut [T], &mut [T]) {
    let result = self.split_at_mut(i);
    kani::assert(self.len() == result.0.len() + result.1.len());
    result
  }
}
```
This proposal also introduces a new hidden builtin `kani::unchecked_deref`. The
necessity for this builtin is explained [later](#dealing-with-mutable-borrows).


`self` would not be permitted to be used here until `result` goes out of scope
and releasese the borrow. To avoid this issue we break the borrowchecker
guarantee with a new biltin `fn kani::unsafe_deref<T>(t: &T) -> T`. The
implementation of this function is simply a CBMC level `*` (deref). In Rust this
implementation would be illegal without the `Copy` trait (which generates a
copy) but in CBMC this is acceptable. Breaking the borrow checker this way is safe for 2 reasons:

1. Postconditions are not allowed perform mutation[^side-effects] and
2. Post conditions are of type `bool`, meaning they cannot leak references to
   the arguments and cause race conditions.

Circumventing the borrow checker is facilitated with a visit over the initial
postcondition expression that renames every occurrence of the arguments to a
fresh identifier and then generates a call `let i = unsafe_deref(&a)` for each
argument `a` and fresh identifier `i` **before** the call to the contracted
function. Because `unsafe_deref` creates shallow copies, they will witness any
modifications of memory they point to.

Shadowing.

### History Variables

The special `old` builtin function is implemented as an AST rewrite. Consider the below example:

```rs
impl<T> Vec<T> {
  #[kani::ensures(self.is_empty() || self.len() == old(self.len()) - 1)]
  fn pop(&mut self) -> Option<T> {
    ...
  }
}
```

`old` gives the user access to `self.len()`, evaluated before `pop` to be able
to compare it to `self.len()` after `pop` mutates `self`.

While `old` might appear like a function it is not. The implementation lifts the
argument expression to old via AST rewrite in the `ensures` macro to before the
call to `pop` and binds it to a temporary variable. This makes `old` syntax
rather than a function, but also makes it very powerful as any expression is
allowed in `old` including calculations, function calls etc. Our example would
generate the code below:

```rs
impl<T> Vec<T> {
  fn pop_check_<hash>(&mut self) -> Option<T> {
    let old_1 = self.len();
    let result = Self::pop_copy_<hash>(self);
    kani::assert(self.is_empty() || self.len() == old_1 - 1)
  }
}
```

Note that unlike for arguments for postconditions, we do not use `unsafe_deref`
to break the borrowing rules. Unlike for those arguments, which must witness
mutations, the expression in `old` is supposed to reflect the state *before* the
function call and must therefore not observe mutations performed by e.g. `pop`.
We can use the borrowchecker to enforce this for safe Rust[^old-safety]. The
borrow checker will ensure that none of the temporary variables created borrow
from any mutable arguments and thus guarantee that they cannot witness mutations
in e.g. `pop`. To use e.g. `old(self)` in such a case the user can create copies
with the usual mechanism, such as `clone`, e.g. `old(self.clone())`.

[^old-safety]: For unsafe rust we need additional protections which are not part
    of this proposal but are similar to the side effect freedom checks discussed
    in the [future section](#future-possibilities)

### Assigns Clauses

- Inference
- Lvalue tracking
- Code generation for conditions
- Code generation for slice patterns

### Substitution with `kani_compiler` 

Harnesses annotated with `for_contract` or `use_contract` are subject to
substitution. Only one `for_contract(f)` annotation is allowed per harness and
it triggers substitution of the target function `f` with the check function in
the `#[kanitool::checked_with = ...]` annotation on `f`. Multiple
`use_contract(g)` annotations are allowed on each harness, including on
`for_contract` harnesses, though the simultaneous presence of `for_contract` and
`use_contract` for the same target function is not permissible.

If the target function (`f` or `g`) does not have a `checked_with` or
`replaced_with` attribute (respectively) an error is thrown.

### Invoking `goto-instrument` from `kani-driver`

In addition to the Kani side substitiution we also perform instrumentation on
the CBMC because we rely on it's support for memory predicates. The
generated memory predicates are emitted from the compiler as CBMC contracts. To
enforce the memory contract `goto-instrument` has to be invoked with the correct
functions name. Since this is after lowering into GOTO-C the name provided has
to be the mangled name of the monomorphized instances. The compiler determines
which monomorphized version of the contracted functions are used in a
reachability pass. Those names are passed to the driver (as the component that
invokes `goto-instrument`) via the `HarnessAttributes` struct, using an `Option`
to represent a possible contract to enforce and a `Vec` as the contracts which
are used as abstractions.

We call `goto-instrument --enforce-contract <for_contract fn> --replace-call-with-contract <use_contract fns>`

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

- We assume here entirely derived assigns clauses, instead of explicit one's.
- Semantics of arguments in postconditions: Shold they reflect changes to `mut`
  arguments, e.g. a `mut i: u32`? I think that even in other tools (e.g. CBMC)
  the actual value of arguments is copied into the function and therefore
  changes to it are not reflected.
- Trait contracts
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

 
<!-- 
- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design? 
-->

## Future possibilities

<!-- For Developers -->

- Enforcing contract checking before substitution
- Quantifiers
- Side effect freedom is currently enforced by CBMC. This means that the error
  originates there and is likely not legible. Intead Kani should perform a
  reachability analysis from the contract expressions and determine whether side
  effects are possible, throwing a graceful error.
- Soundness issues with harnesses and remedies that are in the works
- What about mutable trait inputs (wrt memory access patters), e.g. a `mut impl AccessMe`

What are natural extensions and possible improvements that you predict for this feature that is out of the
scope of this RFC? Feel free to brainstorm here.