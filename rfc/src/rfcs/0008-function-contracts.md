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

## User Impact

Enabling function contracts is a non-invasive change. While it contains a new
API, that API is strictly additive for users. All prior operations are unchanged.

The proposal in this RFC lays the ground work for two important goals. The
following describes those goals and any residual challenges to overcome to
achieve them.

- **Scalability:** Function contracts are sound (over)abstractions of function
  behavior. By verifiying the contract against its implemetation and
  subsequently performing caller verification against the (cheaper) abstraction
  verification can be modularized, cached and thus scaled.

  This proposal would yield those benefits within a single verification session,
  but would still re-verify the contract every session. With a suitable caching
  mechanism this could be avoided in future.
- **Unbounded Verification:** Contracts can be reasoned over inductively and
  thus verify recursive functions.

  This proposal adds basic function contract functionality, but does not add an
  indictive reasoning framework. 

## User Experience

This proposal introduces 6 new annotations.

- `#[kani::requires(CONDITION)]` and `#[kani::ensures(CONDITION)]` are
  annotations for non-harness functions (lets use `f` for an example) and encode
  pre- and postconditions respectively.

  Preconditions are a refinement (beyhond the type) of the domain of the
  `f`.

  Postconditions are a refinement (beyond the type) of the codomain of the
  `f`.

  `CONDITION`s are arbitrary boolean Rust expressions[^side-effects]. They may
  reference the arguments of `f`. In the case of preconditions the value of
  those arguments will be the value they have before `f` is called, in the case
  of postconditions the value after the call to `f` returns. Additionally the
  postcondition has access to the result of `f` in a variable called `result`.
  (see [Open Questions](#open-questions) for a discussion on the `result`
  variable.)

  A postcondition may also use the special builtin `old` pseudo function.
  `old(EXPR)` evaluates `EXPR` in the context *before* the call to `f`, e.g.
  like a precondition.

  A function may be annotated with multiple `requires` and `ensures` clauses
  that are implicitly conjoined. The sum total of the clauses and the assigns
  clause (see below) comprise the function contract.

- `#[kani::for_contract(PATH)]` is a harness annotation which designates that
  this harness is especially designed for verifying that function `PATH` obeys
  it's contract. As an example assume `PATH` is `f`.

  The harness itself is implemented as usual by the user and verified the same
  way as any other, including admitting other verification features like
  stubbing. However additionally the preconditions of `f` id `kani::assume`'d
  before every call to `f` and the postconditions are `kani::asssert`'ed after
  it returns. 
  
  Kani checks that `f` is in the call chain of this harness, but no further
  checks are performed whether this harness is suitable for verifying the
  contract of `f`, see also [Open Questions](#open-questions).

  Only one `for_contract` annotation is permitted per harness.

- `#[kani::use_contract(PATH)]` is a harness annotation which instructs the
  verifier to use the contract of `PATH` during verification instead of it's
  body. 

  `use_contract(f)` is only safe if a `for_contract(f)` harness has previously
  been verified. See [Future possibilities](#future-possibilities) for a
  discussion on automatic enforcement mechanisms of this requirement.

  Multiple `use_contract` annotations are permitted for a single harness.

- The memory predicate family `#[kani::assigns(CONDITION, ASSIGN_RANGE...)]`,
  `#[kani::frees(CONDITION, LVALUE...)]`  expresses manual contraints on which
  parts of an object the annotated function may assigned/freed.

  In both cases the `CONDITION`s limit the applicability of the clause, may
  reference the arguments of the function and can be omitted in which case they
  default to `true`.

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

  Because lvalues are restricted to using fields we break encapsulation here.
  You may, if need be, reference fields that are usually hidden without an error
  from the compiler.

  See also discussion on conditions in assigns clauses in
  [Rationale](#rationale-and-alternatives)

- To reduce developer burden we additionally propose to leverage the rust type
  information to overapproximate the memory a function may modify. This allows
  sound havocing in the absence of an `assigns` or `frees`. Inference of memory
  clauses considers any `mut` reachable memory to be potentially freed,
  reallocated or reassigned for any execution of the function. In addition any
  reachable `static` variables are considered modified.

[^side-effects]: Contract conditions are required to be side effect free, e.g.
    perform no I/O perform no memory mutation and allocate/free no heap memory.
    See also the side effect discussion in [Open Questions](#open-questions).

This proposal also introduces a new hidden builtin `kani::unchecked_deref`. The
necessity for this builtin is explained [later](#dealing-with-mutable-borrows).

<!-- 
What is the scope of this RFC? Which use cases do you have in mind? Explain how users will interact with it. Also
please include:

- How would you teach this feature to users? What changes will be required to the user documentation?
- If the RFC is related to architectural changes and there are no visible changes to UX, please state so. 
-->


## Detailed Design

The lifecycle of a contract is split roughly into three phases. Let us consider
as an example this function:

```rs
fn my_div(dividend: u32, divisor: u32) -> u32 {
  dividend / divisor
}
```

1. Specifying the contract

   The user provides a specification (some combination of `requires`, `ensures`,
   `assigns`, ...). In our case this may look like so:

   ```rs
   #[kani::requires(divisor != 0)]
   #[kani::ensures(result <= dividend)]
   fn my_div(dividend: u32, divisor: u32) -> u32 {
     dividend / divisor
   }
   ```

  Any absent clause defaults to `true` (no constraints on input, output or
  memory). 
  

2. Checking the contract

   It is important that the combination of clauses is an
   overapproximation of the function's behavior. This means the domain of the
   function described (by the `requires`) clause is *at most* as large as the
   actual function domain (the input space for which it's behavior is well
   defined) and the codomain (described by `ensures`, `assigns` and `frees`) is
   *at least* as large as the actual space of outputs a function may produce.

   For example in this case it would be permissible to use
   `#[kani::requires(divisor > 100)]` (smaller permissible input domain) or
   `#[kani::ensures(result < dividend + divisor)]` (larger possible output
   domain), but e.g. `#[kani::ensures(result < dividend)]` is not allowed.

   The verifyer must check that this overapproximation property holds. To do so
   it requires a suitably generic environment in which to test pre and
   postconditions. The choice of environment has implications on soundness and
   ideally the verifier can create an environment automatically. This is a
   difficult problem due to heaps and part of [future
   possibilities](#future-possibilities). For the purposes of this proposal the
   user must provide a suitable harness as checking environment. This is done
   with the `for_contract` annotation (below).

   ```rs
   #[kani::proof]
   #[kani::for_contract(my_div)]
   fn my_div_harness() {
     my_div(kani::any(), kani::any())
   }
   ```

   To facilitate contract checking against the implementation of `my_div` the
   verifier performs code generation which turns preconditions (`requires`) into
   `kani::assume` calls before function execution. This restricts the arbitrary
   (`kani::any`) input domain from the harness to the one claimed by the
   precondition. We also turn postconditions (`ensures`, `assigns`...) into
   `kani::assert` calls *after* the function execution verifying the integrity
   of the codomain.

   ... Done like stubbing

- Substituting the contract

Let us consider a complete example:

```rs


#[kani::proof]
#[kani::use_contract]
fn use_div() {
  let v = vec![...];
  let some_idx = my_div(v.len() - 1, 3);
  v[some_idx];
}
```

The following subsections describe the Kani pipeline for this example in order.

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


## Open questions

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

 
<!-- 
- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design? 
-->

## Future possibilities

- Enforcing contract checking before substitution
- Quantifiers
- Side effect freedom is currently enforced by CBMC. This means that the error
  originates there and is likely not legible. Intead Kani should perform a
  reachability analysis from the contract expressions and determine whether side
  effects are possible, throwing a graceful error.

What are natural extensions and possible improvements that you predict for this feature that is out of the
scope of this RFC? Feel free to brainstorm here.