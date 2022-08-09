// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright tokio Contributors
// origin: tokio-stream/tests/

use tokio_stream::{Stream, StreamExt};

use std::pin::Pin;
use std::task::{Context, Poll};

// a stream which alternates between Some and None
struct Alternate {
    state: i32,
}

impl Stream for Alternate {
    type Item = i32;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<i32>> {
        let val = self.state;
        self.state += 1;

        // if it's even, Some(i32), else None
        if val % 2 == 0 { Poll::Ready(Some(val)) } else { Poll::Ready(None) }
    }
}

#[kani::proof]
#[kani::unwind(32)]
async fn basic_usage_fuse() {
    let mut stream = Alternate { state: 0 };

    // the stream goes back and forth
    assert_eq!(stream.next().await, Some(0));
    assert_eq!(stream.next().await, None);
    assert_eq!(stream.next().await, Some(2));
    assert_eq!(stream.next().await, None);

    // however, once it is fused
    let mut stream = stream.fuse();

    assert_eq!(stream.size_hint(), (0, None));
    assert_eq!(stream.next().await, Some(4));

    assert_eq!(stream.size_hint(), (0, None));
    assert_eq!(stream.next().await, None);

    // it will always return `None` after the first time.
    assert_eq!(stream.size_hint(), (0, Some(0)));
    assert_eq!(stream.next().await, None);
    assert_eq!(stream.size_hint(), (0, Some(0)));
}
