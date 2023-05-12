// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

//! Regression test for https://github.com/model-checking/kani/issues/2434
//! The problem was an incorrect order for the operands
use core::{future::Future, pin::Pin};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Sync + 'static>>;

pub struct Scheduler {
    task: Option<BoxFuture>,
}

impl Scheduler {
    /// Adds a future to the scheduler's task list, returning a JoinHandle
    pub fn spawn<F: Future<Output = ()> + Sync + 'static>(&mut self, fut: F) {
        self.task = Some(Box::pin(fut));
    }
}

/// Polls the given future and the tasks it may spawn until all of them complete
///
/// Contrary to block_on, this allows `spawn`ing other futures
pub fn spawnable_block_on<F: Future<Output = ()> + Sync + 'static>(
    scheduler: &mut Scheduler,
    fut: F,
) {
    scheduler.spawn(fut);
}

/// Sender of a channel.
pub struct Sender {}

impl Sender {
    pub async fn send(&self) {}
}

#[kani::proof]
fn check() {
    let mut scheduler = Scheduler { task: None };
    spawnable_block_on(&mut scheduler, async {
        let num: usize = 1;
        let tx = Sender {};

        let _task1 = async move {
            for _i in 0..num {
                tx.send().await;
            }
        };
    });
}
