//-
// Copyright 2017, 2018, 2019 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

use crate::std_facade::{Arc, BTreeMap, Box, String, Vec};
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::SeqCst;
use core::{fmt, iter};
#[cfg(feature = "std")]
use std::panic::{self, AssertUnwindSafe};

#[cfg(feature = "fork")]
use std::cell::{Cell, RefCell};
#[cfg(feature = "fork")]
use std::env;
#[cfg(feature = "fork")]
use std::fs;

use crate::strategy::*;
use crate::test_runner::config::*;
use crate::test_runner::errors::*;
use crate::test_runner::failure_persistence::PersistedSeed;
use crate::test_runner::reason::*;
#[cfg(feature = "fork")]
use crate::test_runner::replay;
use crate::test_runner::result_cache::*;
use crate::test_runner::rng::TestRng;

#[cfg(feature = "fork")]
const ENV_FORK_FILE: &'static str = "_PROPTEST_FORKFILE";

const ALWAYS: u32 = 0;
const SHOW_FALURES: u32 = 1;
const TRACE: u32 = 2;

#[cfg(feature = "std")]
macro_rules! verbose_message {
    ($runner:expr, $level:expr, $fmt:tt $($arg:tt)*) => { {
        #[allow(unused_comparisons)]
        {
            if $runner.config.verbose >= $level {
                eprintln!(concat!("proptest: ", $fmt) $($arg)*);
            }
        };
        ()
    } }
}

#[cfg(not(feature = "std"))]
macro_rules! verbose_message {
    ($runner:expr, $level:expr, $fmt:tt $($arg:tt)*) => {
        let _ = $level;
    };
}

type RejectionDetail = BTreeMap<Reason, u32>;

/// State used when running a proptest test.
#[derive(Clone)]
pub struct TestRunner {
    config: Config,
    successes: u32,
    local_rejects: u32,
    global_rejects: u32,
    rng: TestRng,
    flat_map_regens: Arc<AtomicUsize>,

    local_reject_detail: RejectionDetail,
    global_reject_detail: RejectionDetail,
}

impl fmt::Debug for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TestRunner")
            .field("config", &self.config)
            .field("successes", &self.successes)
            .field("local_rejects", &self.local_rejects)
            .field("global_rejects", &self.global_rejects)
            .field("rng", &"<TestRng>")
            .field("flat_map_regens", &self.flat_map_regens)
            .field("local_reject_detail", &self.local_reject_detail)
            .field("global_reject_detail", &self.global_reject_detail)
            .finish()
    }
}

impl fmt::Display for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\tsuccesses: {}\n\
             \tlocal rejects: {}\n",
            self.successes, self.local_rejects
        )?;
        for (whence, count) in &self.local_reject_detail {
            writeln!(f, "\t\t{} times at {}", count, whence)?;
        }
        writeln!(f, "\tglobal rejects: {}", self.global_rejects)?;
        for (whence, count) in &self.global_reject_detail {
            writeln!(f, "\t\t{} times at {}", count, whence)?;
        }

        Ok(())
    }
}

/// Equivalent to: `TestRunner::new(Config::default())`.
impl Default for TestRunner {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

#[cfg(feature = "fork")]
#[derive(Debug)]
struct ForkOutput {
    file: Option<fs::File>,
}

#[cfg(feature = "fork")]
impl ForkOutput {
    fn append(&mut self, result: &TestCaseResult) {
        if let Some(ref mut file) = self.file {
            replay::append(file, result)
                .expect("Failed to append to replay file");
        }
    }

    fn ping(&mut self) {
        if let Some(ref mut file) = self.file {
            replay::ping(file).expect("Failed to append to replay file");
        }
    }

    fn terminate(&mut self) {
        if let Some(ref mut file) = self.file {
            replay::terminate(file).expect("Failed to append to replay file");
        }
    }

    fn empty() -> Self {
        ForkOutput { file: None }
    }

    fn is_in_fork(&self) -> bool {
        self.file.is_some()
    }
}

#[cfg(not(feature = "fork"))]
#[derive(Debug)]
struct ForkOutput;

#[cfg(not(feature = "fork"))]
impl ForkOutput {
    fn append(&mut self, _result: &TestCaseResult) {}
    fn ping(&mut self) {}
    fn terminate(&mut self) {}
    fn empty() -> Self {
        ForkOutput
    }
    fn is_in_fork(&self) -> bool {
        false
    }
}

#[cfg(not(feature = "std"))]
fn call_test<V, F, R>(
    _runner: &mut TestRunner,
    case: V,
    test: &F,
    replay: &mut R,
    result_cache: &mut dyn ResultCache,
    _: &mut ForkOutput,
) -> TestCaseResult
where
    V: fmt::Debug,
    F: Fn(V) -> TestCaseResult,
    R: Iterator<Item = TestCaseResult>,
{
    if let Some(result) = replay.next() {
        return result;
    }

    let cache_key = result_cache.key(&ResultCacheKey::new(&case));
    if let Some(result) = result_cache.get(cache_key) {
        return result.clone();
    }

    let result = test(case);
    result_cache.put(cache_key, &result);
    result
}

#[cfg(feature = "std")]
fn call_test<V, F, R>(
    runner: &mut TestRunner,
    case: V,
    test: &F,
    replay: &mut R,
    result_cache: &mut dyn ResultCache,
    fork_output: &mut ForkOutput,
) -> TestCaseResult
where
    V: fmt::Debug,
    F: Fn(V) -> TestCaseResult,
    R: Iterator<Item = TestCaseResult>,
{
    use std::time;

    let timeout = runner.config.timeout();

    if let Some(result) = replay.next() {
        return result;
    }

    // Now that we're about to start a new test (as far as the replay system is
    // concerned), ping the replay file so the parent process can determine
    // that we made it this far.
    fork_output.ping();

    verbose_message!(runner, TRACE, "Next test input: {:?}", case);

    let cache_key = result_cache.key(&ResultCacheKey::new(&case));
    if let Some(result) = result_cache.get(cache_key) {
        verbose_message!(
            runner,
            TRACE,
            "Test input hit cache, skipping execution"
        );
        return result.clone();
    }

    let time_start = time::Instant::now();

    let mut result = unwrap_or!(
        panic::catch_unwind(AssertUnwindSafe(|| test(case))),
        what => Err(TestCaseError::Fail(
            what.downcast::<&'static str>().map(|s| (*s).into())
                .or_else(|what| what.downcast::<String>().map(|b| (*b).into()))
                .or_else(|what| what.downcast::<Box<str>>().map(|b| (*b).into()))
                .unwrap_or_else(|_| "<unknown panic value>".into()))));

    // If there is a timeout and we exceeded it, fail the test here so we get
    // consistent behaviour. (The parent process cannot precisely time the test
    // cases itself.)
    if timeout > 0 && result.is_ok() {
        let elapsed = time_start.elapsed();
        let elapsed_millis = elapsed.as_secs() as u32 * 1000
            + elapsed.subsec_nanos() / 1_000_000;

        if elapsed_millis > timeout {
            result = Err(TestCaseError::fail(format!(
                "Timeout of {} ms exceeded: test took {} ms",
                timeout, elapsed_millis
            )));
        }
    }

    result_cache.put(cache_key, &result);
    fork_output.append(&result);

    match result {
        Ok(()) => verbose_message!(runner, TRACE, "Test case passed"),
        Err(TestCaseError::Reject(ref reason)) => verbose_message!(
            runner,
            SHOW_FALURES,
            "Test case rejected: {}",
            reason
        ),
        Err(TestCaseError::Fail(ref reason)) => verbose_message!(
            runner,
            SHOW_FALURES,
            "Test case failed: {}",
            reason
        ),
    }

    result
}

type TestRunResult<S> = Result<(), TestError<<S as Strategy>::Value>>;

impl TestRunner {
    /// Create a fresh `TestRunner` with the given configuration.
    ///
    /// The runner will use an RNG with a generated seed and the default
    /// algorithm.
    ///
    /// In `no_std` environments, every `TestRunner` will use the same
    /// hard-coded seed. This seed is not contractually guaranteed and may be
    /// changed between releases without notice.
    pub fn new(config: Config) -> Self {
        let algorithm = config.rng_algorithm;
        TestRunner::new_with_rng(config, TestRng::default_rng(algorithm))
    }

    /// Create a fresh `TestRunner` with the standard deterministic RNG.
    ///
    /// This is sugar for the following:
    ///
    /// ```rust
    /// # use proptest::test_runner::*;
    /// let config = Config::default();
    /// let algorithm = config.rng_algorithm;
    /// TestRunner::new_with_rng(
    ///     config,
    ///     TestRng::deterministic_rng(algorithm));
    /// ```
    ///
    /// Refer to `TestRng::deterministic_rng()` for more information on the
    /// properties of the RNG used here.
    pub fn deterministic() -> Self {
        let config = Config::default();
        let algorithm = config.rng_algorithm;
        TestRunner::new_with_rng(config, TestRng::deterministic_rng(algorithm))
    }

    /// Create a fresh `TestRunner` with the given configuration and RNG.
    pub fn new_with_rng(config: Config, rng: TestRng) -> Self {
        TestRunner {
            config: config,
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: rng,
            flat_map_regens: Arc::new(AtomicUsize::new(0)),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
        }
    }

    /// Create a fresh `TestRunner` with the same config and global counters as
    /// this one, but with local state reset and an independent `Rng` (but
    /// deterministic).
    pub(crate) fn partial_clone(&mut self) -> Self {
        TestRunner {
            config: self.config.clone(),
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: self.new_rng(),
            flat_map_regens: Arc::clone(&self.flat_map_regens),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
        }
    }

    /// Returns the RNG for this test run.
    pub fn rng(&mut self) -> &mut TestRng {
        &mut self.rng
    }

    /// Create a new, independent but deterministic RNG from the RNG in this
    /// runner.
    pub fn new_rng(&mut self) -> TestRng {
        self.rng.gen_rng()
    }

    /// Returns the configuration of this runner.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Dumps the bytes obtained from the RNG so far (only works if the RNG is
    /// set to `Recorder`).
    ///
    /// ## Panics
    ///
    /// Panics if the RNG does not capture generated data.
    pub fn bytes_used(&self) -> Vec<u8> {
        self.rng.bytes_used()
    }

    /// Run test cases against `f`, choosing inputs via `strategy`.
    ///
    /// If any failure cases occur, try to find a minimal failure case and
    /// report that. If invoking `f` panics, the panic is turned into a
    /// `TestCaseError::Fail`.
    ///
    /// If failure persistence is enabled, all persisted failing cases are
    /// tested first. If a later non-persisted case fails, its seed is
    /// persisted before returning failure.
    ///
    /// Returns success or failure indicating why the test as a whole failed.
    pub fn run<S: Strategy>(
        &mut self,
        strategy: &S,
        test: impl Fn(S::Value) -> TestCaseResult,
    ) -> TestRunResult<S> {
        if self.config.fork() {
            self.run_in_fork(strategy, test)
        } else {
            self.run_in_process(strategy, test)
        }
    }

    #[cfg(not(feature = "fork"))]
    fn run_in_fork<S: Strategy>(
        &mut self,
        _: &S,
        _: impl Fn(S::Value) -> TestCaseResult,
    ) -> TestRunResult<S> {
        unreachable!()
    }

    #[cfg(feature = "fork")]
    fn run_in_fork<S: Strategy>(
        &mut self,
        strategy: &S,
        test: impl Fn(S::Value) -> TestCaseResult,
    ) -> TestRunResult<S> {
        unreachable!()
    }

    fn run_in_process<S: Strategy>(
        &mut self,
        strategy: &S,
        test: impl Fn(S::Value) -> TestCaseResult,
    ) -> TestRunResult<S> {
        let (replay_steps, fork_output) = init_replay(&mut self.rng);
        self.run_in_process_with_replay(
            strategy,
            test,
            replay_steps.into_iter(),
            fork_output,
        )
    }

    fn run_in_process_with_replay<S: Strategy>(
        &mut self,
        strategy: &S,
        test: impl Fn(S::Value) -> TestCaseResult,
        mut replay: impl Iterator<Item = TestCaseResult>,
        mut fork_output: ForkOutput,
    ) -> TestRunResult<S> {
        let old_rng = self.rng.clone();

        let persisted_failure_seeds: Vec<PersistedSeed> = self
            .config
            .failure_persistence
            .as_ref()
            .map(|f| f.load_persisted_failures2(self.config.source_file))
            .unwrap_or_default();

        let mut result_cache = self.new_cache();

        for PersistedSeed(persisted_seed) in persisted_failure_seeds {
            self.rng.set_seed(persisted_seed);
            self.gen_and_run_case(
                strategy,
                &test,
                &mut replay,
                &mut *result_cache,
                &mut fork_output,
            )?;
        }
        self.rng = old_rng;

        while self.successes < self.config.cases {
            // Generate a new seed and make an RNG from that so that we know
            // what seed to persist if this case fails.
            let seed = self.rng.gen_get_seed();
            let result = self.gen_and_run_case(
                strategy,
                &test,
                &mut replay,
                &mut *result_cache,
                &mut fork_output,
            );
            if let Err(TestError::Fail(_, ref value)) = result {
                if let Some(ref mut failure_persistence) =
                    self.config.failure_persistence
                {
                    let source_file = &self.config.source_file;

                    // Don't update the persistence file if we're a child
                    // process. The parent relies on it remaining consistent
                    // and will take care of updating it itself.
                    if !fork_output.is_in_fork() {
                        failure_persistence.save_persisted_failure2(
                            *source_file,
                            PersistedSeed(seed),
                            value,
                        );
                    }
                }
            }

            if let Err(e) = result {
                fork_output.terminate();
                return Err(e.into());
            }
        }

        fork_output.terminate();
        Ok(())
    }

    fn gen_and_run_case<S: Strategy>(
        &mut self,
        strategy: &S,
        f: &impl Fn(S::Value) -> TestCaseResult,
        replay: &mut impl Iterator<Item = TestCaseResult>,
        result_cache: &mut dyn ResultCache,
        fork_output: &mut ForkOutput,
    ) -> TestRunResult<S> {
        let case = unwrap_or!(strategy.new_tree(self), msg =>
                return Err(TestError::Abort(msg)));

        if self.run_one_with_replay(
            case,
            f,
            replay,
            result_cache,
            fork_output,
        )? {
            self.successes += 1;
        }
        Ok(())
    }

    /// Run one specific test case against this runner.
    ///
    /// If the test fails, finds the minimal failing test case. If the test
    /// does not fail, returns whether it succeeded or was filtered out.
    ///
    /// This does not honour the `fork` config, and will not be able to
    /// terminate the run if it runs for longer than `timeout`. However, if the
    /// test function returns but took longer than `timeout`, the test case
    /// will fail.
    pub fn run_one<V: ValueTree>(
        &mut self,
        case: V,
        test: impl Fn(V::Value) -> TestCaseResult,
    ) -> Result<bool, TestError<V::Value>> {
        let mut result_cache = self.new_cache();
        self.run_one_with_replay(
            case,
            test,
            &mut iter::empty::<TestCaseResult>().fuse(),
            &mut *result_cache,
            &mut ForkOutput::empty(),
        )
    }

    fn run_one_with_replay<V: ValueTree>(
        &mut self,
        mut case: V,
        test: impl Fn(V::Value) -> TestCaseResult,
        replay: &mut impl Iterator<Item = TestCaseResult>,
        result_cache: &mut dyn ResultCache,
        fork_output: &mut ForkOutput,
    ) -> Result<bool, TestError<V::Value>> {
        let result = call_test(
            self,
            case.current(),
            &test,
            replay,
            result_cache,
            fork_output,
        );

        match result {
            Ok(_) => Ok(true),
            Err(TestCaseError::Fail(why)) => {
                let why = self
                    .shrink(&mut case, test, replay, result_cache, fork_output)
                    .unwrap_or(why);
                Err(TestError::Fail(why, case.current()))
            }
            Err(TestCaseError::Reject(whence)) => {
                self.reject_global(whence)?;
                Ok(false)
            }
        }
    }

    fn shrink<V: ValueTree>(
        &mut self,
        case: &mut V,
        test: impl Fn(V::Value) -> TestCaseResult,
        replay: &mut impl Iterator<Item = TestCaseResult>,
        result_cache: &mut dyn ResultCache,
        fork_output: &mut ForkOutput,
    ) -> Option<Reason> {
        #[cfg(feature = "std")]
        use std::time;

        let mut last_failure = None;
        let mut iterations = 0;
        #[cfg(feature = "std")]
        let start_time = time::Instant::now();

        if case.simplify() {
            loop {
                #[cfg(feature = "std")]
                let timed_out = if self.config.max_shrink_time > 0 {
                    let elapsed = start_time.elapsed();
                    let elapsed_ms = elapsed
                        .as_secs()
                        .saturating_mul(1000)
                        .saturating_add(elapsed.subsec_millis().into());
                    if elapsed_ms > self.config.max_shrink_time as u64 {
                        Some(elapsed_ms)
                    } else {
                        None
                    }
                } else {
                    None
                };
                #[cfg(not(feature = "std"))]
                let timed_out: Option<u64> = None;

                let bail = if iterations >= self.config.max_shrink_iters() {
                    #[cfg(feature = "std")]
                    const CONTROLLER: &str =
                        "the PROPTEST_MAX_SHRINK_ITERS environment \
                         variable or ProptestConfig.max_shrink_iters";
                    #[cfg(not(feature = "std"))]
                    const CONTROLLER: &str = "ProptestConfig.max_shrink_iters";
                    verbose_message!(
                        self,
                        ALWAYS,
                        "Aborting shrinking after {} iterations (set {} \
                         to a large(r) value to shrink more; current \
                         configuration: {} iterations)",
                        CONTROLLER,
                        self.config.max_shrink_iters(),
                        iterations
                    );
                    true
                } else if let Some(ms) = timed_out {
                    #[cfg(feature = "std")]
                    const CONTROLLER: &str =
                        "the PROPTEST_MAX_SHRINK_TIME environment \
                         variable or ProptestConfig.max_shrink_time";
                    #[cfg(feature = "std")]
                    let current = self.config.max_shrink_time;
                    #[cfg(not(feature = "std"))]
                    const CONTROLLER: &str = "(not configurable in no_std)";
                    #[cfg(not(feature = "std"))]
                    let current = 0;
                    verbose_message!(
                        self,
                        ALWAYS,
                        "Aborting shrinking after taking too long: {} ms \
                         (set {} to a large(r) value to shrink more; current \
                         configuration: {} ms)",
                        ms,
                        CONTROLLER,
                        current
                    );
                    true
                } else {
                    false
                };

                if bail {
                    // Move back to the most recent failing case
                    while case.complicate() {
                        fork_output.append(&Ok(()));
                    }
                    break;
                }

                iterations += 1;

                let result = call_test(
                    self,
                    case.current(),
                    &test,
                    replay,
                    result_cache,
                    fork_output,
                );

                match result {
                    // Rejections are effectively a pass here,
                    // since they indicate that any behaviour of
                    // the function under test is acceptable.
                    Ok(_) | Err(TestCaseError::Reject(..)) => {
                        if !case.complicate() {
                            break;
                        }
                    }
                    Err(TestCaseError::Fail(why)) => {
                        last_failure = Some(why);
                        if !case.simplify() {
                            break;
                        }
                    }
                }
            }
        }

        last_failure
    }

    /// Update the state to account for a local rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    pub fn reject_local(
        &mut self,
        whence: impl Into<Reason>,
    ) -> Result<(), Reason> {
        if self.local_rejects >= self.config.max_local_rejects {
            Err("Too many local rejects".into())
        } else {
            self.local_rejects += 1;
            Self::insert_or_increment(
                &mut self.local_reject_detail,
                whence.into(),
            );
            Ok(())
        }
    }

    /// Update the state to account for a global rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    fn reject_global<T>(&mut self, whence: Reason) -> Result<(), TestError<T>> {
        if self.global_rejects >= self.config.max_global_rejects {
            Err(TestError::Abort("Too many global rejects".into()))
        } else {
            self.global_rejects += 1;
            Self::insert_or_increment(&mut self.global_reject_detail, whence);
            Ok(())
        }
    }

    /// Insert 1 or increment the rejection detail at key for whence.
    fn insert_or_increment(into: &mut RejectionDetail, whence: Reason) {
        into.entry(whence)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// Increment the counter of flat map regenerations and return whether it
    /// is still under the configured limit.
    pub fn flat_map_regen(&self) -> bool {
        self.flat_map_regens.fetch_add(1, SeqCst)
            < self.config.max_flat_map_regens as usize
    }

    fn new_cache(&self) -> Box<dyn ResultCache> {
        (self.config.result_cache)()
    }
}

#[cfg(feature = "fork")]
fn init_replay(rng: &mut TestRng) -> (Vec<TestCaseResult>, ForkOutput) {
    use crate::test_runner::replay::{open_file, Replay, ReplayFileStatus::*};

    if let Some(path) = env::var_os(ENV_FORK_FILE) {
        let mut file = open_file(&path).expect("Failed to open replay file");
        let loaded =
            Replay::parse_from(&mut file).expect("Failed to read replay file");
        match loaded {
            InProgress(replay) => {
                rng.set_seed(replay.seed);
                (replay.steps, ForkOutput { file: Some(file) })
            }

            Terminated(_) => {
                panic!("Replay file for child process is terminated?")
            }

            Corrupt => panic!("Replay file for child process is corrupt"),
        }
    } else {
        (vec![], ForkOutput::empty())
    }
}

#[cfg(not(feature = "fork"))]
fn init_replay(
    _rng: &mut TestRng,
) -> (iter::Empty<TestCaseResult>, ForkOutput) {
    (iter::empty(), ForkOutput::empty())
}

#[cfg(TBD)]
#[cfg(test)]
mod test {
    use std::cell::Cell;
    use std::fs;

    use super::*;
    use crate::strategy::Strategy;
    use crate::test_runner::{FileFailurePersistence, RngAlgorithm, TestRng};

    #[test]
    fn test_pass() {
        let mut runner = TestRunner::default();
        let result = runner.run(&(1u32..), |v| {
            assert!(v > 0);
            Ok(())
        });
        assert_eq!(Ok(()), result);
    }

    #[derive(Clone, Copy, PartialEq)]
    struct PoorlyBehavedDebug(i32);
    impl fmt::Debug for PoorlyBehavedDebug {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "\r\n{:?}\r\n", self.0)
        }
    }

    #[cfg(feature = "fork")]
    #[test]
    fn normal_failure_in_fork_results_in_correct_failure() {
        let mut runner = TestRunner::new(Config {
            fork: true,
            test_name: Some(concat!(
                module_path!(),
                "::normal_failure_in_fork_results_in_correct_failure"
            )),
            ..Config::default()
        });

        let failure = runner
            .run(&(0u32..1000), |v| {
                prop_assert!(v < 500);
                Ok(())
            })
            .err()
            .unwrap();

        match failure {
            TestError::Fail(_, value) => assert_eq!(500, value),
            failure => panic!("Unexpected failure: {:?}", failure),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn duplicate_tests_not_run_with_basic_result_cache() {
        use std::cell::{Cell, RefCell};
        use std::collections::HashSet;
        use std::rc::Rc;

        for _ in 0..256 {
            let mut runner = TestRunner::new(Config {
                failure_persistence: None,
                result_cache:
                    crate::test_runner::result_cache::basic_result_cache,
                ..Config::default()
            });
            let pass = Rc::new(Cell::new(true));
            let seen = Rc::new(RefCell::new(HashSet::new()));
            let result =
                runner.run(&(0u32..65536u32).prop_map(|v| v % 10), |val| {
                    if !seen.borrow_mut().insert(val) {
                        println!("Value {} seen more than once", val);
                        pass.set(false);
                    }

                    prop_assert!(val <= 5);
                    Ok(())
                });

            assert!(pass.get());
            if let Err(TestError::Fail(_, val)) = result {
                assert_eq!(6, val);
            } else {
                panic!("Incorrect result: {:?}", result);
            }
        }
    }
}
