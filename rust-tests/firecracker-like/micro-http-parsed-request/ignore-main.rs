// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Example from Firecracker micro_http request handling
// https://github.com/firecracker-microvm/firecracker/commit/22908c9fb0cd5fb20febc5d18ff1284caa5f3a53

fn __nondet<T>() -> T {
    unimplemented!()
}

// Should return a nondet string of up to n characters
// Currently RMC does not support strings
fn __nondet_string(n: u32) -> String {
    unimplemented!()
}

// from 4e905f741
fn bug(n: u32) {
    let request_uri: String = __nondet_string(n);
    let _path_tokens: Vec<&str> = request_uri[1..].split_terminator('/').collect();
    //                                        ^ slice of empty string panics
}

// from 22908c9fb
fn fix(n: u32) {
    let request_uri: String = __nondet_string(n);
    let _path_tokens: Vec<&str> =
        request_uri.trim_start_matches('/').split_terminator('/').collect();
}

fn main() {
    let n: u32 = __nondet();
    bug(n);
    fix(n);
}
