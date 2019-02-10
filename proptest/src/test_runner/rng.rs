//-
// Copyright 2017, 2018, 2019 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::{u8, str};
use crate::std_facade::{Arc, String, Vec, ToOwned};

use byteorder::{ByteOrder, LittleEndian};
use rand::{self, RngCore, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use rand_chacha::ChaChaRng;

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
    /// It is the user's responsibility to ensure that the seed is "big
    /// enough". Proptest makes no guarantees about how much data is consumed
    /// from the seed for any particular strategy. If the seed is exhausted,
    /// the RNG panics.
    ///
    /// Note that in cases where a new RNG is to be derived from an existing
    /// one, *the data is split evenly between them*, regardless of how much
    /// entropy is actually needed. This means that combinators like
    /// `prop_perturb` and `prop_flat_map` can require extremely large inputs.
    PassThrough,
    #[allow(missing_docs)] #[doc(hidden)] _NonExhaustive,
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
            RngAlgorithm::_NonExhaustive => unreachable!(),
        }
    }

    pub(crate) fn from_persistence_key(k: &str) -> Option<Self> {
        match k {
            "xs" => Some(RngAlgorithm::XorShift),
            "cc" => Some(RngAlgorithm::ChaCha),
            "pt" => Some(RngAlgorithm::PassThrough),
            _ => None,
        }
    }
}

/// Proptest's random number generator.
#[derive(Clone, Debug)]
pub struct TestRng { rng: TestRngImpl }

#[derive(Clone, Debug)]
enum TestRngImpl {
    XorShift(XorShiftRng),
    ChaCha(ChaChaRng),
    PassThrough { off: usize, end: usize, data: Arc<[u8]> },
}

impl RngCore for TestRng {
    fn next_u32(&mut self) -> u32 {
        match &mut self.rng {
            &mut TestRngImpl::XorShift(ref mut rng) =>
                rng.next_u32(),

            &mut TestRngImpl::ChaCha(ref mut rng) =>
                rng.next_u32(),

            &mut TestRngImpl::PassThrough { .. } => {
                let mut buf = [0; 4];
                self.fill_bytes(&mut buf[..]);
                LittleEndian::read_u32(&buf[..])
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        match &mut self.rng {
            &mut TestRngImpl::XorShift(ref mut rng) =>
                rng.next_u64(),

            &mut TestRngImpl::ChaCha(ref mut rng) =>
                rng.next_u64(),

            &mut TestRngImpl::PassThrough { .. } => {
                let mut buf = [0; 8];
                self.fill_bytes(&mut buf[..]);
                LittleEndian::read_u64(&buf[..])
            },
        }
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        match &mut self.rng {
            &mut TestRngImpl::XorShift(ref mut rng) =>
                rng.fill_bytes(dest),

            &mut TestRngImpl::ChaCha(ref mut rng) =>
                rng.fill_bytes(dest),

            &mut TestRngImpl::PassThrough { ref mut off, end, ref data } => {
                assert!(*off + dest.len() <= end, "out of PassThrough data");
                dest.copy_from_slice(&data[*off..*off + dest.len()]);
                *off += dest.len();
            },
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        match self.rng {
            TestRngImpl::XorShift(ref mut rng) =>
                rng.try_fill_bytes(dest),

            TestRngImpl::ChaCha(ref mut rng) =>
                rng.try_fill_bytes(dest),

            TestRngImpl::PassThrough { ref mut off, end, ref data } => {
                if *off + dest.len() > end {
                    return Err(rand::Error::new(
                        rand::ErrorKind::Unavailable,
                        "out of PassThrough data"));
                }

                dest.copy_from_slice(&data[*off..*off + dest.len()]);
                *off += dest.len();
                Ok(())
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Seed {
    XorShift([u8; 16]),
    ChaCha([u8; 32]),
    PassThrough(Option<(usize, usize)>, Arc<[u8]>),
}

impl Seed {
    pub(crate) fn from_bytes(algorithm: RngAlgorithm, seed: &[u8]) -> Self {
        match algorithm {
            RngAlgorithm::XorShift => {
                assert_eq!(16, seed.len(), "XorShift requires a 16-byte seed");
                let mut buf = [0; 16];
                buf.copy_from_slice(seed);
                Seed::XorShift(buf)
            },

            RngAlgorithm::ChaCha => {
                assert_eq!(32, seed.len(), "ChaCha requires a 32-byte seed");
                let mut buf = [0; 32];
                buf.copy_from_slice(seed);
                Seed::ChaCha(buf)
            },

            RngAlgorithm::PassThrough =>
                Seed::PassThrough(None, seed.into()),

            RngAlgorithm::_NonExhaustive => unreachable!(),
        }
    }

    pub(crate) fn from_persistence(string: &str) -> Option<Seed> {
        fn from_base16(dst: &mut [u8], src: &str) -> Option<()> {
            if dst.len() * 2 != src.len() {
                return None;
            }

            for (dst_byte, src_pair) in dst.into_iter().zip(src.as_bytes().chunks(2)) {
                *dst_byte = u8::from_str_radix(str::from_utf8(src_pair).ok()?, 16).ok()?;
            }

            Some(())
        }

        let parts = string.trim().split(char::is_whitespace).collect::<Vec<_>>();
        RngAlgorithm::from_persistence_key(&parts[0]).and_then(|alg| match alg {
            RngAlgorithm::XorShift => {
                if 5 != parts.len() {
                    return None;
                }

                let mut dwords = [0u32; 4];
                for (dword, part) in (&mut dwords[..]).into_iter().zip(&parts[1..]) {
                    *dword = part.parse().ok()?;
                }

                let mut seed = [0u8; 16];
                LittleEndian::write_u32_into(&dwords[..], &mut seed[..]);
                Some(Seed::XorShift(seed))
            },

            RngAlgorithm::ChaCha => {
                if 2 != parts.len() {
                    return None;
                }

                let mut seed = [0u8; 32];
                from_base16(&mut seed, &parts[1])?;
                Some(Seed::ChaCha(seed))
            },

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
            },

            RngAlgorithm::_NonExhaustive => unreachable!(),
        })
    }

    pub(crate) fn to_persistence(&self) -> String {
        fn to_base16(dst: &mut String, src: &[u8]) {
            for byte in src {
                dst.push_str(&format!("{:02x}", byte));
            }
        }

        match *self {
            Seed::XorShift(ref seed) => {
                let mut dwords = [0u32; 4];
                LittleEndian::read_u32_into(seed, &mut dwords[..]);
                format!("{} {} {} {} {}",
                        RngAlgorithm::XorShift.persistence_key(),
                        dwords[0], dwords[1], dwords[2], dwords[3])
            },

            Seed::ChaCha(ref seed) => {
                let mut string =
                    RngAlgorithm::ChaCha.persistence_key().to_owned();
                string.push(' ');
                to_base16(&mut string, seed);
                string
            },

            Seed::PassThrough(bounds, ref data) => {
                let data = bounds
                    .map_or(&data[..], |(start, end)| &data[start..end]);
                let mut string =
                    RngAlgorithm::PassThrough.persistence_key().to_owned();
                string.push(' ');
                to_base16(&mut string, data);
                string
            },
        }
    }
}

impl TestRng {
    /// Construct a default TestRng from entropy.
    pub(crate) fn default_rng() -> Self {
        #[cfg(feature = "std")]
        {
            use rand::FromEntropy;
            Self { rng: TestRngImpl::ChaCha(ChaChaRng::from_entropy()) }
        }
        #[cfg(not(feature = "std"))]
        Self::from_seed(Seed::XorShift([
            0x19, 0x3a, 0x67, 0x54, // x
            0x69, 0xd4, 0xa7, 0xa8, // y
            0x05, 0x0e, 0x83, 0x97, // z
            0xbb, 0xa7, 0x3b, 0x11, // w
        ]))
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
        self.set_seed(seed.clone());
        seed
    }

    /// Randomize a perturbed randomized seed from the given TestRng.
    pub(crate) fn new_rng_seed(&mut self) -> Seed {
        match self.rng {
            TestRngImpl::XorShift(ref mut rng) => {
                let mut seed = rng.gen::<[u8;16]>();

                // Directly using XorShiftRng::from_seed() at this point would
                // result in rng and the returned value being exactly the same.
                // Perturb the seed with some arbitrary values to prevent this.
                for word in seed.chunks_mut(4) {
                    word[3] ^= 0xde;
                    word[2] ^= 0xad;
                    word[1] ^= 0xbe;
                    word[0] ^= 0xef;
                }

                Seed::XorShift(seed)
            },

            TestRngImpl::ChaCha(ref mut rng) =>
                Seed::ChaCha(rng.gen()),

            TestRngImpl::PassThrough { ref mut off, ref mut end, ref data } => {
                let len = *end - *off;
                let child_start = *off + len / 2;
                let child_end = *off + len;
                *end = child_start;
                Seed::PassThrough(Some((child_start, child_end)), Arc::clone(data))
            },
        }
    }

    /// Construct a TestRng from a given seed.
    fn from_seed(seed: Seed) -> Self {
        Self { rng: match seed {
            Seed::XorShift(seed) =>
                TestRngImpl::XorShift(XorShiftRng::from_seed(seed)),

            Seed::ChaCha(seed) =>
                TestRngImpl::ChaCha(ChaChaRng::from_seed(seed)),

            Seed::PassThrough(bounds, data) => {
                let (start, end) = bounds.unwrap_or((0, data.len()));
                TestRngImpl::PassThrough { off: start, end, data }
            }
        } }
    }
}

#[cfg(test)]
mod test {
    use crate::std_facade::Vec;

    use rand::{Rng, RngCore};

    use super::{Seed, TestRng};
    use crate::arbitrary::any;
    use crate::strategy::*;

    proptest! {
        #[test]
        fn gen_parse_seeds(
            seed in prop_oneof![
                any::<[u8;16]>().prop_map(Seed::XorShift),
                any::<[u8;32]>().prop_map(Seed::ChaCha),
                any::<Vec<u8>>().prop_map(|data| Seed::PassThrough(None, data.into())),
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
            ])
        {
            type Value = [u8;32];
            let orig = TestRng::from_seed(seed);

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
}
