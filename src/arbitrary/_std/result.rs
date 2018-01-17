//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::result`.

use std::fmt;
use std::result::IntoIter;

use strategy::*;
use strategy::statics::static_map;
use result::*;
use arbitrary::*;

// We assume that `MaybeOk` is canonical as it's the most likely Strategy
// a user wants.

arbitrary!([A: Arbitrary, B: Arbitrary] Result<A, B>,
    MaybeOk<A::Strategy, B::Strategy>,
    product_type![Probability, A::Parameters, B::Parameters];
    args => {
        let product_unpack![prob, a, b] = args;
        let (p, a, b) = (prob, any_with::<A>(a), any_with::<B>(b));
        maybe_ok_weighted(p, a, b)
    }
);

impl<A: fmt::Debug, E: Arbitrary> functor::ArbitraryF1<A> for Result<A, E>
where
    E::Strategy: 'static
{
    type Parameters = product_type![Probability, E::Parameters];

    fn lift1_with<AS>(base: AS, args: Self::Parameters) -> BoxedStrategy<Self>
    where
        AS: Strategy + 'static,
        AS::Value: ValueTree<Value = A>
    {
        let product_unpack![prob, e] = args;
        let (p, a, e) = (prob, base, any_with::<E>(e));
        maybe_ok_weighted(p, a, e).boxed()
    }
}

impl<A: fmt::Debug, B: fmt::Debug> functor::ArbitraryF2<A, B>
for Result<A, B> {
    type Parameters = Probability;

    fn lift2_with<AS, BS>(fst: AS, snd: BS, args: Self::Parameters)
        -> BoxedStrategy<Self>
    where
        AS: Strategy + 'static,
        AS::Value: ValueTree<Value = A>,
        BS: Strategy + 'static,
        BS::Value: ValueTree<Value = B>
    {
        maybe_ok_weighted(args, fst, snd).boxed()
    }
}

arbitrary!([A: Arbitrary] IntoIter<A>,
    SMapped<Result<A, ()>, Self>,
    <Result<A, ()> as Arbitrary>::Parameters;
    args => static_map(any_with::<Result<A, ()>>(args), Result::into_iter)
);

lift1!(['static] IntoIter<A>, Probability; base, args => {
    maybe_ok_weighted(args, base, Just(())).prop_map(Result::into_iter)
});

#[cfg(test)]
mod test {
    no_panic_test!(
        result    => Result<u8, u16>,
        into_iter => IntoIter<u8>
    );
}