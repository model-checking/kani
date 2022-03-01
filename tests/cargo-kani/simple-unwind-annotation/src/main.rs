// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-unwinding-checks

// The expected file presently looks for "1 == 2" above.
// But eventually this test may start to fail as we might stop regarding 'main'
// as a valid proof harness, since it isn't annotated as such.
// This test should be updated if we go that route.

fn main() {
    assert!(1 == 2);
}

#[kani::proof]
#[kani::unwind(10)]
fn harness() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}

#[kani::proof]
fn harness_2() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}

// NOTE: These are potentially all scenarios that produce user errors. Uncomment each harness to test how the user error
// looks like.

// #[kani::proof]
// #[kani::unwind(10,5)]
// fn harness_3() {
//     let mut counter = 0;
//     loop {
//         counter += 1;
//         assert!(counter < 10);
//     }
// }

// #[kani::unwind(8)]
// fn harness_4() {
//     let mut counter = 0;
//     for i in 0..7 {
//         counter += 1;
//         assert!(counter < 5);
//     }
// }

// #[kani::proof]
// #[kani::proof]
// fn harness_5() {
//     let mut counter = 0;
//     loop {
//         counter += 1;
//         assert!(counter < 10);
//     }
// }

// #[kani::proof(some, argument2)]
// fn harness_6() {
//     let mut counter = 0;
//     loop {
//         counter += 1;
//         assert!(counter < 10);
//     }
// }

// // #[kani::unwind(9)]
// // fn harness_7() {
// //     let mut counter = 0;
// //     for i in 0..10 {
// //         counter += 1;
// //         assert!(counter < 8);
// //     }
// // }
