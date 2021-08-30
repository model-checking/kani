// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This is a regression test for size_and_align_of_dst computing the
// size and alignment of a dynamically-sized type like
// Arc<Mutex<dyn Subscriber>>.

// https://github.com/model-checking/rmc/issues/426
// Current RMC-time compiler panic in implementing drop_in_place:

// thread 'rustc' panicked at 'Function call does not type check:
// "func":"Expr"{
//     "value":"Symbol"{
//        "identifier":"_RINvNtCsgWci0eQkB8o_4core3ptr13drop_in_placeNtNtNtCs1HAdiQHUxxm_3std10sys_common5mutex12MovableMutexECsb7rQPrKk64Y_4main"
//     },
//     "typ":"Code"{
//        "parameters":[
//           "Parameter"{
//              "typ":"Pointer"{
//                 "typ":"StructTag(""tag-std::sys_common::mutex::MovableMutex"")"
//              },
//           }
//        ],
//        "return_type":"StructTag(""tag-Unit"")"
//     },
//  }"args":[
//     "Expr"{
//        "value":"Symbol"{
//           "identifier":"_RINvNtCsgWci0eQkB8o_4core3ptr13drop_in_placeINtNtNtCs1HAdiQHUxxm_3std4sync5mutex5MutexDNtCsb7rQPrKk64Y_4main10SubscriberEL_EEB1p_::1::var_1"
//        },
//        "typ":"StructTag(""tag-dyn Subscriber"")",
//     }
//  ]"', compiler/rustc_codegen_llvm/src/gotoc/cbmc/goto_program/expr.rs:532:9"

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;

pub trait Subscriber {
    fn process(&mut self);
    fn interest_list(&self);
}

struct DummySubscriber {}

impl DummySubscriber {
    fn new() -> Self {
        DummySubscriber {}
    }
}

impl Subscriber for DummySubscriber {
    fn process(&mut self) {}
    fn interest_list(&self) {}
}

fn main() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
}
