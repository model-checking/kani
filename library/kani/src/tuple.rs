// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! Support for arbitrary tuples where each element implements
//! `kani::Arbitrary`. Tuples of size up to 11 are supported in this
//! file.

use crate::Arbitrary;

/// This macro implements `kani::Arbitrary` on a tuple whose elements
/// already implement `kani::Arbitrary` by running `kani::any()` on
/// each index of the tuple.
macro_rules! tuple {
    ($($typ:ident),*) => {
	impl<$($typ : Arbitrary),*>  Arbitrary for ($($typ,)*) {
            #[inline(always)]
	    fn any() -> Self {
		($(crate::any::<$typ>(),)*)
            }
        }

    }
}

tuple!(A);
tuple!(A, B);
tuple!(A, B, C);
tuple!(A, B, C, D);
tuple!(A, B, C, D, E);
tuple!(A, B, C, D, E, F);
tuple!(A, B, C, D, E, F, G);
tuple!(A, B, C, D, E, F, G, H);
tuple!(A, B, C, D, E, F, G, H, I);
tuple!(A, B, C, D, E, F, G, H, I, J);
tuple!(A, B, C, D, E, F, G, H, I, J, K);
tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
