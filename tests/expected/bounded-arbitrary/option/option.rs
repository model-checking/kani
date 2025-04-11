// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn check_option() {
    let my_option: Option<Vec<bool>> = kani::bounded_any::<_, 4>();
    kani::cover!(my_option.is_none());
    if let Some(inner) = my_option {
        kani::cover!(inner.len() == 0);
        kani::cover!(inner.len() == 1);
        kani::cover!(inner.len() == 2);
        kani::cover!(inner.len() == 3);
        kani::cover!(inner.len() == 4);
    }
}
