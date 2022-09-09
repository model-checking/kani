//-
// Copyright 2017, 2018, 2019, 2020 The proptest developers
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

use crate::std_facade::{Arc, String, ToOwned, Vec};
use core::result::Result;
use core::{fmt, str, u8};

use rand::{self, RngCore};

/// Identifies a particular RNG algorithm supported by proptest.
///
/// Proptest supports dynamic configuration of algorithms to allow it to
/// continue operating with persisted regression files and to allow the
/// configuration to be expressed in the `Config` struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RngAlgorithm {
    /// The [XorShift](https://rust-random.github.io/rand/rand_xorshift/struct.XorShiftRng.html)
    /// algorithm. This was the default up through and including Proptest 0.9.0.
    ///
    /// It is faster than ChaCha but produces lower quality randomness and has
    /// some pathological cases where it may fail to produce outputs that are
    /// random even to casual observation.
    ///
    /// The seed must be exactly 16 bytes.
    XorShift,
    /// The [ChaCha](https://rust-random.github.io/rand/rand_chacha/struct.ChaChaRng.html)
    /// algorithm. This became the default with Proptest 0.9.1.
    ///
    /// The seed must be exactly 32 bytes.
    ChaCha,
    /// This is not an actual RNG algorithm, but instead returns data directly
    /// from its "seed".
    ///
    /// This is useful when Proptest is being driven from some other entropy
    /// source, such as a fuzzer.
    ///
    /// If the seed is depleted, the RNG will return 0s forever.
    ///
    /// Note that in cases where a new RNG is to be derived from an existing
    /// one, *the data is split evenly between them*, regardless of how much
    /// entropy is actually needed. This means that combinators like
    /// `prop_perturb` and `prop_flat_map` can require extremely large inputs.
    PassThrough,
    /// This is equivalent to the `ChaCha` RNG, with the addition that it
    /// records the bytes used to create a value.
    ///
    /// This is useful when Proptest is used for fuzzing, and a corpus of
    /// initial inputs need to be created. Note that in these cases, you need
    /// to use the `TestRunner` API directly yourself instead of using the
    /// `proptest!` macro, as otherwise there is no way to obtain the bytes
    /// this captures.
    Recorder,
    #[allow(missing_docs)]
    #[doc(hidden)]
    _NonExhaustive,
}

impl Default for RngAlgorithm {
    fn default() -> Self {
        RngAlgorithm::ChaCha
    }
}

impl RngAlgorithm {
    pub(crate) fn persistence_key(self) -> &'static str {
        match self {
            RngAlgorithm::XorShift => "xs",
            RngAlgorithm::ChaCha => "cc",
            RngAlgorithm::PassThrough => "pt",
            RngAlgorithm::Recorder => "rc",
            RngAlgorithm::_NonExhaustive => unreachable!(),
        }
    }

    pub(crate) fn from_persistence_key(k: &str) -> Option<Self> {
        match k {
            "xs" => Some(RngAlgorithm::XorShift),
            "cc" => Some(RngAlgorithm::ChaCha),
            "pt" => Some(RngAlgorithm::PassThrough),
            "rc" => Some(RngAlgorithm::Recorder),
            _ => None,
        }
    }
}

// These two are only used for parsing the environment variable
// PROPTEST_RNG_ALGORITHM.
impl str::FromStr for RngAlgorithm {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        RngAlgorithm::from_persistence_key(s).ok_or(())
    }
}
impl fmt::Display for RngAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.persistence_key())
    }
}

/// Proptest's random number generator.
#[derive(Clone, Debug)]
pub struct TestRng {}

impl RngCore for TestRng {
    fn next_u32(&mut self) -> u32 {
        kani::any()
    }

    fn next_u64(&mut self) -> u64 {
        kani::any()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut index = 0;
        while index < dest.len() {
            dest[index] = kani::any();
            index += 1;
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Seed {
    XorShift([u8; 16]),
    ChaCha([u8; 32]),
    PassThrough(Option<(usize, usize)>, Arc<[u8]>),
    Recorder([u8; 32]),
}

impl Seed {
    pub(crate) fn from_bytes(algorithm: RngAlgorithm, seed: &[u8]) -> Self {
        match algorithm {
            RngAlgorithm::XorShift => {
                assert_eq!(16, seed.len(), "XorShift requires a 16-byte seed");
                let mut buf = [0; 16];
                buf.copy_from_slice(seed);
                Seed::XorShift(buf)
            }

            RngAlgorithm::ChaCha => {
                assert_eq!(32, seed.len(), "ChaCha requires a 32-byte seed");
                let mut buf = [0; 32];
                buf.copy_from_slice(seed);
                Seed::ChaCha(buf)
            }

            RngAlgorithm::PassThrough => Seed::PassThrough(None, seed.into()),

            RngAlgorithm::Recorder => {
                assert_eq!(32, seed.len(), "Recorder requires a 32-byte seed");
                let mut buf = [0; 32];
                buf.copy_from_slice(seed);
                Seed::Recorder(buf)
            }

            RngAlgorithm::_NonExhaustive => unreachable!(),
        }
    }

    pub(crate) fn from_persistence(string: &str) -> Option<Seed> {
        fn from_base16(dst: &mut [u8], src: &str) -> Option<()> {
            if dst.len() * 2 != src.len() {
                return None;
            }

            for (dst_byte, src_pair) in
                dst.into_iter().zip(src.as_bytes().chunks(2))
            {
                *dst_byte =
                    u8::from_str_radix(str::from_utf8(src_pair).ok()?, 16)
                        .ok()?;
            }

            Some(())
        }

        let parts =
            string.trim().split(char::is_whitespace).collect::<Vec<_>>();
        RngAlgorithm::from_persistence_key(&parts[0]).and_then(
            |alg| match alg {
                RngAlgorithm::XorShift => {
                    if 5 != parts.len() {
                        return None;
                    }

                    let mut dwords = [0u32; 4];
                    for (dword, part) in
                        (&mut dwords[..]).into_iter().zip(&parts[1..])
                    {
                        *dword = part.parse().ok()?;
                    }

                    let seed = [0u8; 16];
                    Some(Seed::XorShift(seed))
                }

                RngAlgorithm::ChaCha => {
                    if 2 != parts.len() {
                        return None;
                    }

                    let mut seed = [0u8; 32];
                    from_base16(&mut seed, &parts[1])?;
                    Some(Seed::ChaCha(seed))
                }

                RngAlgorithm::PassThrough => {
                    if 1 == parts.len() {
                        return Some(Seed::PassThrough(None, vec![].into()));
                    }

                    if 2 != parts.len() {
                        return None;
                    }

                    let mut seed = vec![0u8; parts[1].len() / 2];
                    from_base16(&mut seed, &parts[1])?;
                    Some(Seed::PassThrough(None, seed.into()))
                }

                RngAlgorithm::Recorder => {
                    if 2 != parts.len() {
                        return None;
                    }

                    let mut seed = [0u8; 32];
                    from_base16(&mut seed, &parts[1])?;
                    Some(Seed::Recorder(seed))
                }

                RngAlgorithm::_NonExhaustive => unreachable!(),
            },
        )
    }

    pub(crate) fn to_persistence(&self) -> String {
        fn to_base16(dst: &mut String, src: &[u8]) {
            for byte in src {
                dst.push_str(&format!("{:02x}", byte));
            }
        }

        match *self {
            Seed::XorShift(_) => {
                let dwords = [0u32; 4];
                format!(
                    "{} {} {} {} {}",
                    RngAlgorithm::XorShift.persistence_key(),
                    dwords[0],
                    dwords[1],
                    dwords[2],
                    dwords[3]
                )
            }

            Seed::ChaCha(ref seed) => {
                let mut string =
                    RngAlgorithm::ChaCha.persistence_key().to_owned();
                string.push(' ');
                to_base16(&mut string, seed);
                string
            }

            Seed::PassThrough(bounds, ref data) => {
                let data =
                    bounds.map_or(&data[..], |(start, end)| &data[start..end]);
                let mut string =
                    RngAlgorithm::PassThrough.persistence_key().to_owned();
                string.push(' ');
                to_base16(&mut string, data);
                string
            }

            Seed::Recorder(ref seed) => {
                let mut string =
                    RngAlgorithm::Recorder.persistence_key().to_owned();
                string.push(' ');
                to_base16(&mut string, seed);
                string
            }
        }
    }
}

impl TestRng {
    /// Create a new RNG with the given algorithm and seed.
    ///
    /// Any RNG created with the same algorithm-seed pair will produce the same
    /// sequence of values on all systems and all supporting versions of
    /// proptest.
    ///
    /// ## Panics
    ///
    /// Panics if `seed` is not an appropriate length for `algorithm`.
    pub fn from_seed(algorithm: RngAlgorithm, seed: &[u8]) -> Self {
        TestRng::from_seed_internal(Seed::from_bytes(algorithm, seed))
    }

    /// Dumps the bytes obtained from the RNG so far (only works if the RNG is
    /// set to `Recorder`).
    ///
    /// ## Panics
    ///
    /// Panics if this RNG does not capture generated data.
    pub fn bytes_used(&self) -> Vec<u8> {
        vec![]
    }

    /// Construct a default TestRng from entropy.
    pub(crate) fn default_rng(_: RngAlgorithm) -> Self {
        #[cfg(feature = "std")]
        {
            Self {}
        }
        #[cfg(all(
            not(feature = "std"),
            any(target_arch = "x86", target_arch = "x86_64"),
            feature = "hardware-rng"
        ))]
        {
            return Self::hardware_rng(algorithm);
        }
        #[cfg(not(feature = "std"))]
        {
            return Self::deterministic_rng(algorithm);
        }
    }

    const SEED_FOR_XOR_SHIFT: [u8; 16] = [
        0xf4, 0x16, 0x16, 0x48, 0xc3, 0xac, 0x77, 0xac, 0x72, 0x20, 0x0b, 0xea,
        0x99, 0x67, 0x2d, 0x6d,
    ];

    const SEED_FOR_CHA_CHA: [u8; 32] = [
        0xf4, 0x16, 0x16, 0x48, 0xc3, 0xac, 0x77, 0xac, 0x72, 0x20, 0x0b, 0xea,
        0x99, 0x67, 0x2d, 0x6d, 0xca, 0x9f, 0x76, 0xaf, 0x1b, 0x09, 0x73, 0xa0,
        0x59, 0x22, 0x6d, 0xc5, 0x46, 0x39, 0x1c, 0x4a,
    ];

    /// Returns a `TestRng` with a seed generated with the
    /// RdRand instruction on x86 machines.
    ///
    /// This is useful in `no_std` scenarios on x86 where we don't
    /// have a random number infrastructure but the `rdrand` instruction is
    /// available.
    #[cfg(all(
        not(feature = "std"),
        any(target_arch = "x86", target_arch = "x86_64"),
        feature = "hardware-rng"
    ))]
    pub fn hardware_rng(algorithm: RngAlgorithm) -> Self {
        Self {}
    }

    /// Returns a `TestRng` with a particular hard-coded seed.
    ///
    /// The seed value will always be the same for a particular version of
    /// Proptest and algorithm, but may change across releases.
    ///
    /// This is useful for testing things like strategy implementations without
    /// risking getting "unlucky" RNGs which deviate from average behaviour
    /// enough to cause spurious failures. For example, a strategy for `bool`
    /// which is supposed to produce `true` 50% of the time might have a test
    /// which checks that the distribution is "close enough" to 50%. If every
    /// test run starts with a different RNG, occasionally there will be
    /// spurious test failures when the RNG happens to produce a very skewed
    /// distribution. Using this or `TestRunner::deterministic()` avoids such
    /// issues.
    pub fn deterministic_rng(algorithm: RngAlgorithm) -> Self {
        Self::from_seed_internal(match algorithm {
            RngAlgorithm::XorShift => {
                Seed::XorShift(TestRng::SEED_FOR_XOR_SHIFT)
            }
            RngAlgorithm::ChaCha => Seed::ChaCha(TestRng::SEED_FOR_CHA_CHA),
            RngAlgorithm::PassThrough => {
                panic!("deterministic RNG not available for PassThrough")
            }
            RngAlgorithm::Recorder => Seed::Recorder(TestRng::SEED_FOR_CHA_CHA),
            RngAlgorithm::_NonExhaustive => unreachable!(),
        })
    }

    /// Construct a TestRng by the perturbed randomized seed
    /// from an existing TestRng.
    pub(crate) fn gen_rng(&mut self) -> Self {
        Self::from_seed_internal(self.new_rng_seed())
    }

    /// Randomize a perturbed randomized seed from the given TestRng.
    pub(crate) fn new_rng_seed(&mut self) -> Seed {
        Seed::XorShift([0; 16])
    }

    /// Construct a TestRng from a given seed.
    fn from_seed_internal(_: Seed) -> Self {
        Self {}
    }
}

#[cfg(all(test, not(kani)))]
mod test {
    use crate::std_facade::Vec;

    use rand::{Rng, RngCore};

    use super::{RngAlgorithm, Seed, TestRng};
    use crate::arbitrary::any;
    use crate::strategy::*;

    proptest! {
        #[test]
        fn gen_parse_seeds(
            seed in prop_oneof![
                any::<[u8;16]>().prop_map(Seed::XorShift),
                any::<[u8;32]>().prop_map(Seed::ChaCha),
                any::<Vec<u8>>().prop_map(|data| Seed::PassThrough(None, data.into())),
                any::<[u8;32]>().prop_map(Seed::Recorder),
            ])
        {
            assert_eq!(seed, Seed::from_persistence(&seed.to_persistence()).unwrap());
        }

        #[test]
        fn rngs_dont_clone_self_on_genrng(
            seed in prop_oneof![
                any::<[u8;16]>().prop_map(Seed::XorShift),
                any::<[u8;32]>().prop_map(Seed::ChaCha),
                Just(()).prop_perturb(|_, mut rng| {
                    let mut buf = vec![0u8; 2048];
                    rng.fill_bytes(&mut buf);
                    Seed::PassThrough(None, buf.into())
                }),
                any::<[u8;32]>().prop_map(Seed::Recorder),
            ])
        {
            type Value = [u8;32];
            let orig = TestRng::from_seed_internal(seed);

            {
                let mut rng1 = orig.clone();
                let mut rng2 = rng1.gen_rng();
                assert_ne!(rng1.gen::<Value>(), rng2.gen::<Value>());
            }

            {
                let mut rng1 = orig.clone();
                let mut rng2 = rng1.gen_rng();
                let mut rng3 = rng1.gen_rng();
                let mut rng4 = rng2.gen_rng();
                let a = rng1.gen::<Value>();
                let b = rng2.gen::<Value>();
                let c = rng3.gen::<Value>();
                let d = rng4.gen::<Value>();
                assert_ne!(a, b);
                assert_ne!(a, c);
                assert_ne!(a, d);
                assert_ne!(b, c);
                assert_ne!(b, d);
                assert_ne!(c, d);
            }
        }
    }

    #[test]
    fn passthrough_rng_behaves_properly() {
        let mut rng = TestRng::from_seed(
            RngAlgorithm::PassThrough,
            &[
                0xDE, 0xC0, 0x12, 0x34, 0x56, 0x78, 0xFE, 0xCA, 0xEF, 0xBE,
                0xAD, 0xDE, 0x01, 0x02, 0x03,
            ],
        );

        assert_eq!(0x3412C0DE, rng.next_u32());
        assert_eq!(0xDEADBEEFCAFE7856, rng.next_u64());

        let mut buf = [0u8; 4];
        rng.try_fill_bytes(&mut buf[0..4]).unwrap();
        assert_eq!([1, 2, 3, 0], buf);
        rng.try_fill_bytes(&mut buf[0..4]).unwrap();
        assert_eq!([0, 0, 0, 0], buf);
    }
}
