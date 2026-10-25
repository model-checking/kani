// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A function-pointer *self* type in a contract target must fail to resolve
// cleanly (no ICE). `resolve_ty` builds a `FnPtr` for `fn(u8) -> u8`, so
// `<fn(u8) -> u8>::foo` reaches `resolve_in_user_type`, whose fallback arm must
// decline it rather than hit `unreachable!`.

#[kani::proof_for_contract(<fn(u8) -> u8>::foo)]
fn check_fn_ptr_self_type() {}
