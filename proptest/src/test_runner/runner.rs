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
use crate::test_runner::reason::*;

use crate::test_runner::rng::TestRng;

/// State used when running a proptest test.
#[derive(Clone)]
pub struct TestRunner {
    config: Config,
    successes: u32,
    local_rejects: u32,
    global_rejects: u32,
    rng: TestRng,
}

impl fmt::Debug for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TestRunner")
            .field("config", &self.config)
            .field("successes", &self.successes)
            .field("local_rejects", &self.local_rejects)
            .field("global_rejects", &self.global_rejects)
            .field("rng", &"<TestRng>")
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
struct ForkOutput {}

#[cfg(feature = "fork")]
impl ForkOutput {}

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
        let tree = strategy.new_tree(self).unwrap();
        assert!(matches!(
            test(tree.current()),
            Ok(_) | Err(TestCaseError::Reject(_))
        ));
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
        test(case.current()).unwrap();
        Ok(true)
    }

    /// Update the state to account for a local rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    /// Kani Note: This function will always succeed because Kani only runs once.
    pub fn reject_local(&mut self, _: impl Into<Reason>) -> Result<(), Reason> {
        if self.local_rejects >= self.config.max_local_rejects {
            Err("Too many local rejects".into())
        } else {
            self.local_rejects += 1;
            Ok(())
        }
    }

    /// Increment the counter of flat map regenerations and return
    /// whether it is still under the configured limit.  Kani Note:
    /// This function will always return false because Kani does not
    /// require this functionality,
    pub fn flat_map_regen(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod test {
    use std::cell::Cell;
    use std::fs;

    use super::*;
    use crate::strategy::{Just, Strategy};
    use crate::test_runner::Config;

    proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
        #[test]
        fn successfully_linked_proptest(_ in &Just(()) ) {
            let config = Config::default();
            prop_assert_eq!(
                config.cases,
                256,
                "Default .cases should be 256. Check: src/test_runner/config.rs"
            );
        }
    }

    proptest! {
    #[test]
    fn possible_values_are_even(
            x in
        crate::prop_oneof![
                    1 => Just(0 as u32),
                    2 => Just(2 as u32),
                    0 => Just(3 as u32), // cannot be picked
        ]
    ) {
            assert_eq!(x % 2, 0, "Just(3) cannot be picked b/c weight is 0");
    }
    }

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

        // Due to kani-side limitations in kani::expect_fail, this test had to be modified. However
        // it should be reverted once the issue is fixed. See #1679 for details.
        runner
            .run(&(0u32..1000), |v| {
                prop_assert!(v < 1000);
                Ok(())
            })
            .err();
    }
}
