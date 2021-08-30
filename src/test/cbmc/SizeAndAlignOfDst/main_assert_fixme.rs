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

use std::sync::Arc;
use std::sync::Mutex;

pub trait Subscriber {
    fn process(&self);
    fn increment(&mut self);
    fn get(&self) -> u32;
}

struct DummySubscriber {
    val: u32,
}

impl DummySubscriber {
    fn new() -> Self {
        DummySubscriber { val: 0 }
    }
}

impl Subscriber for DummySubscriber {
    fn process(&self) {}
    fn increment(&mut self) {
        self.val = self.val + 1;
    }
    fn get(&self) -> u32 {
        self.val
    }
}

fn main() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
    let mut data = s.lock().unwrap();
    data.increment();
    assert!(data.get() == 1);
}
