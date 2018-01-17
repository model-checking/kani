//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::ops`.

use std::ops::*;

use strategy::Strategy;
use strategy::statics::static_map;
use arbitrary::*;

arbitrary!(RangeFull; ..);
wrap_ctor!(RangeFrom, |a| a..);
wrap_ctor!(RangeTo, |a| ..a);

#[cfg(feature = "unstable")]
wrap_ctor!(RangeToInclusive, |a| ..=a);

#[cfg(feature = "unstable")]
arbitrary!(
    [A: PartialOrd + Arbitrary] RangeInclusive<A>,
    SMapped<(A, A), Self>, product_type![A::Parameters, A::Parameters];
    args => static_map(any_with(args),
        |(a, b)| if b < a { b..=a } else { a..=b })
);

#[cfg(feature = "unstable")]
lift1!([PartialOrd] RangeInclusive<A>; base => {
    let base = ::std::sync::Arc::new(base);
    (base.clone(), base).prop_map(|(a, b)| if b < a { b..=a } else { a..=b })
});

arbitrary!(
    [A: PartialOrd + Arbitrary] Range<A>,
    SMapped<(A, A), Self>, product_type![A::Parameters, A::Parameters];
    args => static_map(any_with::<(A, A)>(args),
        |(a, b)| if b < a { b..a } else { a..b })
);

lift1!([PartialOrd] Range<A>; base => {
    let base = ::std::sync::Arc::new(base);
    (base.clone(), base).prop_map(|(a, b)| if b < a { b..a } else { a..b })
});

#[cfg(feature = "unstable")]
arbitrary!(
    [Y: Arbitrary, R: Arbitrary] GeneratorState<Y, R>,
    TupleUnion<(W<SMapped<Y, Self>>, W<SMapped<R, Self>>)>,
    product_type![Y::Parameters, R::Parameters];
    args => {
        let product_unpack![y, r] = args;
        prop_oneof![
            static_map(any_with(y), GeneratorState::Yielded),
            static_map(any_with(r), GeneratorState::Complete)
        ]
    }
);

#[cfg(feature = "unstable")]
impl<A: Debug + 'static, B: Debug + 'static>
    functor::ArbitraryF2<A, B>
for GeneratorState<A, B> {
    type Parameters = ();

    fn lift2_with<AS, BS>(fst: AS, snd: BS, _args: Self::Parameters)
        -> BoxedStrategy<Self>
    where
        AS: Strategy + 'static,
        AS::Value: ValueTree<Value = A>,
        BS: Strategy + 'static,
        BS::Value: ValueTree<Value = B>
    {
        prop_oneof![
            fst.prop_map(GeneratorState::Yielded),
            snd.prop_map(GeneratorState::Complete)
        ].boxed()
    }
}

#[cfg(test)]
mod test {
    no_panic_test!(
        range_full => RangeFull,
        range_from => RangeFrom<usize>,
        range_to   => RangeTo<usize>,
        range      => Range<usize>
    );

    #[cfg(feature = "unstable")]
    no_panic_test!(
        range_to_inclusive => RangeToInclusive<usize>,
        range_inclusive => RangeInclusive<usize>,
        generator_state => GeneratorState<u32, u64>
    );
}