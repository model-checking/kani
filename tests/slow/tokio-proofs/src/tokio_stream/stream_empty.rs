// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright tokio Contributors
// origin: tokio-stream/tests/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

use tokio_stream::{self as stream, Stream, StreamExt};

#[kani::proof]
#[kani::unwind(3)]
async fn basic_usage_empty() {
    let mut stream = stream::empty::<i32>();

    for _ in 0..2 {
        assert_eq!(stream.size_hint(), (0, Some(0)));
        assert_eq!(None, stream.next().await);
    }
}
