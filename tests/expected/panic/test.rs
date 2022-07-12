// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check_panic() {
    let msg = "Panic message";
    match kani::any::<u8>() {
        0 => panic!(),
        1 => panic!("Panic message"),
        2 => panic!("Panic message with arg {}", "str"),
        3 => panic!("{}", msg),
        _ => panic!(msg),
    }
}
