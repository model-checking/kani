//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::net`.

use std::net::*;

use crate::strategy::*;
use crate::strategy::statics::static_map;
use crate::arbitrary::*;

// TODO: Can we design a workable semantic for PBT wrt. actual networking
// connections?

arbitrary!(AddrParseError; "".parse::<Ipv4Addr>().unwrap_err());

arbitrary!(Ipv4Addr,
    TupleUnion<(
        W<Just<Self>>,
        W<Just<Self>>,
        W<MapInto<StrategyFor<u32>, Self>>
    )>;
    prop_oneof![
        1  => Just(Self::new(0, 0, 0, 0)),
        4  => Just(Self::new(127, 0, 0, 1)),
        10 => any::<u32>().prop_map_into()
    ]
);

arbitrary!(Ipv6Addr,
    TupleUnion<(
        W<SMapped<Ipv4Addr, Self>>,
        W<MapInto<StrategyFor<[u16; 8]>, Self>>
    )>;
    prop_oneof![
        2 => static_map(any::<Ipv4Addr>(), |ip| ip.to_ipv6_mapped()),
        1 => any::<[u16; 8]>().prop_map_into()
    ]
);

arbitrary!(SocketAddrV4, SMapped<(Ipv4Addr, u16), Self>;
    static_map(any::<(Ipv4Addr, u16)>(), |(a, b)| Self::new(a, b))
);

arbitrary!(SocketAddrV6, SMapped<(Ipv6Addr, u16, u32, u32), Self>;
    static_map(any::<(Ipv6Addr, u16, u32, u32)>(),
        |(a, b, c, d)| Self::new(a, b, c, d))
);

arbitrary!(IpAddr,
    TupleUnion<(W<MapInto<StrategyFor<Ipv4Addr>, Self>>,
                W<MapInto<StrategyFor<Ipv6Addr>, Self>>)>;
    prop_oneof![
        any::<Ipv4Addr>().prop_map_into(),
        any::<Ipv6Addr>().prop_map_into()
    ]
);

arbitrary!(Shutdown,
    TupleUnion<(W<Just<Self>>, W<Just<Self>>, W<Just<Self>>)>;
    {
        use std::net::Shutdown::*;
        prop_oneof![Just(Both), Just(Read), Just(Write)]
    }
);
arbitrary!(SocketAddr,
    TupleUnion<(W<MapInto<StrategyFor<SocketAddrV4>, Self>>,
                W<MapInto<StrategyFor<SocketAddrV6>, Self>>)>;
    prop_oneof![
        any::<SocketAddrV4>().prop_map_into(),
        any::<SocketAddrV6>().prop_map_into()
    ]
);

#[cfg(feature = "unstable")]
arbitrary!(Ipv6MulticastScope,
    TupleUnion<( W<Just<Self>>, W<Just<Self>>, W<Just<Self>>
               , W<Just<Self>>, W<Just<Self>>, W<Just<Self>>, W<Just<Self>>)>;
    {
        use std::net::Ipv6MulticastScope::*;
        prop_oneof![
            Just(InterfaceLocal),
            Just(LinkLocal),
            Just(RealmLocal),
            Just(AdminLocal),
            Just(SiteLocal),
            Just(OrganizationLocal),
            Just(Global),
        ]
    }
);

#[cfg(test)]
mod test {
    no_panic_test!(
        addr_parse_error => AddrParseError,
        ipv4_addr => Ipv4Addr,
        ipv6_addr => Ipv6Addr,
        socket_addr_v4 => SocketAddrV4,
        socket_addr_v6 => SocketAddrV6,
        ip_addr => IpAddr,
        shutdown => Shutdown,
        socket_addr => SocketAddr
    );

    #[cfg(feature = "unstable")]
    no_panic_test!(
        ipv6_multicast_scope => Ipv6MulticastScope
    );
}
