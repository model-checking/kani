//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::time`.

use std::time::*;

use strategy::statics::static_map;
use arbitrary::*;

arbitrary!(Duration, SMapped<(u64, u32), Self>;
    static_map(any::<(u64, u32)>(), |(a, b)| Duration::new(a, b))
);

// Instant::now() "never" returns the same Instant, so no shrinking may occur!
arbitrary!(Instant; Self::now());

// Same for SystemTime.
arbitrary!(SystemTime; Self::now());

/*
A possible logic for SystemTimeError:
fn gen_ste() -> SystemTimeError {
    (SystemTime::now() + Duration::from_millis(10)).elapsed().unwrap_err()
}
This may however panic from time to time. NTP could also ruin our day!
*/

#[cfg(test)]
mod test {
    no_panic_test!(
        duration => Duration,
        instant  => Instant,
        system_time => SystemTime
    );
}