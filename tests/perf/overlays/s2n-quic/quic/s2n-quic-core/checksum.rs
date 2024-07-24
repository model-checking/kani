// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use core::{fmt, hash::Hasher, num::Wrapping};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

/// Computes the [IP checksum](https://www.rfc-editor.org/rfc/rfc1071) over the given slice of bytes
#[inline]
pub fn checksum(data: &[u8]) -> u16 {
    let mut checksum = Checksum::default();
    checksum.write(data);
    checksum.finish()
}

/// Minimum size for a payload to be considered for platform-specific code
const LARGE_WRITE_LEN: usize = 32;

type Accumulator = u64;
type State = Wrapping<Accumulator>;

/// Platform-specific function for computing a checksum
type LargeWriteFn = for<'a> unsafe fn(&mut State, bytes: &'a [u8]) -> &'a [u8];

#[inline(always)]
fn write_sized_generic<'a, const MAX_LEN: usize, const CHUNK_LEN: usize>(
    state: &mut State,
    mut bytes: &'a [u8],
    on_chunk: impl Fn(&[u8; CHUNK_LEN], &mut Accumulator),
) -> &'a [u8] {
    //= https://www.rfc-editor.org/rfc/rfc1071#section-4.1
    //# The following "C" code algorithm computes the checksum with an inner
    //# loop that sums 16-bits at a time in a 32-bit accumulator.
    //#
    //# in 6
    //#    {
    //#        /* Compute Internet Checksum for "count" bytes
    //#         *         beginning at location "addr".
    //#         */
    //#    register long sum = 0;
    //#
    //#     while( count > 1 )  {
    //#        /*  This is the inner loop */
    //#            sum += * (unsigned short) addr++;
    //#            count -= 2;
    //#    }
    //#
    //#        /*  Add left-over byte, if any */
    //#    if( count > 0 )
    //#            sum += * (unsigned char *) addr;
    //#
    //#        /*  Fold 32-bit sum to 16 bits */
    //#    while (sum>>16)
    //#        sum = (sum & 0xffff) + (sum >> 16);
    //#
    //#    checksum = ~sum;
    //# }

    while bytes.len() >= MAX_LEN {
        // use `get_unchecked` to make it easier for kani to analyze
        let chunks = unsafe { bytes.get_unchecked(..MAX_LEN) };
        bytes = unsafe { bytes.get_unchecked(MAX_LEN..) };

        let mut sum = 0;
        // for each pair of bytes, interpret them as integers and sum them up
        for chunk in chunks.chunks_exact(CHUNK_LEN) {
            let chunk = unsafe {
                // SAFETY: chunks_exact always produces a slice of CHUNK_LEN
                debug_assert_eq!(chunk.len(), CHUNK_LEN);
                &*(chunk.as_ptr() as *const [u8; CHUNK_LEN])
            };
            on_chunk(chunk, &mut sum);
        }
        *state += sum;
    }

    bytes
}

/// Generic implementation of a function that computes a checksum over the given slice
#[inline(always)]
fn write_sized_generic_u16<'a, const LEN: usize>(state: &mut State, bytes: &'a [u8]) -> &'a [u8] {
    write_sized_generic::<LEN, 2>(
        state,
        bytes,
        #[inline(always)]
        |&bytes, acc| {
            *acc += u16::from_ne_bytes(bytes) as Accumulator;
        },
    )
}

#[inline(always)]
fn write_sized_generic_u32<'a, const LEN: usize>(state: &mut State, bytes: &'a [u8]) -> &'a [u8] {
    write_sized_generic::<LEN, 4>(
        state,
        bytes,
        #[inline(always)]
        |&bytes, acc| {
            *acc += u32::from_ne_bytes(bytes) as Accumulator;
        },
    )
}

/// Returns the most optimized function implementation for the current platform
#[inline]
#[cfg(all(feature = "once_cell", not(any(kani, miri))))]
fn probe_write_large() -> LargeWriteFn {
    static LARGE_WRITE_FN: once_cell::sync::Lazy<LargeWriteFn> = once_cell::sync::Lazy::new(|| {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if let Some(fun) = x86::probe() {
                return fun;
            }
        }

        write_sized_generic_u32::<16>
    });

    *LARGE_WRITE_FN
}

#[inline]
#[cfg(not(all(feature = "once_cell", not(any(kani, miri)))))]
fn probe_write_large() -> LargeWriteFn {
    write_sized_generic_u32::<16>
}

/// Computes the [IP checksum](https://www.rfc-editor.org/rfc/rfc1071) over an arbitrary set of inputs
#[derive(Clone, Copy)]
pub struct Checksum {
    state: State,
    partial_write: bool,
    write_large: LargeWriteFn,
}

impl Default for Checksum {
    fn default() -> Self {
        Self {
            state: Default::default(),
            partial_write: false,
            write_large: probe_write_large(),
        }
    }
}

impl fmt::Debug for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut v = *self;
        v.carry();
        f.debug_tuple("Checksum").field(&v.finish()).finish()
    }
}

impl Checksum {
    /// Creates a checksum instance without enabling the native implementation
    #[inline]
    pub fn generic() -> Self {
        Self {
            state: Default::default(),
            partial_write: false,
            write_large: write_sized_generic_u32::<16>,
        }
    }

    /// Writes a single byte to the checksum state
    #[inline]
    fn write_byte(&mut self, byte: u8, shift: bool) {
        if shift {
            self.state += (byte as Accumulator) << 8;
        } else {
            self.state += byte as Accumulator;
        }
    }

    /// Carries all of the bits into a single 16 bit range
    #[inline]
    fn carry(&mut self) {
        #[cfg(kani)]
        self.carry_rfc();
        #[cfg(not(kani))]
        self.carry_optimized();
    }

    /// Carries all of the bits into a single 16 bit range
    ///
    /// This implementation is very similar to the way the RFC is written.
    #[inline]
    #[allow(dead_code)]
    fn carry_rfc(&mut self) {
        let mut state = self.state.0;

        for _ in 0..core::mem::size_of::<Accumulator>() {
            state = (state & 0xffff) + (state >> 16);
        }

        self.state.0 = state;
    }

    /// Carries all of the bits into a single 16 bit range
    ///
    /// This implementation was written after some optimization on the RFC version. It results in
    /// about half the instructions needed as the RFC.
    #[inline]
    #[allow(dead_code)]
    fn carry_optimized(&mut self) {
        let values: [u16; core::mem::size_of::<Accumulator>() / 2] = unsafe {
            // SAFETY: alignment of the State is >= of u16
            debug_assert!(core::mem::align_of::<State>() >= core::mem::align_of::<u16>());
            core::mem::transmute(self.state.0)
        };

        let mut sum = 0u16;

        for value in values {
            let (res, overflowed) = sum.overflowing_add(value);
            sum = res;
            if overflowed {
                sum += 1;
            }
        }

        self.state.0 = sum as _;
    }

    /// Writes bytes to the checksum and ensures any single byte remainders are padded
    #[inline]
    pub fn write_padded(&mut self, bytes: &[u8]) {
        self.write(bytes);

        // write a null byte if `bytes` wasn't 16-bit aligned
        if core::mem::take(&mut self.partial_write) {
            self.write_byte(0, cfg!(target_endian = "little"));
        }
    }

    /// Computes the final checksum
    #[inline]
    pub fn finish(self) -> u16 {
        self.finish_be().to_be()
    }

    #[inline]
    pub fn finish_be(mut self) -> u16 {
        self.carry();

        let value = self.state.0 as u16;
        let value = !value;

        // if value is 0, we need to set it to the max value to indicate the checksum was actually
        // computed
        if value == 0 {
            return 0xffff;
        }

        value
    }
}

impl Hasher for Checksum {
    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        // Check to see if we have a partial write to flush
        if core::mem::take(&mut self.partial_write) {
            let (chunk, remaining) = bytes.split_at(1);
            bytes = remaining;

            // shift the byte if we're on little endian
            self.write_byte(chunk[0], cfg!(target_endian = "little"));
        }

        // Only delegate to the optimized platform function if the payload is big enough
        if bytes.len() >= LARGE_WRITE_LEN {
            bytes = unsafe { (self.write_large)(&mut self.state, bytes) };
        }

        // Fall back on the generic implementation to wrap things up
        //
        // NOTE: We don't use the u32 version with kani as it causes the verification time to
        // increase by quite a bit. We have a separate proof for the functional equivalence of
        // these two configurations.
        #[cfg(not(kani))]
        {
            bytes = write_sized_generic_u32::<4>(&mut self.state, bytes);
        }

        bytes = write_sized_generic_u16::<2>(&mut self.state, bytes);

        // if we only have a single byte left, write it to the state and mark it as a partial write
        if let Some(byte) = bytes.first().copied() {
            self.partial_write = true;
            self.write_byte(byte, cfg!(target_endian = "big"));
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        Self::finish(*self) as _
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::check;

    #[test]
    fn rfc_example_test() {
        //= https://www.rfc-editor.org/rfc/rfc1071#section-3
        //= type=test
        //# We now present explicit examples of calculating a simple 1's
        //# complement sum on a 2's complement machine.  The examples show the
        //# same sum calculated byte by bye, by 16-bits words in normal and
        //# swapped order, and 32 bits at a time in 3 different orders.  All
        //# numbers are in hex.
        //#
        //#               Byte-by-byte    "Normal"  Swapped
        //#                                 Order    Order
        //#
        //#     Byte 0/1:    00   01        0001      0100
        //#     Byte 2/3:    f2   03        f203      03f2
        //#     Byte 4/5:    f4   f5        f4f5      f5f4
        //#     Byte 6/7:    f6   f7        f6f7      f7f6
        //#                 ---  ---       -----     -----
        //#     Sum1:       2dc  1f0       2ddf0     1f2dc
        //#
        //#                  dc   f0        ddf0      f2dc
        //#     Carrys:       1    2           2         1
        //#                  --   --        ----      ----
        //#     Sum2:        dd   f2        ddf2      f2dd
        //#
        //#     Final Swap:  dd   f2        ddf2      ddf2
        let bytes = [0x00, 0x01, 0xf2, 0x03, 0xf4, 0xf5, 0xf6, 0xf7];

        let mut checksum = Checksum::default();
        checksum.write(&bytes);
        checksum.carry();

        assert_eq!((checksum.state.0 as u16).to_le_bytes(), [0xdd, 0xf2]);
        assert_eq!((!rfc_c_port(&bytes)).to_be_bytes(), [0xdd, 0xf2]);
    }

    fn rfc_c_port(data: &[u8]) -> u16 {
        //= https://www.rfc-editor.org/rfc/rfc1071#section-4.1
        //= type=test
        //# The following "C" code algorithm computes the checksum with an inner
        //# loop that sums 16-bits at a time in a 32-bit accumulator.
        //#
        //# in 6
        //#    {
        //#        /* Compute Internet Checksum for "count" bytes
        //#         *         beginning at location "addr".
        //#         */
        //#    register long sum = 0;
        //#
        //#     while( count > 1 )  {
        //#        /*  This is the inner loop */
        //#            sum += * (unsigned short) addr++;
        //#            count -= 2;
        //#    }
        //#
        //#        /*  Add left-over byte, if any */
        //#    if( count > 0 )
        //#            sum += * (unsigned char *) addr;
        //#
        //#        /*  Fold 32-bit sum to 16 bits */
        //#    while (sum>>16)
        //#        sum = (sum & 0xffff) + (sum >> 16);
        //#
        //#    checksum = ~sum;
        //# }

        let mut addr = data.as_ptr();
        let mut count = data.len();

        unsafe {
            let mut sum = 0u32;

            while count > 1 {
                let value = u16::from_be_bytes([*addr, *addr.add(1)]);
                sum = sum.wrapping_add(value as u32);
                addr = addr.add(2);
                count -= 2;
            }

            if count > 0 {
                let value = u16::from_be_bytes([*addr, 0]);
                sum = sum.wrapping_add(value as u32);
            }

            while sum >> 16 != 0 {
                sum = (sum & 0xffff) + (sum >> 16);
            }

            !(sum as u16)
        }
    }

    // Reduce the length to 4 for Kani until
    // https://github.com/model-checking/kani/issues/3030 is fixed
    #[cfg(any(kani, miri))]
    const LEN: usize = if cfg!(kani) { 4 } else { 32 };

    /// * Compares the implementation to a port of the C code defined in the RFC
    /// * Ensures partial writes are correctly handled, even if they're not at a 16 bit boundary
    #[test]
    #[cfg_attr(kani, kani::proof, kani::unwind(9), kani::solver(minisat))]
    fn differential() {
        #[cfg(any(kani, miri))]
        type Bytes = crate::testing::InlineVec<u8, LEN>;
        #[cfg(not(any(kani, miri)))]
        type Bytes = Vec<u8>;

        check!()
            .with_type::<(usize, Bytes)>()
            .for_each(|(index, bytes)| {
                let index = if bytes.is_empty() {
                    0
                } else {
                    *index % bytes.len()
                };
                let (a, b) = bytes.split_at(index);
                let mut cs = Checksum::default();
                cs.write(a);
                cs.write(b);

                let mut rfc_value = rfc_c_port(bytes);
                if rfc_value == 0 {
                    rfc_value = 0xffff;
                }

                assert_eq!(rfc_value.to_be_bytes(), cs.finish().to_be_bytes());
            });
    }

    /// Shows that using the u32+u16 methods is the same as only using u16
    #[test]
    #[cfg_attr(kani, kani::proof, kani::unwind(9), kani::solver(kissat))]
    fn u32_u16_differential() {
        #[cfg(any(kani, miri))]
        type Bytes = crate::testing::InlineVec<u8, 8>;
        #[cfg(not(any(kani, miri)))]
        type Bytes = Vec<u8>;

        check!().with_type::<Bytes>().for_each(|bytes| {
            let a = {
                let mut cs = Checksum::generic();
                let bytes = write_sized_generic_u32::<4>(&mut cs.state, bytes);
                write_sized_generic_u16::<2>(&mut cs.state, bytes);
                cs.finish()
            };

            let b = {
                let mut cs = Checksum::generic();
                write_sized_generic_u16::<2>(&mut cs.state, bytes);
                cs.finish()
            };

            assert_eq!(a, b);
        });
    }

    /// Shows that RFC carry implementation is the same as the optimized version
    #[test]
    #[cfg_attr(kani, kani::proof, kani::unwind(9), kani::solver(kissat))]
    fn carry_differential() {
        check!().with_type::<u64>().cloned().for_each(|state| {
            let mut opt = Checksum::generic();
            opt.state.0 = state;
            opt.carry_optimized();

            let mut rfc = Checksum::generic();
            rfc.state.0 = state;
            rfc.carry_rfc();

            assert_eq!(opt.state.0, rfc.state.0);
        });
    }
}
