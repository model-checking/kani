// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! The representation of a `#[doc(cfg(...))]` attribute.

// FIXME: Once the portability lint RFC is implemented (see tracking issue #41619),
// switch to use those structures instead.

use std::mem;
use std::ops;

use rustc_ast::{LitKind, MetaItem, MetaItemKind, NestedMetaItem};
use rustc_span::symbol::{sym, Symbol};

use rustc_span::Span;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Cfg {
    /// Accepts all configurations.
    True,
    /// Denies all configurations.
    False,
    /// A generic configuration option, e.g., `test` or `target_os = "linux"`.
    Cfg(Symbol, Option<Symbol>),
    /// Negates a configuration requirement, i.e., `not(x)`.
    Not(Box<Cfg>),
    /// Union of a list of configuration requirements, i.e., `any(...)`.
    Any(Vec<Cfg>),
    /// Intersection of a list of configuration requirements, i.e., `all(...)`.
    All(Vec<Cfg>),
}

#[derive(PartialEq, Debug)]
pub(crate) struct InvalidCfgError {
    pub(crate) msg: &'static str,
    pub(crate) span: Span,
}

impl Cfg {
    /// Parses a `NestedMetaItem` into a `Cfg`.
    fn parse_nested(nested_cfg: &NestedMetaItem) -> Result<Cfg, InvalidCfgError> {
        match nested_cfg {
            NestedMetaItem::MetaItem(ref cfg) => Cfg::parse(cfg),
            NestedMetaItem::Literal(ref lit) => {
                Err(InvalidCfgError { msg: "unexpected literal", span: lit.span })
            }
        }
    }

    /// Parses a `MetaItem` into a `Cfg`.
    ///
    /// The `MetaItem` should be the content of the `#[cfg(...)]`, e.g., `unix` or
    /// `target_os = "redox"`.
    ///
    /// If the content is not properly formatted, it will return an error indicating what and where
    /// the error is.
    pub(crate) fn parse(cfg: &MetaItem) -> Result<Cfg, InvalidCfgError> {
        let name = match cfg.ident() {
            Some(ident) => ident.name,
            None => {
                return Err(InvalidCfgError {
                    msg: "expected a single identifier",
                    span: cfg.span,
                });
            }
        };
        match cfg.kind {
            MetaItemKind::Word => Ok(Cfg::Cfg(name, None)),
            MetaItemKind::NameValue(ref lit) => match lit.kind {
                LitKind::Str(value, _) => Ok(Cfg::Cfg(name, Some(value))),
                _ => Err(InvalidCfgError {
                    // FIXME: if the main #[cfg] syntax decided to support non-string literals,
                    // this should be changed as well.
                    msg: "value of cfg option should be a string literal",
                    span: lit.span,
                }),
            },
            MetaItemKind::List(ref items) => {
                let mut sub_cfgs = items.iter().map(Cfg::parse_nested);
                match name {
                    sym::all => sub_cfgs.fold(Ok(Cfg::True), |x, y| Ok(x? & y?)),
                    sym::any => sub_cfgs.fold(Ok(Cfg::False), |x, y| Ok(x? | y?)),
                    sym::not => {
                        if sub_cfgs.len() == 1 {
                            Ok(!sub_cfgs.next().unwrap()?)
                        } else {
                            Err(InvalidCfgError { msg: "expected 1 cfg-pattern", span: cfg.span })
                        }
                    }
                    _ => Err(InvalidCfgError { msg: "invalid predicate", span: cfg.span }),
                }
            }
        }
    }
}

impl ops::Not for Cfg {
    type Output = Cfg;
    fn not(self) -> Cfg {
        match self {
            Cfg::False => Cfg::True,
            Cfg::True => Cfg::False,
            Cfg::Not(cfg) => *cfg,
            s => Cfg::Not(Box::new(s)),
        }
    }
}

impl ops::BitAndAssign for Cfg {
    fn bitand_assign(&mut self, other: Cfg) {
        match (self, other) {
            (&mut Cfg::False, _) | (_, Cfg::True) => {}
            (s, Cfg::False) => *s = Cfg::False,
            (s @ &mut Cfg::True, b) => *s = b,
            (&mut Cfg::All(ref mut a), Cfg::All(ref mut b)) => {
                for c in b.drain(..) {
                    if !a.contains(&c) {
                        a.push(c);
                    }
                }
            }
            (&mut Cfg::All(ref mut a), ref mut b) => {
                if !a.contains(b) {
                    a.push(mem::replace(b, Cfg::True));
                }
            }
            (s, Cfg::All(mut a)) => {
                let b = mem::replace(s, Cfg::True);
                if !a.contains(&b) {
                    a.push(b);
                }
                *s = Cfg::All(a);
            }
            (s, b) => {
                if *s != b {
                    let a = mem::replace(s, Cfg::True);
                    *s = Cfg::All(vec![a, b]);
                }
            }
        }
    }
}

impl ops::BitAnd for Cfg {
    type Output = Cfg;
    fn bitand(mut self, other: Cfg) -> Cfg {
        self &= other;
        self
    }
}

impl ops::BitOrAssign for Cfg {
    fn bitor_assign(&mut self, other: Cfg) {
        match (self, other) {
            (&mut Cfg::True, _) | (_, Cfg::False) => {}
            (s, Cfg::True) => *s = Cfg::True,
            (s @ &mut Cfg::False, b) => *s = b,
            (&mut Cfg::Any(ref mut a), Cfg::Any(ref mut b)) => {
                for c in b.drain(..) {
                    if !a.contains(&c) {
                        a.push(c);
                    }
                }
            }
            (&mut Cfg::Any(ref mut a), ref mut b) => {
                if !a.contains(b) {
                    a.push(mem::replace(b, Cfg::True));
                }
            }
            (s, Cfg::Any(mut a)) => {
                let b = mem::replace(s, Cfg::True);
                if !a.contains(&b) {
                    a.push(b);
                }
                *s = Cfg::Any(a);
            }
            (s, b) => {
                if *s != b {
                    let a = mem::replace(s, Cfg::True);
                    *s = Cfg::Any(vec![a, b]);
                }
            }
        }
    }
}

impl ops::BitOr for Cfg {
    type Output = Cfg;
    fn bitor(mut self, other: Cfg) -> Cfg {
        self |= other;
        self
    }
}
