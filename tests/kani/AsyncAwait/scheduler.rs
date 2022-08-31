// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    future::Future,
    pin::Pin,
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

/// Allows to parameterize how the scheduler picks the next task to poll in `spawnable_block_on`
pub trait SchedulingStrategy {
    /// Picks the next task to be scheduled whenever the scheduler needs to pick a task to run next, and whether it can be assumed that the picked task is still running
    ///
    /// Tasks are numbered `0..num_tasks`.
    /// For example, if pick_task(4) returns (2, true) than it picked the task with index 2 and allows Kani to `assume` that this task is still running.
    /// This is useful if the task is chosen nondeterministicall (`kani::any()`) and allows the verifier to discard useless execution branches (such as polling a completed task again).
    fn pick_task(&mut self, num_tasks: usize) -> (usize, bool);
}

impl<F: FnMut(usize) -> usize> SchedulingStrategy for F {
    fn pick_task(&mut self, num_tasks: usize) -> (usize, bool) {
        (self(num_tasks), false)
    }
}

/// Keeps cycling through the tasks in a deterministic order
#[derive(Default)]
pub struct RoundRobin {
    index: usize,
}

impl SchedulingStrategy for RoundRobin {
    fn pick_task(&mut self, num_tasks: usize) -> (usize, bool) {
        self.index = (self.index + 1) % num_tasks;
        (self.index, false)
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
                        let prev = std::mem::replace(task, None);
                    }
                    std::task::Poll::Pending => (),
                }
            } else if can_assume_running {
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
