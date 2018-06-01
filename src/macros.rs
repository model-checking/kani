//-
// Copyright 2017, 2018 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Macros for internal use to reduce boilerplate.

// Pervasive internal sugar
macro_rules! mapfn {
    ($(#[$meta:meta])* [$($vis:tt)*]
     fn $name:ident[$($gen:tt)*]($parm:ident: $input:ty) -> $output:ty {
         $($body:tt)*
     }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug)]
        $($vis)* struct $name;
        impl $($gen)* ::strategy::statics::MapFn<$input> for $name {
            type Output = $output;
            fn apply(&self, $parm: $input) -> $output {
                $($body)*
            }
        }
    }
}

macro_rules! delegate_vt_0 {
    () => {
        fn current(&self) -> Self::Value {
            self.0.current()
        }

        fn simplify(&mut self) -> bool {
            self.0.simplify()
        }

        fn complicate(&mut self) -> bool {
            self.0.complicate()
        }
    }
}

macro_rules! opaque_strategy_wrapper {
    ($(#[$smeta:meta])* pub struct $stratname:ident
     [$($sgen:tt)*][$($swhere:tt)*]
     ($innerstrat:ty) -> $stratvtty:ty;

     $(#[$vmeta:meta])* pub struct $vtname:ident
     [$($vgen:tt)*][$($vwhere:tt)*]
     ($innervt:ty) -> $actualty:ty;
    ) => {
        $(#[$smeta])* pub struct $stratname $($sgen)* ($innerstrat)
            $($swhere)*;

        $(#[$vmeta])* pub struct $vtname $($vgen)* ($innervt) $($vwhere)*;

        impl $($sgen)* Strategy for $stratname $($sgen)* $($swhere)* {
            type Tree = $stratvtty;
            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                self.0.new_value(runner).map($vtname)
            }
        }

        impl $($vgen)* ValueTree for $vtname $($vgen)* $($vwhere)* {
            type Value = $actualty;

            delegate_vt_0!();
        }
    }
}

// Example: unwrap_or!(result, err => handle_err(err));
macro_rules! unwrap_or {
    ($unwrap: expr, $err: ident => $on_err: expr) => {
        match $unwrap {
            Ok(ok) => ok,
            Err($err) => $on_err,
        }
    };
}
