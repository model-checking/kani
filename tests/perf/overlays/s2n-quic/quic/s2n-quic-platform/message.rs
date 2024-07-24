// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use core::{alloc::Layout, ptr::NonNull};
use s2n_quic_core::{inet::datagram, io::tx, path};

#[cfg(s2n_quic_platform_cmsg)]
pub mod cmsg;
#[cfg(s2n_quic_platform_socket_mmsg)]
pub mod mmsg;
#[cfg(s2n_quic_platform_socket_msg)]
pub mod msg;
pub mod simple;

pub mod default {
    cfg_if::cfg_if! {
        if #[cfg(s2n_quic_platform_socket_mmsg)] {
            pub use super::mmsg::*;
        } else if #[cfg(s2n_quic_platform_socket_msg)] {
            pub use super::msg::*;
        } else {
            pub use super::simple::*;
        }
    }
}

/// Tracks allocations of message ring buffer state
pub struct Storage {
    ptr: NonNull<u8>,
    layout: Layout,
}

/// Safety: the ring buffer controls access to the underlying storage
unsafe impl Send for Storage {}
/// Safety: the ring buffer controls access to the underlying storage
unsafe impl Sync for Storage {}

impl Storage {
    #[inline]
    pub fn new(layout: Layout) -> Self {
        unsafe {
            let ptr = alloc::alloc::alloc_zeroed(layout);
            let ptr = NonNull::new(ptr).expect("could not allocate message storage");
            Self { layout, ptr }
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Asserts that the pointer is in bounds of the allocation
    #[inline]
    pub fn check_bounds<T: Sized>(&self, ptr: *mut T) {
        let start = self.as_ptr();
        let end = unsafe {
            // Safety: pointer is allocated with the self.layout
            start.add(self.layout.size())
        };
        let allocation_range = start..=end;
        let actual_end_ptr = ptr as *mut u8;
        debug_assert!(allocation_range.contains(&actual_end_ptr));
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        unsafe {
            // Safety: pointer was allocated with self.layout
            alloc::alloc::dealloc(self.as_ptr(), self.layout)
        }
    }
}

/// An abstract message that can be sent and received on a network
pub trait Message: 'static + Copy {
    type Handle: path::Handle;

    const SUPPORTS_GSO: bool;
    const SUPPORTS_ECN: bool;
    const SUPPORTS_FLOW_LABELS: bool;

    /// Allocates `entries` messages, each with `payload_len` bytes
    fn alloc(entries: u32, payload_len: u32, offset: usize) -> Storage;

    /// Returns the length of the payload
    fn payload_len(&self) -> usize;

    /// Sets the payload length for the message
    ///
    /// # Safety
    /// This method should only set the payload less than or
    /// equal to its initially allocated size.
    unsafe fn set_payload_len(&mut self, payload_len: usize);

    /// Validates that the `source` message can be replicated to `dest`.
    ///
    /// # Panics
    ///
    /// This panics when the messages cannot be replicated
    fn validate_replication(source: &Self, dest: &Self);

    /// Returns a mutable pointer for the message payload
    fn payload_ptr_mut(&mut self) -> *mut u8;

    /// Returns a mutable slice for the message payload
    #[inline]
    fn payload_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.payload_ptr_mut(), self.payload_len()) }
    }

    /// Sets the segment size for the message payload
    fn set_segment_size(&mut self, _size: usize) {
        panic!("cannot use GSO on the current platform");
    }

    /// Resets the message for future use
    ///
    /// # Safety
    /// This method should only set the MTU to the original value
    unsafe fn reset(&mut self, mtu: usize);

    /// Reads the message as an RX packet
    fn rx_read(&mut self, local_address: &path::LocalAddress) -> Option<RxMessage<Self::Handle>>;

    /// Writes the message into the TX packet
    fn tx_write<M: tx::Message<Handle = Self::Handle>>(
        &mut self,
        message: M,
    ) -> Result<usize, tx::Error>;
}

pub struct RxMessage<'a, Handle: Copy> {
    /// The received header for the message
    pub header: datagram::Header<Handle>,
    /// The number of segments inside the message
    pub segment_size: usize,
    /// The full payload of the message
    pub payload: &'a mut [u8],
}

impl<'a, Handle: Copy> RxMessage<'a, Handle> {
    #[inline]
    pub fn for_each<F: FnMut(datagram::Header<Handle>, &mut [u8])>(self, mut on_packet: F) {
        // `chunks_mut` doesn't know what to do with zero-sized segments so return early
        if self.segment_size == 0 {
            return;
        }

        for segment in self.payload.chunks_mut(self.segment_size) {
            on_packet(self.header, segment);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::check;

    #[test]
    #[cfg_attr(kani, kani::proof, kani::unwind(17), kani::solver(minisat))]
    fn rx_message_test() {
        let path = bolero::gen::<path::RemoteAddress>();
        let ecn = bolero::gen();
        let segment_size = bolero::gen();
        let max_payload_len = if cfg!(kani) { 16 } else { u16::MAX as usize };
        let payload_len = 0..=max_payload_len;

        check!()
            .with_generator((path, ecn, segment_size, payload_len))
            .cloned()
            .for_each(|(path, ecn, segment_size, payload_len)| {
                let mut payload = vec![0u8; payload_len];
                let rx_message = RxMessage {
                    header: datagram::Header { path, ecn },
                    segment_size,
                    payload: &mut payload,
                };

                rx_message.for_each(|header, segment| {
                    assert_eq!(header.path, path);
                    assert_eq!(header.ecn, ecn);
                    assert!(segment.len() <= payload_len);
                    assert!(segment.len() <= segment_size);
                })
            })
    }
}
