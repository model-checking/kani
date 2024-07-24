// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use bolero::check;
use core::mem::zeroed;
use s2n_quic_core::inet::{SocketAddress, Unspecified};

fn test_msghdr<F: FnOnce(&mut msghdr)>(f: F) {
    const PAYLOAD_LEN: usize = 16;

    let mut msghdr = unsafe { zeroed::<msghdr>() };

    let mut msgname = unsafe { zeroed::<sockaddr_in6>() };
    msghdr.msg_name = &mut msgname as *mut _ as *mut _;
    msghdr.msg_namelen = size_of::<sockaddr_in6>() as _;

    let mut iovec = unsafe { zeroed::<iovec>() };

    let mut payload = [0u8; PAYLOAD_LEN];
    iovec.iov_base = &mut payload as *mut _ as *mut _;
    iovec.iov_len = 1;

    msghdr.msg_iov = &mut iovec;

    let mut msg_control = <cmsg::Storage<{ cmsg::MAX_LEN }>>::default();
    msghdr.msg_controllen = msg_control.len() as _;
    msghdr.msg_control = msg_control.as_mut_ptr() as *mut _;

    unsafe {
        msghdr.reset(PAYLOAD_LEN);
    }

    f(&mut msghdr);
}

#[cfg(kani)]
#[allow(dead_code)] // Avoid warning when using stubs.
mod stubs {
    use s2n_quic_core::inet::AncillaryData;

    pub fn collect(_iter: crate::message::cmsg::decode::Iter) -> AncillaryData {
        let ancillary_data = kani::any();

        ancillary_data
    }
}

#[test]
#[cfg_attr(kani, kani::proof, kani::solver(cadical), kani::unwind(17))]
fn address_inverse_pair_test() {
    check!()
        .with_type::<SocketAddress>()
        .cloned()
        .for_each(|addr| {
            test_msghdr(|message| {
                message.set_remote_address(&addr);

                assert_eq!(message.remote_address(), Some(addr));
            });
        });
}

#[test]
#[cfg_attr(
    kani,
    kani::proof,
    kani::solver(minisat),
    kani::unwind(65),
    // it's safe to stub out cmsg::decode since the cmsg result isn't actually checked in this particular test
    kani::stub(cmsg::decode::collect, stubs::collect)
)]
fn handle_get_set_test() {
    check!()
        .with_generator((
            gen::<Handle>(),
            1..=crate::features::gso::MaxSegments::MAX.into(),
        ))
        .cloned()
        .for_each(|(handle, segment_size)| {
            test_msghdr(|message| {
                handle.update_msg_hdr(message);

                if segment_size > 1 {
                    message.set_segment_size(segment_size);
                }

                let (header, _cmsg) = message.header().unwrap();

                assert_eq!(header.path.remote_address, handle.remote_address);

                // no need to check this on kani since we abstract the decode() function to avoid performance issues
                #[cfg(not(kani))]
                {
                    if features::pktinfo::IS_SUPPORTED
                        && !handle.local_address.ip().is_unspecified()
                    {
                        assert_eq!(header.path.local_address.ip(), handle.local_address.ip());
                    }
                }

                // reset the message and ensure everything is zeroed
                unsafe {
                    message.reset(0);
                }

                let (header, _cmsg) = message.header().unwrap();
                assert!(header.path.remote_address.is_unspecified());
            });
        });
}
