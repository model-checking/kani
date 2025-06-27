// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand respects the invariants specified in the type's kani::any() implementation.
// In other words, test that autoharness is actually finding and using the appropriate kani::any() implementation,
// rather than letting CBMC generate nondetermistic values for the type.
// In this example, we check that the methods that take `Duration` as an argument generate Durations that respect the type invariant that nanos < NANOS_PER_SEC.
// See the "TEST NOTE" inline comments for how we specifically check this.

// Simplified code from https://github.com/model-checking/verify-rust-std/blob/3f4234a19211e677d51df3061db67477b29af171/library/core/src/time.rs#L1

use kani::Invariant;

const NANOS_PER_SEC: u32 = 1_000_000_000;

#[derive(kani::Arbitrary)]
#[derive(Clone, Copy)]
pub struct Nanoseconds(u32);

#[derive(Clone, Copy)]
#[derive(kani::Invariant)]
pub struct Duration {
    secs: u64,
    nanos: Nanoseconds,
}

impl kani::Invariant for Nanoseconds {
    fn is_safe(&self) -> bool {
        self.as_inner() < NANOS_PER_SEC
    }
}

impl Nanoseconds {
    #[kani::requires(val < NANOS_PER_SEC)]
    #[kani::ensures(|nano| nano.is_safe())]
    pub const unsafe fn new_unchecked(val: u32) -> Self {
        // SAFETY: caller promises that val < NANOS_PER_SEC
        Self(val)
    }

    pub const fn as_inner(self) -> u32 {
        // SAFETY: This is a transparent wrapper, so unwrapping it is sound
        unsafe { core::mem::transmute(self) }
    }
}

impl kani::Arbitrary for Duration {
    fn any() -> Self {
        let d = Duration { secs: kani::any(), nanos: kani::any() };
        kani::assume(d.is_safe());
        d
    }
}

impl Duration {
    // TEST NOTE: the automatic harness for this method fails because it can panic.
    #[kani::ensures(|duration| duration.is_safe())]
    pub const fn new(secs: u64, nanos: u32) -> Duration {
        if nanos < NANOS_PER_SEC {
            // SAFETY: nanos < NANOS_PER_SEC, therefore nanos is within the valid range
            Duration { secs, nanos: unsafe { Nanoseconds::new_unchecked(nanos) } }
        } else {
            let secs = secs
                .checked_add((nanos / NANOS_PER_SEC) as u64)
                .expect("overflow in Duration::new");
            let nanos = nanos % NANOS_PER_SEC;
            // SAFETY: nanos % NANOS_PER_SEC < NANOS_PER_SEC, therefore nanos is within the valid range
            Duration { secs, nanos: unsafe { Nanoseconds::new_unchecked(nanos) } }
        }
    }

    pub const fn abs_diff(self, other: Duration) -> Duration {
        if let Some(res) = self.checked_sub(other) { res } else { other.checked_sub(self).unwrap() }
    }

    #[kani::ensures(|duration| duration.is_none() || duration.unwrap().is_safe())]
    pub const fn checked_add(self, rhs: Duration) -> Option<Duration> {
        if let Some(mut secs) = self.secs.checked_add(rhs.secs) {
            // TEST NOTE: this addition doesn't overflow iff `self` and `rhs` respect the Duration type invariant
            let mut nanos = self.nanos.as_inner() + rhs.nanos.as_inner();
            if nanos >= NANOS_PER_SEC {
                nanos -= NANOS_PER_SEC;
                if let Some(new_secs) = secs.checked_add(1) {
                    secs = new_secs;
                } else {
                    return None;
                }
            }
            Some(Duration::new(secs, nanos))
        } else {
            None
        }
    }

    #[kani::ensures(|duration| duration.is_none() || duration.unwrap().is_safe())]
    pub const fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        if let Some(mut secs) = self.secs.checked_sub(rhs.secs) {
            let nanos = if self.nanos.as_inner() >= rhs.nanos.as_inner() {
                self.nanos.as_inner() - rhs.nanos.as_inner()
            } else if let Some(sub_secs) = secs.checked_sub(1) {
                secs = sub_secs;
                // TEST NOTE: this arithmetic doesn't overflow iff `self` and `rhs` respect the Duration type invariant
                self.nanos.as_inner() + NANOS_PER_SEC - rhs.nanos.as_inner()
            } else {
                return None;
            };
            Some(Duration::new(secs, nanos))
        } else {
            None
        }
    }
}
