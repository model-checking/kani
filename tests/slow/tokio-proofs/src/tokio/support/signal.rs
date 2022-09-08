// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Original copyright tokio contributors.
// origin: tokio/tests/support

pub fn send_signal(signal: libc::c_int) {
    use libc::{getpid, kill};

    unsafe {
        assert_eq!(kill(getpid(), signal), 0);
    }
}
