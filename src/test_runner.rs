//-
// Copyright 2017 2017 Jason Lingle
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

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

use rand::{self, XorShiftRng};

use strategy::*;

/// Configuration for how a proptest test should be run.
#[derive(Clone, Debug)]
pub struct Config {
    /// The number of successful test cases that must execute for the test as a
    /// whole to pass.
    ///
    /// The default is 256.
    pub cases: u32,
    /// The maximum number of individual inputs that may be rejected before the
    /// test as a whole aborts.
    ///
    /// The default is 65536.
    pub max_local_rejects: u32,
    /// The maximum number of combined inputs that may be rejected before the
    /// test as a whole aborts.
    ///
    /// The default is 1024.
    pub max_global_rejects: u32,
    // Needs to be public so FRU syntax can be used.
    #[doc(hidden)]
    pub _non_exhaustive: (),
}

impl Default for Config {
    fn default() -> Config {
        Config {
            cases: 256,
            max_local_rejects: 65536,
            max_global_rejects: 1024,
            _non_exhaustive: (),
        }
    }
}

/// Non-success stati produced by generating or running tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status<T> {
    /// Reject the generated test case in its entirety.
    ///
    /// The string indicates the "location" of the code that caused this value
    /// to be rejected.
    RejectGlobal(String),
    /// Fail the test case with the given message and example input.
    Fail(String, T),
    /// Fail the entire test with the given message.
    Abort(String),
}

/// State used when running a proptest test.
#[derive(Clone)]
pub struct TestRunner {
    config: Config,
    successes: u32,
    local_rejects: u32,
    global_rejects: u32,
    rng: XorShiftRng,

    local_reject_detail: BTreeMap<String, u32>,
    global_reject_detail: BTreeMap<String, u32>,
}

impl fmt::Debug for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TestRunner")
            .field("config", &self.config)
            .field("successes", &self.successes)
            .field("local_rejects", &self.local_rejects)
            .field("global_rejects", &self.global_rejects)
            .field("rng", &"<XorShiftRng>".to_owned())
            .field("local_reject_detail", &self.local_reject_detail)
            .field("global_reject_detail", &self.global_reject_detail)
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

impl TestRunner {
    /// Create a fresh `TestRunner` with the given configuration.
    pub fn new(config: Config) -> Self {
        TestRunner {
            config: config,
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: rand::weak_rng(),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
        }
    }

    /// Returns the RNG for this test run.
    pub fn rng(&mut self) -> &mut XorShiftRng {
        &mut self.rng
    }

    pub fn run<S : Strategy,
               F : Fn (&<S::Value as ValueTree>::Value)
                       -> Result<(), Status<<S::Value as ValueTree>::Value>>>
        (&mut self, strategy: &S, f: F)
         -> Result<(), Status<<S::Value as ValueTree>::Value>>
    {
        macro_rules! test {
            ($v:expr) => { {
                let v = $v;
                match panic::catch_unwind(AssertUnwindSafe(|| f(&v))) {
                    Ok(r) => r,
                    Err(what) => {
                        let msg = what.downcast::<&'static str>()
                            .map(|s| (*s).to_owned())
                            .or_else(|what| what.downcast::<String>().map(|b| *b))
                            .unwrap_or_else(
                                |_| "<unknown panic value>".to_owned());
                        Err(Status::Fail(msg, v))
                    },
                }
            } }
        }

        while self.successes < self.config.cases {
            let mut case = match strategy.new_value(self) {
                Ok(v) => v,
                Err(msg) => return Err(Status::Abort(msg)),
            };

            match test!(case.current()) {
                Ok(_) => {
                    self.successes += 1;
                }
                Err(Status::Fail(why, input)) => {
                    let mut last_failure = (why, input);
                    if case.simplify() {
                        loop {
                            let passed = match test!(case.current()) {
                                Ok(_) => true,
                                // Rejections are effectively a pass here,
                                // since they indicate that any behaviour of
                                // the function under test is acceptable.
                                Err(Status::RejectGlobal(_)) => true,

                                Err(Status::Fail(why, input)) => {
                                    last_failure = (why, input);
                                    false
                                },
                                Err(_) => false,
                            };

                            if passed {
                                if !case.complicate() {
                                    break;
                                }
                            } else {
                                if !case.simplify() {
                                    break;
                                }
                            }
                        }
                    }

                    return Err(Status::Fail(last_failure.0, last_failure.1));
                },
                Err(Status::RejectGlobal(whence)) => {
                    self.reject_global(whence)?;
                },
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Update the state to account for a local rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    pub fn reject_local<T>(&mut self, whence: String) -> Result<(),Status<T>> {
        if self.local_rejects >= self.config.max_local_rejects {
            Err(Status::Abort("Too many local rejects".to_owned()))
        } else {
            self.local_rejects += 1;
            let need_insert = if let Some(count) =
                self.local_reject_detail.get_mut(&whence)
            {
                *count += 1;
                false
            } else {
                true
            };
            if need_insert {
                self.local_reject_detail.insert(whence, 1);
            }

            Ok(())
        }
    }

    /// Update the state to account for a global rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    fn reject_global<T>(&mut self, whence: String) -> Result<(),Status<T>> {
        if self.global_rejects >= self.config.max_global_rejects {
            Err(Status::Abort("Too many global rejects".to_owned()))
        } else {
            self.global_rejects += 1;
            let need_insert = if let Some(count) =
                self.global_reject_detail.get_mut(&whence)
            {
                *count += 1;
                false
            } else {
                true
            };
            if need_insert {
                self.global_reject_detail.insert(whence.clone(), 1);
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn gives_up_after_too_many_rejections() {
        let config = Config::default();
        let mut runner = TestRunner::new(config.clone());
        let runs = Cell::new(0);
        let result = runner.run(&(0u32..), |_| {
            runs.set(runs.get() + 1);
            Err(Status::RejectGlobal("reject".to_owned()))
        });
        match result {
            Err(Status::Abort(_)) => (),
            e => panic!("Unexpected result: {:?}", e),
        }
        assert_eq!(config.max_global_rejects + 1, runs.get());
    }

    #[test]
    fn test_pass() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&(1u32..), |&v| { assert!(v > 0); Ok(()) });
        assert_eq!(Ok(()), result);
    }

    #[test]
    fn test_fail_via_result() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&(0u32..10u32), |&v| if v < 5 {
            Ok(())
        } else {
            // TODO Returning the input here is kind of awkward
            Err(Status::Fail("not less than 5".to_owned(), v))
        });

        assert_eq!(Err(Status::Fail("not less than 5".to_owned(), 5)),
                   result);
    }

    #[test]
    fn test_fail_via_panic() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&(0u32..10u32), |&v| {
            assert!(v < 5, "not less than 5");
            Ok(())
        });
        assert_eq!(Err(Status::Fail("not less than 5".to_owned(), 5)),
                   result);
    }
}
