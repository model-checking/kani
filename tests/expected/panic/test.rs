// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check_panic() {
    let msg = "Panic message";
    match kani::any::<u8>() {
        0 => panic!(),
        1 => panic!("Panic message"),
        2 => panic!("Panic message with arg {}", "str"),
        3 => panic!("{}", msg),
        4 => panic!(msg),
        _ => panic!(concat!("Panic: {} code: ", 10), msg),
    }
}

macro_rules! panic_oob {
    ($method_name:expr, $index:expr, $len:expr) => {
        panic!(
            concat!(
                "ArrayVec::",
                $method_name,
                ": index {} is out of bounds in vector of length {}"
            ),
            $index, $len
        )
    };
}
