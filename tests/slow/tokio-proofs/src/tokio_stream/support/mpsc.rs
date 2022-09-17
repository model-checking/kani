// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright tokio Contributors
// origin: tokio-stream/tests/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23fsupport/

use async_stream::stream;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::Stream;

pub fn unbounded_channel_stream<T: Unpin>() -> (UnboundedSender<T>, impl Stream<Item = T>) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let stream = stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
    };

    (tx, stream)
}
