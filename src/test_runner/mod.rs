//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! State and functions for running proptest tests.
//!
//! You do not normally need to access things in this module directly except
//! when implementing new low-level strategies.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

use rand::{self, Rand, SeedableRng, XorShiftRng};

use strategy::*;

mod failure_persistence;
mod config;
mod reason;
mod errors;

pub use self::failure_persistence::*;
pub use self::config::*;
pub use self::reason::*;
pub use self::errors::*;

type RejectionDetail = BTreeMap<Reason, u32>;

/// State used when running a proptest test.
#[derive(Clone)]
pub struct TestRunner {
    config: Config,
    successes: u32,
    local_rejects: u32,
    global_rejects: u32,
    rng: XorShiftRng,
    flat_map_regens: Arc<AtomicUsize>,

    local_reject_detail: RejectionDetail,
    global_reject_detail: RejectionDetail,

    source_file: Option<Cow<'static, Path>>,
}

impl fmt::Debug for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TestRunner")
            .field("config", &self.config)
            .field("successes", &self.successes)
            .field("local_rejects", &self.local_rejects)
            .field("global_rejects", &self.global_rejects)
            .field("rng", &"<XorShiftRng>")
            .field("flat_map_regens", &self.flat_map_regens)
            .field("local_reject_detail", &self.local_reject_detail)
            .field("global_reject_detail", &self.global_reject_detail)
            .field("source_file", &self.source_file)
            .finish()
    }
}

impl fmt::Display for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tsuccesses: {}\n\
                   \tlocal rejects: {}\n",
               self.successes, self.local_rejects)?;
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

fn panic_guard<V, F>(case: &V, test: &F) -> TestCaseResult
where
    F: Fn(&V) -> TestCaseResult
{
    match panic::catch_unwind(AssertUnwindSafe(|| test(case))) {
        Ok(r) => r,
        Err(what) => Err(TestCaseError::Fail(
            what.downcast::<&'static str>().map(|s| (*s).into())
                .or_else(|what| what.downcast::<String>().map(|b| (*b).into()))
                .or_else(|what| what.downcast::<Box<str>>().map(|b| (*b).into()))
                .unwrap_or_else(|_| "<unknown panic value>".into()))),
    }
}

impl TestRunner {
    /// Create a fresh `TestRunner` with the given configuration.
    pub fn new(config: Config) -> Self {
        TestRunner {
            config: config,
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: rand::weak_rng(),
            flat_map_regens: Arc::new(AtomicUsize::new(0)),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
            source_file: None,
        }
    }

    /// Create a fresh `TestRunner` with the same config and global counters as
    /// this one, but with local state reset and an independent `Rng` (but
    /// deterministic).
    pub(crate) fn partial_clone(&mut self) -> Self {
        let rng = self.new_rng();

        TestRunner {
            config: self.config.clone(),
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: rng,
            flat_map_regens: Arc::clone(&self.flat_map_regens),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
            source_file: self.source_file.clone(),
        }
    }

    /// Returns the RNG for this test run.
    pub fn rng(&mut self) -> &mut XorShiftRng {
        &mut self.rng
    }

    fn new_rng_seed(&mut self) -> [u32;4] {
        let mut seed = <[u32;4] as Rand>::rand(&mut self.rng);
        // Directly using XorShiftRng::from_seed() at this point would result
        // in self.rng and the returned value being exactly the same. Perturb
        // the seed with some arbitrary values to prevent this.
        for word in &mut seed {
            *word ^= 0xdead_beef;
        }
        seed
    }

    /// Create a new, independent but deterministic RNG from the RNG in this
    /// runner.
    pub fn new_rng(&mut self) -> XorShiftRng {
        XorShiftRng::from_seed(self.new_rng_seed())
    }

    /// Returns the configuration of this runner.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Set the source file to use for resolving the location of the persisted
    /// failing cases file.
    ///
    /// The source location can only be used if it is absolute. If `source` is
    /// not an absolute path, an attempt will be made to determine the absolute
    /// path based on the current working directory and its parents. If no
    /// absolute path can be determined, a warning will be printed and proptest
    /// will continue as if this function had never been called.
    ///
    /// See [`FailurePersistence`](enum.FailurePersistence.html) for details on
    /// how this value is used once it is made absolute.
    ///
    /// This is normally called automatically by the `proptest!` macro, which
    /// passes `file!()`.
    pub fn set_source_file(&mut self, source: &'static Path) {
        self.set_source_file_with_cwd(env::current_dir, source)
    }

    pub(crate) fn set_source_file_with_cwd<F>(
        &mut self, getcwd: F,
        source: &'static Path)
    where F : FnOnce () -> io::Result<PathBuf> {
        self.source_file = if source.is_absolute() {
            // On Unix, `file!()` is absolute. In these cases, we can use
            // that path directly.
            Some(Cow::Borrowed(source))
        } else {
            // On Windows, `file!()` is relative to the crate root, but the
            // test is not generally run with the crate root as the working
            // directory, so the path is not directly usable. However, the
            // working directory is almost always a subdirectory of the crate
            // root, so pop directories off until pushing the source onto the
            // directory results in a path that refers to an existing file.
            // Once we find such a path, we can use that.
            //
            // If we can't figure out an absolute path, print a warning and act
            // as if no source had been given.
            match getcwd() {
                Ok(mut cwd) => {
                    loop {
                        let joined = cwd.join(source);
                        if joined.is_file() {
                            break Some(Cow::Owned(joined));
                        }

                        if !cwd.pop() {
                            eprintln!(
                                "proptest: Failed to find absolute path of \
                                 source file '{:?}'. Ensure the test is \
                                 being run from somewhere within the crate \
                                 directory hierarchy.", source);
                            break None;
                        }
                    }
                },

                Err(e) => {
                    eprintln!("proptest: Failed to determine current \
                               directory, so the relative source path \
                               '{:?}' cannot be resolved: {}",
                              source, e);
                    None
                }
            }
        }
    }

    pub(crate) fn source_file(&self) -> Option<&Path> {
        self.source_file.as_ref().map(|cow| &**cow)
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
    pub fn run<S : Strategy,
               F : Fn (&ValueFor<S>) -> TestCaseResult>
        (&mut self, strategy: &S, test: F)
         -> Result<(), TestError<ValueFor<S>>>
    {
        let persist_path = self.config.failure_persistence.resolve(
            self.source_file());

        let old_rng = self.rng.clone();
        for persisted_seed in load_persisted_failures(persist_path.as_ref()) {
            self.rng = XorShiftRng::from_seed(persisted_seed);
            self.gen_and_run_case(strategy, &test)?;
        }
        self.rng = old_rng;

        while self.successes < self.config.cases {
            // Generate a new seed and make an RNG from that so that we know
            // what seed to persist if this case fails.
            let seed = self.new_rng_seed();
            self.rng = XorShiftRng::from_seed(seed);
            let result = self.gen_and_run_case(strategy, &test);
            if let Err(TestError::Fail(_, ref value)) = result {
                save_persisted_failure(persist_path.as_ref(), seed, value);
            }

            result?;
        }

        Ok(())
    }

    fn gen_and_run_case<S : Strategy, F : Fn (&ValueFor<S>) -> TestCaseResult>
        (&mut self, strategy: &S, f: &F)
        -> Result<(), TestError<ValueFor<S>>>
    {
        let case = match strategy.new_value(self) {
            Ok(v) => v,
            Err(msg) => return Err(TestError::Abort(msg)),
        };
        if self.run_one(case, f)? {
            self.successes += 1;
        }
        Ok(())
    }

    /// Run one specific test case against this runner.
    ///
    /// If the test fails, finds the minimal failing test case. If the test
    /// does not fail, returns whether it succeeded or was filtered out.
    pub fn run_one<V : ValueTree, F : Fn (&V::Value) -> TestCaseResult>
        (&mut self, case: V, test: F) -> Result<bool, TestError<V::Value>>
    {
        let curr = case.current();
        match panic_guard(&curr, &test) {
            Ok(_) => Ok(true),
            Err(TestCaseError::Fail(why)) => {
                let (why, curr) = self.shrink(case, test).unwrap_or((why, curr));
                Err(TestError::Fail(why, curr))
            },
            Err(TestCaseError::Reject(whence)) => {
                self.reject_global(whence)?;
                Ok(false)
            },
        }
    }

    fn shrink<V: ValueTree, F : Fn (&V::Value) -> TestCaseResult>
        (&mut self, mut case: V, test: F) -> Option<(Reason, V::Value)>
    {
        let mut last_failure = None;

        if case.simplify() {
            loop {
                let curr = case.current();
                match panic_guard(&curr, &test) {
                    // Rejections are effectively a pass here,
                    // since they indicate that any behaviour of
                    // the function under test is acceptable.
                    Ok(_) | Err(TestCaseError::Reject(..)) => {
                        if !case.complicate() {
                            break;
                        }
                    },
                    Err(TestCaseError::Fail(why)) => {
                        last_failure = Some((why, curr));
                        if !case.simplify() {
                            break;
                        }
                    },
                }
            }
        }

        last_failure
    }

    /// Update the state to account for a local rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    pub fn reject_local<R>(&mut self, whence: R) -> Result<(), Reason>
    where
        R: Into<Reason>
    {
        if self.local_rejects >= self.config.max_local_rejects {
            Err("Too many local rejects".into())
        } else {
            self.local_rejects += 1;
            Self::insert_or_increment(&mut self.local_reject_detail,
                whence.into());
            Ok(())
        }
    }

    /// Update the state to account for a global rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    fn reject_global<T>(&mut self, whence: Reason) -> Result<(),TestError<T>> {
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
        use std::collections::btree_map::Entry::*;
        match into.entry(whence) {
            Occupied(oe) => { *oe.into_mut() += 1; },
            Vacant(ve)   => { ve.insert(1); },
        }
        /*
        // TODO: Replace with once and_modify is stable:
        into.entry(whence)
            .and_modify(|count| { *count += 1 })
            .or_insert(1);
        */
    }

    /// Increment the counter of flat map regenerations and return whether it
    /// is still under the configured limit.
    pub fn flat_map_regen(&self) -> bool {
        self.flat_map_regens.fetch_add(1, SeqCst) <
            self.config.max_flat_map_regens as usize
    }
}

#[cfg(test)]
mod test {
    use std::cell::Cell;
    use std::fs;

    use super::*;
    use strategy::Strategy;

    #[test]
    fn gives_up_after_too_many_rejections() {
        let config = Config::default();
        let mut runner = TestRunner::new(config.clone());
        let runs = Cell::new(0);
        let result = runner.run(&(0u32..), |_| {
            runs.set(runs.get() + 1);
            Err(TestCaseError::reject("reject"))
        });
        match result {
            Err(TestError::Abort(_)) => (),
            e => panic!("Unexpected result: {:?}", e),
        }
        assert_eq!(config.max_global_rejects + 1, runs.get());
    }

    #[test]
    fn test_pass() {
        let mut runner = TestRunner::default();
        let result = runner.run(&(1u32..), |&v| { assert!(v > 0); Ok(()) });
        assert_eq!(Ok(()), result);
    }

    #[test]
    fn test_fail_via_result() {
        let mut runner = TestRunner::new(Config {
            failure_persistence: FailurePersistence::Off,
            .. Config::default()
        });
        let result = runner.run(
            &(0u32..10u32), |&v| {
                if v < 5 {
                    Ok(())
                } else {
                    Err(TestCaseError::fail("not less than 5"))
                }
            });

        assert_eq!(Err(TestError::Fail("not less than 5".into(), 5)), result);
    }

    #[test]
    fn test_fail_via_panic() {
        let mut runner = TestRunner::new(Config {
            failure_persistence: FailurePersistence::Off,
            .. Config::default()
        });
        let result = runner.run(&(0u32..10u32), |&v| {
            assert!(v < 5, "not less than 5");
            Ok(())
        });
        assert_eq!(Err(TestError::Fail("not less than 5".into(), 5)), result);
    }

    #[derive(Clone, Copy, PartialEq)]
    struct PoorlyBehavedDebug(i32);
    impl fmt::Debug for PoorlyBehavedDebug {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "\r\n{:?}\r\n", self.0)
        }
    }

    #[test]
    fn failing_cases_persisted_and_reloaded() {
        const FILE: &'static str = "persistence-test.txt";
        let _ = fs::remove_file(FILE);

        let max = 10_000_000i32;
        let input = (0i32..max).prop_map(PoorlyBehavedDebug);
        let config = Config {
            failure_persistence: FailurePersistence::Direct(FILE),
            .. Config::default()
        };

        // First test with cases that fail above half max, and then below half
        // max, to ensure we can correctly parse both lines of the persistence
        // file.
        let first_sub_failure = {
            TestRunner::new(config.clone()).run(&input, |v| {
                if v.0 < max/2 {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail("too big".into()))
                }
            }).expect_err("didn't fail?")
        };
        let first_super_failure = {
            TestRunner::new(config.clone()).run(&input, |v| {
                if v.0 >= max/2 {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail("too small".into()))
                }
            }).expect_err("didn't fail?")
        };
        let second_sub_failure = {
            TestRunner::new(config.clone()).run(&input, |v| {
                if v.0 < max/2 {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail("too big".into()))
                }
            }).expect_err("didn't fail?")
        };
        let second_super_failure = {
            TestRunner::new(config.clone()).run(&input, |v| {
                if v.0 >= max/2 {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail("too small".into()))
                }
            }).expect_err("didn't fail?")
        };

        assert_eq!(first_sub_failure, second_sub_failure);
        assert_eq!(first_super_failure, second_super_failure);
    }

    #[test]
    fn relative_source_files_absolutified() {
        const TEST_RUNNER_PATH: &[&str] = &["src", "test_runner", "mod.rs"];
        lazy_static! {
            static ref TEST_RUNNER_RELATIVE: PathBuf =
                TEST_RUNNER_PATH.iter().collect();
        }
        const CARGO_DIR: &str = env!("CARGO_MANIFEST_DIR");

        let expected = ::std::iter::once(CARGO_DIR)
                        .chain(TEST_RUNNER_PATH.iter().map(|s| *s))
                        .collect::<PathBuf>();

        let mut runner = TestRunner::default();
        // Running from crate root
        runner.set_source_file_with_cwd(
            || Ok(Path::new(CARGO_DIR).to_owned()),
            &TEST_RUNNER_RELATIVE);
        assert_eq!(&*expected, runner.source_file().unwrap());

        // Running from test subdirectory
        runner.set_source_file_with_cwd(
            || Ok(Path::new(CARGO_DIR).join("target")),
            &TEST_RUNNER_RELATIVE);
        assert_eq!(&*expected, runner.source_file().unwrap());
    }

    #[test]
    fn new_rng_makes_separate_rng() {
        let mut runner = TestRunner::default();
        let mut rng2 = runner.new_rng();
        let rng1 = runner.rng();

        let from_1 = <[u32;4] as Rand>::rand(rng1);
        let from_2 = <[u32;4] as Rand>::rand(&mut rng2);

        assert_ne!(from_1, from_2);
    }
}
