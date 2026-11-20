// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn check_result() {
    let my_result: Result<Vec<bool>, Vec<u8>> = kani::bounded_any::<_, 4>();
    match my_result {
        Ok(inner_vec) => {
            kani::cover!(inner_vec.len() == 0);
            kani::cover!(inner_vec.len() == 1);
            kani::cover!(inner_vec.len() == 2);
            kani::cover!(inner_vec.len() == 3);
            kani::cover!(inner_vec.len() == 4);
        }
        Err(inner_vec) => {
            kani::cover!(inner_vec.len() == 0);
            kani::cover!(inner_vec.len() == 1);
            kani::cover!(inner_vec.len() == 2);
            kani::cover!(inner_vec.len() == 3);
            kani::cover!(inner_vec.len() == 4);
        }
    }
}
