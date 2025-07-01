// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the loop contracts code generation.
//!

use proc_macro::TokenStream;
use proc_macro_error2::abort_call_site;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::AndAnd;
use syn::{
    BinOp, Block, Expr, ExprBinary, Ident, Stmt, Token, parse_macro_input, parse_quote,
    visit_mut::VisitMut,
};

/*
    Transform the loop to support on_entry(expr) : the value of expr before entering the loop
    1. For each on_entry(expr) in the loop variant, replace it with a newly generated "memory" variable old_k
    2. Add the declaration of i before the loop: let old_k = expr
    For example:
    #[kani::loop_invariant(on_entry(x+y) = x + y -1)]
    while(....)

    is transformed into
    let old_1 = x + y
    #[kani::loop_invariant(old_1 = x + y -1)]
    while(....)

    Then the loop_invartiant is transformed.

    Transform the loop to support prev(expr) : the value of expr at the end of the previous iteration
    Semantic: If the loop has at least 1 iteration: prev(expr) is the value of expr at the end of the previous iteration. Otherwise, just remove the loop (without check for the invariant too).

    Transformation: basically, if the loop has at least 1 iteration (loop_quard is satisfied at the beginning), we unfold the loop once, declare the variables for prev values and update them at the beginning of the loop body.
    Otherwise, we remove the loop.
    If there is a prev(expr) in the loop_invariant:
    1. Firstly, add an if block whose condition is the loop_quard, inside its body add/do the followings:
    2. For each prev(expr) in the loop variant, replace it with a newly generated "memory" variable prev_k
    3. Add the declaration of prev_k before the loop: let mut prev_k = expr
    4. Define a mut closure whose body is exactly the loop body, but replace all continue/break statements with return true/false statements,
            then add a final return true statement at the end of it
    5. Add an if statement with condition to be the that closure's call (the same as run the loop once):
        True block: add the loop with expanded macros (see next section) and inside the loop body:
            add the assignment statements (exactly the same as the declarations without the "let mut") on the top to update the "memory" variables
        Else block: Add the assertion for the loop_invariant (not includes the loop_quard): check if the loop_invariant holds after the first iteration.

    For example:
    #[kani::loop_invariant(prev(x+y) = x + y -1 && ...)]
    while(loop_guard)
    {
        loop_body
    }

    is transformed into

    assert!(loop_guard);
    let mut prev_1 = x + y;
    let mut loop_body_closure = || {
        loop_body_replaced //replace breaks/continues in loop_body with returns
    };
    if loop_body_closure(){
        #[kani::loop_invariant(prev_1  = x + y -1)]
        while(loop_guard)
        {
            prev_1 = x + y;
            loop_body
        }
    }
    else{
        assert!(prev_1 = x + y -1 && ...);
    }

    Finally, expand the loop contract macro.

    A while loop of the form
    ``` rust
     while guard {
         body
     }
    ```
    will be annotated as
    ``` rust
    #[inline(never)]
    #[kanitool::fn_marker = "kani_register_loop_contract"]
    const fn kani_register_loop_contract_id<T, F: FnOnce() -> T>(f: F) -> T {
        unreachable!()
    }
     while kani_register_loop_contract_id(|| -> bool {inv};) && guard {
         body
     }
    ```
*/

struct TransformationResult {
    transformed_expr: Expr,
    declarations_block: Block,
    assignments_block: Block,
}

struct CallReplacer {
    old_name: String,
    replacements: Vec<(Expr, proc_macro2::Ident)>,
    counter: usize,
    var_prefix: String,
}

// This impl replaces any function call of a function name : old_name with a newly generated variable.
impl CallReplacer {
    fn new(old_name: &str, var_prefix: String) -> Self {
        Self { old_name: old_name.to_string(), replacements: Vec::new(), counter: 0, var_prefix }
    }

    fn generate_var_name(&mut self) -> proc_macro2::Ident {
        let var_name = format_ident!("{}_{}", self.var_prefix, self.counter);
        self.counter += 1;
        var_name
    }

    //Check if the function name is old_name
    fn should_replace(&self, expr_path: &syn::ExprPath) -> bool {
        let full_path = expr_path
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        full_path == self.old_name
    }
}

impl VisitMut for CallReplacer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Visit nested expressions first
        syn::visit_mut::visit_expr_mut(self, expr);

        if let Expr::Call(call) = expr {
            if let Expr::Path(expr_path) = &*call.func {
                if self.should_replace(expr_path) {
                    let replace_var =
                        self.replacements.iter().find(|(e, _)| e == expr).map(|(_, v)| v);
                    if let Some(var) = replace_var {
                        *expr = syn::parse_quote!(#var);
                    } else {
                        let new_var = self.generate_var_name();
                        self.replacements.push((expr.clone(), new_var.clone()));
                        *expr = syn::parse_quote!(#new_var);
                    };
                }
            }
        }
    }
}

// The main function to replace the function call with the variables
fn transform_function_calls(
    expr: Expr,
    function_name: &str,
    var_prefix: String,
) -> TransformationResult {
    let mut replacer = CallReplacer::new(function_name, var_prefix);
    let mut transformed_expr = expr;
    replacer.visit_expr_mut(&mut transformed_expr);

    let mut newreplace: Vec<(Expr, Ident)> = Vec::new();
    for (call, var) in replacer.replacements {
        if let Expr::Call(call_expr) = call {
            let insideexpr = call_expr.args[0].clone();
            newreplace.push((insideexpr, var.clone()));
        }
    }

    // Generate declarations block of the newly generated variables (will added before the loop)
    let declarations: Vec<Stmt> = newreplace
        .iter()
        .map(|(call, var)| syn::parse_quote!(let mut #var = #call.clone();))
        .collect();
    let declarations_block: Block = syn::parse_quote!({
        #(#declarations)*
    });

    // Generate declarations block of the newly generated variables (will be added on the loop of the loop body)
    let assignments: Vec<Stmt> = newreplace
        .into_iter()
        .map(|(call, var)| syn::parse_quote!(#var = #call.clone();))
        .collect();
    let assignments_block: Block = syn::parse_quote!({
        #(#assignments)*
    });

    TransformationResult { transformed_expr, declarations_block, assignments_block }
}

struct BreakContinueReplacer;

impl VisitMut for BreakContinueReplacer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Visit nested expressions first
        syn::visit_mut::visit_expr_mut(self, expr);

        // Replace the expression
        *expr = match expr {
            Expr::Break(_) => {
                syn::parse_quote!(return (false, None))
            }
            Expr::Continue(_) => {
                syn::parse_quote!(return (true, None))
            }
            Expr::Return(rexpr) => match rexpr.expr.clone() {
                Some(ret) => syn::parse_quote!(return (false, Some(#ret))),
                _ => syn::parse_quote!(return (false, Some(()))),
            },
            _ => return,
        };
    }
}

// This function replace the break/continue statements inside a loop body with return statements
fn transform_break_continue(block: &mut Block) {
    let mut replacer = BreakContinueReplacer;
    replacer.visit_block_mut(block);
    let return_stmt: Stmt = syn::parse_quote! {
        return (true, None);
    };
    // Add semicolon to the last statement if it's an expression without semicolon
    if let Some(Stmt::Expr(_, ref mut semi)) = block.stmts.last_mut() {
        if semi.is_none() {
            *semi = Some(Default::default());
        }
    }
    block.stmts.push(return_stmt);
}

/*
`for` loop implementation:
A for loop of the form
``` rust
for pat in expr {
    body
}
```
is transformed into a `while` loop:

``` rust
let kaniiter = kani::KaniIntoIter::kani_into_iter(expr); \\ init_iter_stmt
let kaniiterlen = kani::KaniIter::len(&kaniiter); \\ init_len_stmt
if kaniiterlen > 0 {
    let mut kaniindex = 0; \\ init_index_stmt
    kani::assume (kani::KaniIter::assumption(&kaniiter)); \\ iter_assume_stmt
    let pat = kani::KaniIter::first(&kaniiter); \\ init_pat_stmt
    while kaniindex < kaniiterlen {
        kani::assume (kani::KaniIter::assumption(&kaniiter)); \\ iter_assume_stmt
        let pat = kani::KaniIter::indexing(&kaniiter, kaniindex); \\ pat_assign_stmt
        kaniindex = kaniindex + 1; \\ increase_iter_stmt
        body
    }
}
```
Note that

1. expr's type is required to impl KaniIntoIter trait, which is the override impl of Rust's IntoIter (see library/kani_core/src/iter.rs).
Reason:
    a) Reduce stack-call
    b) Avoid types that cannot be havoc effectively (user cannot state the type invariant in the loop invariant due to private fields)
    c) There is no generic way to handle Rust's into_iter().

2. The init_index_stmt and init_pat_stmt statements supports writing the loop-invariant properties that involve pat and kaniindex

3. The iter_assume_stmt assumes some truths about allocation, so that the user does not neet to specify them in the loop invariant
*/

//The group of 5 extra stmts (mentioned above) that are outside of the loop
pub struct ForLoopExtraStmts {
    init_iter_stmt: Stmt,
    init_len_stmt: Stmt,
    init_index_stmt: Stmt,
    init_pat_stmt: Stmt,
    iter_assume_stmt: Stmt,
}

//Transform a for loop into a while loop according to the mechanism mentioned above
pub fn transform_for_to_loop(
    for_loop: ExprForLoop,
    loop_id: &str,
) -> (Stmt, Option<ForLoopExtraStmts>) {
    // Extract components from the for loop
    let pat = *for_loop.pat;
    let expr = for_loop.expr;
    let body = for_loop.body;

    // Create an iterator variable name
    let indexname = "kaniindex".to_owned();
    let index_ident = format_ident!("{}", indexname);

    let mut itername = "kaniiter".to_owned();
    itername.push_str(loop_id);
    let iter_ident = format_ident!("{}", itername);

    let lenname = "kaniiterlen".to_owned();
    let len_ident = format_ident!("{}", lenname);

    // Create initialization statement for the iterator
    let init_iter_stmt: Stmt = parse_quote! {
        let #iter_ident = kani::KaniIntoIter::kani_into_iter(#expr);
    };

    let init_index_stmt: Stmt = parse_quote! {
        let mut #index_ident = 0;
    };

    let init_len_stmt: Stmt = parse_quote! {
        let #len_ident = kani::KaniIter::len(&#iter_ident);
    };

    let init_pat_stmt: Stmt = parse_quote! {
        let #pat = kani::KaniIter::first(&#iter_ident);
    };

    // Create the new loop body with iterator advancement
    let mut new_body_stmts = Vec::new();

    let iter_assume_stmt: Stmt = parse_quote! {
        kani::assume (kani::KaniIter::assumption(&#iter_ident));
    };

    // Increase the iter
    let increase_iter_stmt: Stmt = parse_quote! {
        #index_ident = #index_ident + 1;
    };

    let pat_assign_stmt: Stmt = parse_quote! {
        let #pat = kani::KaniIter::indexing(&#iter_ident, #index_ident);
    };

    new_body_stmts.push(iter_assume_stmt.clone());
    new_body_stmts.push(pat_assign_stmt);
    new_body_stmts.push(increase_iter_stmt);

    // Add the original loop body statements
    new_body_stmts.extend(body.stmts.iter().cloned());

    // Create the final expression with the iterator initialization
    let loop_loop: Stmt = parse_quote! {
            while (#index_ident < #len_ident) {
                #(#new_body_stmts)*
            }
    };
    let for_loop_extras = ForLoopExtraStmts {
        init_iter_stmt,
        init_len_stmt,
        init_index_stmt,
        init_pat_stmt,
        iter_assume_stmt,
    };

    (loop_loop, Some(for_loop_extras))
}

pub fn loop_invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse the stmt of the loop
    let mut loop_stmt: Stmt = syn::parse(item.clone()).unwrap();
    let loop_id = generate_unique_id_from_span(&loop_stmt);
    //We use Option to mark if the loop is a for loop.
    //i.e. if for_loop_extras.is_some() then it is a for loop
    let mut for_loop_extras: Option<ForLoopExtraStmts> = None;
    if let Stmt::Expr(Expr::ForLoop(for_loop), _) = &loop_stmt {
        (loop_stmt, for_loop_extras) = transform_for_to_loop(for_loop.clone(), &loop_id);
    }
    // name of the loop invariant as closure of the form
    // __kani_loop_invariant_#startline_#startcol_#endline_#endcol
    let mut inv_name: String = "__kani_loop_invariant".to_owned();
    inv_name.push_str(&loop_id);

    // expr of the loop invariant
    let mut inv_expr: Expr = syn::parse(attr).unwrap();
    if for_loop_extras.is_some() {
        inv_expr = parse_quote! { kaniindex <= kaniiterlen && #inv_expr}
    }

    // adding on_entry variables
    let mut onentry_var_prefix: String = "__kani_onentry_var".to_owned();
    onentry_var_prefix.push_str(&loop_id);
    let replace_onentry: TransformationResult =
        transform_function_calls(inv_expr.clone(), "on_entry", onentry_var_prefix);
    inv_expr = replace_onentry.transformed_expr.clone();
    let onentry_decl_stms = replace_onentry.declarations_block.stmts.clone();

    // adding prev variables
    let mut prev_var_prefix: String = "__kani_prev_var".to_owned();
    prev_var_prefix.push_str(&loop_id);
    let transform_inv: TransformationResult =
        transform_function_calls(inv_expr.clone(), "prev", prev_var_prefix);
    let has_prev = !transform_inv.declarations_block.stmts.is_empty();
    let prev_decl_stms = transform_inv.declarations_block.stmts.clone();
    let mut assign_stms = transform_inv.assignments_block.stmts.clone();
    let (mut loop_body, loop_guard) = match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ew) => (ew.body.clone(), ew.cond.clone()),
            Expr::Loop(el) => (el.body.clone(), parse_quote!(true)),
            _ => panic!(),
        },
        _ => panic!(),
    };
    let loop_body_stms = loop_body.stmts.clone();
    assign_stms.extend(loop_body_stms);
    transform_break_continue(&mut loop_body);
    let mut loop_body_closure_name: String = "__kani_loop_body_closure".to_owned();
    loop_body_closure_name.push_str(&loop_id);
    let loop_body_closure = format_ident!("{}", loop_body_closure_name);
    let mut loop_body_closure_ret_1_name: String = "__kani_loop_body_closure_ret_1".to_owned();
    loop_body_closure_ret_1_name.push_str(&loop_id);
    let loop_body_closure_ret_1 = format_ident!("{}", loop_body_closure_ret_1_name);
    let mut loop_body_closure_ret_2_name: String = "__kani_loop_body_closure_ret_2".to_owned();
    loop_body_closure_ret_2_name.push_str(&loop_id);
    let loop_body_closure_ret_2 = format_ident!("{}", loop_body_closure_ret_2_name);
    if has_prev {
        inv_expr = transform_inv.transformed_expr.clone();
        match loop_stmt {
            Stmt::Expr(ref mut e, _) => match e {
                Expr::While(ew) => ew.body.stmts = assign_stms.clone(),
                Expr::Loop(el) => el.body.stmts = assign_stms.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };
    }

    // ident of the register function
    let mut register_name: String = "kani_register_loop_contract".to_owned();
    register_name.push_str(&loop_id);
    let register_ident = format_ident!("{}", register_name);

    match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ref mut ew) => {
                let new_cond: Expr = syn::parse(
                    quote!(
                        #register_ident(&||->bool{#inv_expr}, 0))
                    .into(),
                )
                .unwrap();
                *(ew.cond) = Expr::Binary(ExprBinary {
                    attrs: Vec::new(),
                    left: Box::new(new_cond),
                    op: BinOp::And(AndAnd::default()),
                    right: ew.cond.clone(),
                });
            }
            Expr::Loop(ref mut el) => {
                let invstmt: Stmt = syn::parse(quote!(if !(#register_ident(&||->bool{#inv_expr}, 0)) {assert!(false); unreachable!()};).into()).unwrap();
                let mut new_stmts: Vec<Stmt> = Vec::new();
                new_stmts.push(invstmt);
                new_stmts.extend(el.body.stmts.clone());
                el.body.stmts = new_stmts.clone();
            }
            _ => {
                abort_call_site!("`#[kani::loop_invariant]` is now only supported for while-loops.";
                    note = "for now, loop contracts is only supported for while-loops.";
                )
            }
        },
        _ => abort_call_site!("`#[kani::loop_invariant]` is now only supported for while-loops.";
            note = "for now, loop contracts is only supported for while-loops.";
        ),
    }
    let ret: TokenStream = if let Some(ForLoopExtraStmts {
        init_iter_stmt,
        init_len_stmt,
        init_index_stmt,
        init_pat_stmt,
        iter_assume_stmt,
    }) = for_loop_extras
    {
        if has_prev {
            quote!(
            {
            #init_iter_stmt
            #init_len_stmt
            assert!(kaniiterlen > 0, "undefined prev when iter's length is zero");
            {
            #init_index_stmt
            #iter_assume_stmt
            #init_pat_stmt
            #(#onentry_decl_stms)*
            #(#prev_decl_stms)*
            let mut #loop_body_closure = ||
            #loop_body;
            let (#loop_body_closure_ret_1, #loop_body_closure_ret_2) = #loop_body_closure ();
            if #loop_body_closure_ret_2.is_some() {
                return #loop_body_closure_ret_2.unwrap();
            }
            if #loop_body_closure_ret_1 {
            // Dummy function used to force the compiler to capture the environment.
            // We cannot call closures inside constant functions.
            // This function gets replaced by `kani::internal::call_closure`.
                #[inline(never)]
                #[kanitool::fn_marker = "kani_register_loop_contract"]
                const fn #register_ident<F: Fn() -> bool>(_f: &F, _transformed: usize) -> bool {
                    true
                }
                #loop_stmt
            }
            else {
                assert!(#inv_expr);
            };
            }
            })
            .into()
        } else {
            quote!(
            {
            #init_iter_stmt
            #init_len_stmt
            if kaniiterlen > 0
            {
            #init_index_stmt
            #iter_assume_stmt
            #init_pat_stmt
            #(#onentry_decl_stms)*
            // Dummy function used to force the compiler to capture the environment.
            // We cannot call closures inside constant functions.
            // This function gets replaced by `kani::internal::call_closure`.
            #[inline(never)]
            #[kanitool::fn_marker = "kani_register_loop_contract"]
            const fn #register_ident<F: Fn() -> bool>(_f: &F, _transformed: usize) -> bool {
                true
            }
            #loop_stmt
            }
            })
            .into()
        }
    } else if has_prev {
        quote!(
        {
        if (#loop_guard) {
        #(#onentry_decl_stms)*
        #(#prev_decl_stms)*
        let mut #loop_body_closure = ||
        #loop_body;
        let (#loop_body_closure_ret_1, #loop_body_closure_ret_2) = #loop_body_closure ();
        if #loop_body_closure_ret_2.is_some() {
            return #loop_body_closure_ret_2.unwrap();
        }
        if #loop_body_closure_ret_1 {
        // Dummy function used to force the compiler to capture the environment.
        // We cannot call closures inside constant functions.
        // This function gets replaced by `kani::internal::call_closure`.
            #[inline(never)]
            #[kanitool::fn_marker = "kani_register_loop_contract"]
            const fn #register_ident<F: Fn() -> bool>(_f: &F, _transformed: usize) -> bool {
                true
            }
            #loop_stmt
        }
        else {
            assert!(#inv_expr);
        };
        }
        })
        .into()
    } else {
        quote!(
        {
        #(#onentry_decl_stms)*
        // Dummy function used to force the compiler to capture the environment.
        // We cannot call closures inside constant functions.
        // This function gets replaced by `kani::internal::call_closure`.
        #[inline(never)]
        #[kanitool::fn_marker = "kani_register_loop_contract"]
        const fn #register_ident<F: Fn() -> bool>(_f: &F, _transformed: usize) -> bool {
            true
        }
        #loop_stmt
        })
        .into()
    };
    ret
}

fn generate_unique_id_from_span(stmt: &Stmt) -> String {
    // Extract the span of the expression
    let span = stmt.span().unwrap();

    // Get the start and end line and column numbers
    let start = span.start();
    let end = span.end();

    // Create a tuple of location information (file path, start line, start column, end line, end column)
    format!("_{:?}_{:?}_{:?}_{:?}", start.line(), start.column(), end.line(), end.column())
}

pub fn loop_modifies(attr: TokenStream, item: TokenStream) -> TokenStream {
    let assigns = parse_macro_input!(attr with Punctuated::<Expr, Token![,]>::parse_terminated)
        .into_iter()
        .collect::<Vec<Expr>>();
    let loop_assign_name: String = "kani_loop_modifies".to_owned();
    let loop_assign_ident = format_ident!("{}", loop_assign_name);
    let loop_assign_stmt: Stmt = parse_quote! {
        let #loop_assign_ident = (#(#assigns),*);
    };
    let loop_stmt: Stmt = syn::parse(item.clone()).unwrap();
    let ret: TokenStream = quote!(
    {
        #loop_assign_stmt
        #loop_stmt
    })
    .into();
    ret
}
