//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rand::{Error, RngCore, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

/// Proptest's random number generator.
///
/// Currently, this is just a wrapper around `XorShiftRng`.
#[derive(Clone, Debug)]
pub struct TestRng { rng: XorShiftRng }

impl RngCore for TestRng {
    fn next_u32(&mut self) -> u32 { self.rng.next_u32() }
    fn next_u64(&mut self) -> u64 { self.rng.next_u64() }
    fn fill_bytes(&mut self, dest: &mut [u8]) { self.rng.fill_bytes(dest) }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.rng.try_fill_bytes(dest)
    }
}

pub(crate) type Seed = [u8; 16];

impl TestRng {
    /// Construct a default TestRng from entropy.
    pub(crate) fn default_rng() -> Self {
        #[cfg(feature = "std")]
        {
            use rand::FromEntropy;
            Self { rng: XorShiftRng::from_entropy() }
        }
        #[cfg(not(feature = "std"))]
        Self::from_seed([
            0x19, 0x3a, 0x67, 0x54, // x
            0x69, 0xd4, 0xa7, 0xa8, // y
            0x05, 0x0e, 0x83, 0x97, // z
            0xbb, 0xa7, 0x3b, 0x11, // w
        ])
    }

    /// Construct a TestRng by the perturbed randomized seed
    /// from an existing TestRng.
    pub(crate) fn gen_rng(&mut self) -> Self {
        Self::from_seed(self.new_rng_seed())
    }

    /// Overwrite the given TestRng with the provided seed.
    pub(crate) fn set_seed(&mut self, seed: Seed) {
        *self = Self::from_seed(seed);
    }

    /// Generate a new randomized seed, set it to this TestRng,
    /// and return the seed.
    pub(crate) fn gen_get_seed(&mut self) -> Seed {
        let seed = self.new_rng_seed();
        self.set_seed(seed);
        seed
    }

    /// Randomize a perturbed randomized seed from the given TestRng.
    pub(crate) fn new_rng_seed(&mut self) -> Seed {
        let mut seed = self.rng.gen::<Seed>();
        
        // Directly using XorShiftRng::from_seed() at this point would result
        // in self.rng and the returned value being exactly the same. Perturb
        // the seed with some arbitrary values to prevent this.
        for word in seed.chunks_mut(4) {
            word[3] ^= 0xde;
            word[2] ^= 0xad;
            word[1] ^= 0xbe;
            word[0] ^= 0xef;
        }

        seed
    }

    /// Construct a TestRng from a given seed.
    fn from_seed(seed: Seed) -> Self {
        Self { rng: XorShiftRng::from_seed(seed) }
    }
}
