// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains functions to work with futures (and async/.await) in Kani.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

/// A very simple executor: it polls the future in a busy loop until completion
///
/// This is intended as a drop-in replacement for `futures::block_on`, which Kani cannot handle.
/// Whereas a clever executor like `block_on` in `futures` or `tokio` would interact with the OS scheduler
/// to be woken up when a resource becomes available, this is not supported by Kani.
/// As a consequence, this function completely ignores the waker infrastructure and just polls the given future in a busy loop.
///
/// Note that spawn is not supported with this function. Use [`spawnable_block_on`] if you need it.
// TODO: Give an error if spawn is used in the future passed to this function.
pub fn block_on<T>(mut fut: impl Future<Output = T>) -> T {
    let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
    let cx = &mut Context::from_waker(&waker);
    // SAFETY: we shadow the original binding, so it cannot be accessed again for the rest of the scope.
    // This is the same as what the pin_mut! macro in the futures crate does.
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    loop {
        match fut.as_mut().poll(cx) {
            std::task::Poll::Ready(res) => return res,
            std::task::Poll::Pending => continue,
        }
    }
}

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

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Sync + 'static>>;

/// Indicates to the scheduler whether it can `kani::assume` that the returned task is running.
///
/// This is useful if the task was picked nondeterministically using `kani::any()`.
/// For more information, see [`SchedulingStrategy`].
pub enum SchedulingAssumption {
    CanAssumeRunning,
    CannotAssumeRunning,
}

/// Trait that determines the possible sequence of tasks scheduling for a harness.
///
/// If your harness spawns several tasks, Kani's scheduler has to decide in what order to poll them.
/// This order may depend on the needs of your verification goal.
/// For example, you sometimes may wish to verify all possible schedulings, i.e. a nondeterministic scheduling strategy,
/// provided by [`NondeterministicScheduling`].
///
/// However, this one may poll the same task over and over, which is often undesirable.
/// To ensure some "fairness" in how the tasks are picked, there is [`NondetFairScheduling`].
/// This is probably what you want when verifying a harness under nondeterministic schedulings.
///
/// Nondeterministic scheduling strategies can be very slow to verify because they require Kani to check a large number of permutations of tasks.
/// So if you want to verify a harness that uses `spawn`, but don't care about concurrency issues, you can simply use a deterministic scheduling strategy,
/// such as [`RoundRobin`], which polls each task in turn.
///
/// Finally, you have the option of providing your own scheduling strategy by implementing this trait.
/// This can be useful, for example, if you want to verify that things work correctly for a very specific task ordering.
pub trait SchedulingStrategy {
    /// Picks the next task to be scheduled whenever the scheduler needs to pick a task to run next, and whether it can be assumed that the picked task is still running
    ///
    /// Tasks are numbered `0..num_tasks`.
    /// For example, if pick_task(4) returns (2, CanAssumeRunning) than it picked the task with index 2 and allows Kani to `assume` that this task is still running.
    /// This is useful if the task is chosen nondeterministicall (`kani::any()`) and allows the verifier to discard useless execution branches (such as polling a completed task again).
    ///
    /// As a rule of thumb:
    /// if the scheduling strategy picks the next task nondeterministically (using `kani::any()`), return CanAssumeRunning, otherwise CannotAssumeRunning.
    /// When returning `CanAssumeRunning`, the scheduler will then assume that the picked task is still running, which cuts off "useless" paths where a completed task is polled again.
    /// It is even necessary to make things terminate if nondeterminism is involved:
    /// if we pick the task nondeterministically, and don't have the restriction to still running tasks, we could poll the same task over and over again.
    ///
    /// However, for most deterministic scheduling strategies, e.g. the round robin scheduling strategy, assuming that the picked task is still running is generally not possible
    /// because if that task has ended, we are saying assume(false) and the verification effectively stops (which is undesirable, of course).
    /// In such cases, return `CannotAssumeRunning` instead.
    fn pick_task(&mut self, num_tasks: usize) -> (usize, SchedulingAssumption);
}

/// Keeps cycling through the tasks in a deterministic order
#[derive(Default)]
pub struct RoundRobin {
    index: usize,
}

impl SchedulingStrategy for RoundRobin {
    #[inline]
    fn pick_task(&mut self, num_tasks: usize) -> (usize, SchedulingAssumption) {
        self.index = (self.index + 1) % num_tasks;
        (self.index, SchedulingAssumption::CannotAssumeRunning)
    }
}

/// Picks the next task nondeterministically
#[derive(Default)]
pub struct NondeterministicScheduling;

impl SchedulingStrategy for NondeterministicScheduling {
    fn pick_task(&mut self, num_tasks: usize) -> (usize, SchedulingAssumption) {
        let index = crate::any();
        crate::assume(index < num_tasks);
        (index, SchedulingAssumption::CanAssumeRunning)
    }
}

/// A restricted form of nondeterministic scheduling to have some fairness.
///
/// Each task has a counter that is increased when it is scheduled.
/// If a task has reached a provided limit, it cannot be scheduled anymore until all other tasks have reached the limit too,
/// at which point all the counters are reset to zero.
pub struct NondetFairScheduling {
    counters: Vec<u8>,
    limit: u8,
}

impl NondetFairScheduling {
    #[inline]
    pub fn new(limit: u8) -> Self {
        Self { counters: Vec::new(), limit }
    }
}

impl SchedulingStrategy for NondetFairScheduling {
    fn pick_task(&mut self, num_tasks: usize) -> (usize, SchedulingAssumption) {
        if self.counters.len() != num_tasks || self.counters.iter().all(|x| *x == 0) {
            self.counters = vec![self.limit; num_tasks];
        }
        let index = crate::any();
        crate::assume(index < num_tasks);
        crate::assume(self.counters[index] > 0);
        self.counters[index] -= 1;
        (index, SchedulingAssumption::CanAssumeRunning)
    }
}

pub(crate) struct Scheduler {
    tasks: Vec<Option<BoxFuture>>,
    num_running: usize,
}

impl Scheduler {
    /// Creates a scheduler with an empty task list
    #[inline]
    pub(crate) const fn new() -> Scheduler {
        const INIT: Option<BoxFuture> = None;
        Scheduler { tasks: Vec::new(), num_running: 0 }
    }

    /// Adds a future to the scheduler's task list, returning a JoinHandle
    pub(crate) fn spawn<F: Future<Output = ()> + Sync + 'static>(&mut self, fut: F) -> JoinHandle {
        let index = self.tasks.len();
        self.tasks.push(Some(Box::pin(fut)));
        self.num_running += 1;
        JoinHandle { index }
    }

    /// Runs the scheduler with the given scheduling plan until all tasks have completed
    fn run(&mut self, mut scheduling_plan: impl SchedulingStrategy) {
        let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
        let cx = &mut Context::from_waker(&waker);
        while self.num_running > 0 {
            let (index, assumption) = scheduling_plan.pick_task(self.tasks.len());
            let task = &mut self.tasks[index];
            if let Some(fut) = task.as_mut() {
                match fut.as_mut().poll(cx) {
                    std::task::Poll::Ready(()) => {
                        self.num_running -= 1;
                        let _prev = task.take();
                    }
                    std::task::Poll::Pending => (),
                }
            } else if let SchedulingAssumption::CanAssumeRunning = assumption {
                crate::assume(false); // useful so that we can assume that a nondeterministically picked task is still running
            }
        }
    }

    /// Polls the given future and the tasks it may spawn until all of them complete
    fn block_on<F: Future<Output = ()> + Sync + 'static>(
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

pub fn spawn<F: Future<Output = ()> + Sync + 'static>(fut: F) -> JoinHandle {
    unsafe { GLOBAL_EXECUTOR.spawn(fut) }
}

/// Polls the given future and the tasks it may spawn until all of them complete
///
/// Contrary to [`block_on`], this allows `spawn`ing other futures
pub fn spawnable_block_on<F: Future<Output = ()> + Sync + 'static>(
    fut: F,
    scheduling_plan: impl SchedulingStrategy,
) {
    unsafe {
        GLOBAL_EXECUTOR.block_on(fut, scheduling_plan);
    }
}

/// Suspends execution of the current future, to allow the scheduler to poll another future
///
/// Specifically, it returns a future that
pub fn yield_now() -> impl Future<Output = ()> {
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

    YieldNow { yielded: false }
}
