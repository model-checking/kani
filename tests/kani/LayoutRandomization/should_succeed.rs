// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Kani now has -Zrandomize-layout activated by default.
// This test checks that it doesn't affect repr(C) and Kani is still bit-precise

// kani-flags: --randomize-layout

macro_rules! make_structs {
  ($i:ident) => {
      #[repr(C)]
      struct $i {
          a: u32,
          b: u16,
      }
  };

  ($i:ident, $($rest:ident),+) => {
      make_structs!($i);
      make_structs!($($rest),+);
  };
}

// We only need Foo1 to be clone
impl Clone for Foo1 {
    fn clone(&self) -> Self {
        Self { a: self.a, b: self.b }
    }
}

macro_rules! check_against {
  ($to_check:ident, $current_against:ty) => {
      {
          let other: $current_against = unsafe {std::mem::transmute($to_check.clone())};
          assert_eq!($to_check.a, other.a);
          assert_eq!($to_check.b, other.b);
      }
  };
  ($to_check:ident, $current_against:ty, $($rest:ty),+) => {
      check_against!($to_check, $current_against);
      check_against!($to_check, $($rest),+);
  };
}

make_structs!(
    Foo1, Foo2, Foo3, Foo4, Foo5, Foo6, Foo7, Foo8, Foo9, Foo10, Foo11, Foo12, Foo13, Foo14, Foo15,
    Foo16, Foo17, Foo18, Foo19, Foo20, Foo21, Foo22, Foo23, Foo24, Foo25, Foo26, Foo27, Foo28,
    Foo29, Foo30, Foo31, Foo32, Foo33, Foo34, Foo35, Foo36, Foo37, Foo38, Foo39, Foo40, Foo41,
    Foo42, Foo43, Foo44, Foo45, Foo46, Foo47, Foo48, Foo49, Foo50, Foo51, Foo52, Foo53, Foo54,
    Foo55, Foo56, Foo57, Foo58, Foo59, Foo60, Foo61, Foo62, Foo63, Foo64, Foo65, Foo66, Foo67,
    Foo68, Foo69, Foo70, Foo71, Foo72, Foo73, Foo74, Foo75, Foo76, Foo77, Foo78, Foo79, Foo80,
    Foo81, Foo82, Foo83, Foo84, Foo85, Foo86, Foo87, Foo88, Foo89, Foo90, Foo91, Foo92, Foo93,
    Foo94, Foo95, Foo96, Foo97, Foo98, Foo99, Foo100
);

#[kani::proof]
fn main() {
    let a: u32 = kani::any();
    let b: u16 = kani::any();
    let base = Foo1 { a, b };
    check_against!(
        base, Foo2, Foo3, Foo4, Foo5, Foo6, Foo7, Foo8, Foo9, Foo10, Foo11, Foo12, Foo13, Foo14,
        Foo15, Foo16, Foo17, Foo18, Foo19, Foo20, Foo21, Foo22, Foo23, Foo24, Foo25, Foo26, Foo27,
        Foo28, Foo29, Foo30, Foo31, Foo32, Foo33, Foo34, Foo35, Foo36, Foo37, Foo38, Foo39, Foo40,
        Foo41, Foo42, Foo43, Foo44, Foo45, Foo46, Foo47, Foo48, Foo49, Foo50, Foo51, Foo52, Foo53,
        Foo54, Foo55, Foo56, Foo57, Foo58, Foo59, Foo60, Foo61, Foo62, Foo63, Foo64, Foo65, Foo66,
        Foo67, Foo68, Foo69, Foo70, Foo71, Foo72, Foo73, Foo74, Foo75, Foo76, Foo77, Foo78, Foo79,
        Foo80, Foo81, Foo82, Foo83, Foo84, Foo85, Foo86, Foo87, Foo88, Foo89, Foo90, Foo91, Foo92,
        Foo93, Foo94, Foo95, Foo96, Foo97, Foo98, Foo99, Foo100
    );
}
