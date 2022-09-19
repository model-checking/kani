// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

//! This file tests a hand-written spawn infrastructure and executor.
//! This should be replaced with code from the Kani library as soon as the executor can get merged.
//! Tracking issue: https://github.com/model-checking/kani/issues/1685

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

/// A dummy waker, which is needed to call [`Future::poll`]
const NOOP_RAW_WAKER: RawWaker = {
    #[inline]
    unsafe fn clone_waker(_: *const ()) -> RawWaker {
        NOOP_RAW_WAKER
    }

    #[inline]
    unsafe fn noop(_: *const ()) {}

    RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone_waker, noop, noop, noop))
};

static mut GLOBAL_EXECUTOR: Scheduler = Scheduler::new();
const MAX_TASKS: usize = 16;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Sync + 'static>>;

/// Indicates to the scheduler whether it can `assume` that the returned task is running.
/// This is useful if the task was picked nondeterministically using `any()`.
pub enum SchedulingOptimization {
    CanAssumeRunning,
    CannotAssumeRunning,
}

/// Allows to parameterize how the scheduler picks the next task to poll in `spawnable_block_on`
pub trait SchedulingStrategy {
    /// Picks the next task to be scheduled whenever the scheduler needs to pick a task to run next, and whether it can be assumed that the picked task is still running
    ///
    /// Tasks are numbered `0..num_tasks`.
    /// For example, if pick_task(4) returns `(2, CanAssumeRunning)` than it picked the task with index 2 and allows Kani to `assume` that this task is still running.
    /// This is useful if the task is chosen nondeterministicall (`kani::any()`) and allows the verifier to discard useless execution branches (such as polling a completed task again).
    fn pick_task(&mut self, num_tasks: usize) -> (usize, SchedulingOptimization);
}

/// Keeps cycling through the tasks in a deterministic order
#[derive(Default)]
pub struct RoundRobin {
    index: usize,
}

impl SchedulingStrategy for RoundRobin {
    fn pick_task(&mut self, num_tasks: usize) -> (usize, SchedulingOptimization) {
        self.index = (self.index + 1) % num_tasks;
        (self.index, SchedulingOptimization::CannotAssumeRunning)
    }
}

pub struct Scheduler {
    /// Using a Vec instead of an array makes the runtime increase by a factor of 200.
    tasks: [Option<BoxFuture>; MAX_TASKS],
    num_tasks: usize,
    num_running: usize,
}

impl Scheduler {
    /// Creates a scheduler with an empty task list
    pub const fn new() -> Scheduler {
        const INIT: Option<BoxFuture> = None;
        Scheduler { tasks: [INIT; MAX_TASKS], num_tasks: 0, num_running: 0 }
    }

    /// Adds a future to the scheduler's task list, returning a JoinHandle
    pub fn spawn<F: Future<Output = ()> + Sync + 'static>(&mut self, fut: F) -> JoinHandle {
        let index = self.num_tasks;
        self.tasks[index] = Some(Box::pin(fut));
        self.num_tasks += 1;
        assert!(self.num_tasks < MAX_TASKS, "more than {} tasks", MAX_TASKS);
        self.num_running += 1;
        JoinHandle { index }
    }

    /// Runs the scheduler with the given scheduling plan until all tasks have completed
    pub fn run(&mut self, mut scheduling_plan: impl SchedulingStrategy) {
        let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
        let cx = &mut Context::from_waker(&waker);
        while self.num_running > 0 {
            let (index, can_assume_running) = scheduling_plan.pick_task(self.num_tasks);
            let task = &mut self.tasks[index];
            if let Some(fut) = task.as_mut() {
                match fut.as_mut().poll(cx) {
                    std::task::Poll::Ready(()) => {
                        self.num_running -= 1;
                        let _prev = std::mem::replace(task, None);
                    }
                    std::task::Poll::Pending => (),
                }
            } else if let SchedulingOptimization::CanAssumeRunning = can_assume_running {
                #[cfg(kani)]
                kani::assume(false); // useful so that we can assume that a nondeterministically picked task is still running
            }
        }
    }

    /// Polls the given future and the tasks it may spawn until all of them complete.
    pub fn block_on<F: Future<Output = ()> + Sync + 'static>(
        &mut self,
        fut: F,
        scheduling_plan: impl SchedulingStrategy,
    ) {
        self.spawn(fut);
        self.run(scheduling_plan);
    }
}

/// Result of spawning a task.
///
/// If you `.await` a JoinHandle, this will wait for the spawned task to complete.
pub struct JoinHandle {
    index: usize,
}

impl Future for JoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        if unsafe { GLOBAL_EXECUTOR.tasks[self.index].is_some() } {
            std::task::Poll::Pending
        } else {
            cx.waker().wake_by_ref(); // For completeness. But Kani currently ignores wakers.
            std::task::Poll::Ready(())
        }
    }
}

#[inline] // to work around linking issue
pub fn spawn<F: Future<Output = ()> + Sync + 'static>(fut: F) -> JoinHandle {
    unsafe { GLOBAL_EXECUTOR.spawn(fut) }
}

/// Polls the given future and the tasks it may spawn until all of them complete
///
/// Contrary to block_on, this allows `spawn`ing other futures
pub fn spawnable_block_on<F: Future<Output = ()> + Sync + 'static>(
    fut: F,
    scheduling_plan: impl SchedulingStrategy,
) {
    unsafe {
        GLOBAL_EXECUTOR.block_on(fut, scheduling_plan);
    }
}

struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        if self.yielded {
            cx.waker().wake_by_ref(); // For completeness. But Kani currently ignores wakers.
            std::task::Poll::Ready(())
        } else {
            self.yielded = true;
            std::task::Poll::Pending
        }
    }
}

/// Suspends execution of the current future, to allow the scheduler to poll another future
pub fn yield_now() -> impl Future<Output = ()> {
    YieldNow { yielded: false }
}

#[kani::proof]
#[kani::unwind(4)]
fn arc_spawn_deterministic_test() {
    let x = Arc::new(AtomicI64::new(0)); // Surprisingly, Arc verified faster than Rc
    let x2 = x.clone();
    spawnable_block_on(
        async move {
            let x3 = x2.clone();
            spawn(async move {
                x3.fetch_add(1, Ordering::Relaxed);
            });
            yield_now().await;
            x2.fetch_add(1, Ordering::Relaxed);
        },
        RoundRobin::default(),
    );
    assert_eq!(x.load(Ordering::Relaxed), 2);
}
