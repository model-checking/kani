// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use crate::clean::{
    inline, Clean, Generic, GenericArg, GenericArgs, ImportSource, Item, ItemKind, Lifetime, Path,
    PathSegment, Primitive, PrimitiveType, Type, TypeBinding, Visibility,
};
use crate::core::DocContext;
use crate::formats::item_type::ItemType;

use rustc_ast as ast;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::subst::{GenericArgKind, SubstsRef};
use rustc_middle::ty::{self, DefIdTree, TyCtxt};
use rustc_span::symbol::{kw, sym, Symbol};
use std::mem;

fn external_generic_args(
    cx: &mut DocContext<'_>,
    did: DefId,
    has_self: bool,
    bindings: Vec<TypeBinding>,
    substs: SubstsRef<'_>,
) -> GenericArgs {
    let mut skip_self = has_self;
    let mut ty_kind = None;
    let args: Vec<_> = substs
        .iter()
        .filter_map(|kind| match kind.unpack() {
            GenericArgKind::Lifetime(lt) => match lt.kind() {
                ty::ReLateBound(_, ty::BoundRegion { kind: ty::BrAnon(_), .. }) => {
                    Some(GenericArg::Lifetime(Lifetime::elided()))
                }
                _ => lt.clean(cx).map(GenericArg::Lifetime),
            },
            GenericArgKind::Type(_) if skip_self => {
                skip_self = false;
                None
            }
            GenericArgKind::Type(ty) => {
                ty_kind = Some(ty.kind());
                Some(GenericArg::Type(ty.clean(cx)))
            }
            GenericArgKind::Const(ct) => Some(GenericArg::Const(Box::new(ct.clean(cx)))),
        })
        .collect();

    if cx.tcx.fn_trait_kind_from_lang_item(did).is_some() {
        let inputs = match ty_kind.unwrap() {
            ty::Tuple(tys) => tys.iter().map(|t| t.clean(cx)).collect(),
            _ => return GenericArgs::AngleBracketed { args, bindings: bindings.into() },
        };
        let output = None;
        // FIXME(#20299) return type comes from a projection now
        // match types[1].kind {
        //     ty::Tuple(ref v) if v.is_empty() => None, // -> ()
        //     _ => Some(types[1].clean(cx))
        // };
        GenericArgs::Parenthesized { inputs, output }
    } else {
        GenericArgs::AngleBracketed { args, bindings: bindings.into() }
    }
}

pub(super) fn external_path(
    cx: &mut DocContext<'_>,
    did: DefId,
    has_self: bool,
    bindings: Vec<TypeBinding>,
    substs: SubstsRef<'_>,
) -> Path {
    let def_kind = cx.tcx.def_kind(did);
    let name = cx.tcx.item_name(did);
    Path {
        res: Res::Def(def_kind, did),
        segments: vec![PathSegment {
            name,
            args: external_generic_args(cx, did, has_self, bindings, substs),
        }],
    }
}

crate fn qpath_to_string(p: &hir::QPath<'_>) -> String {
    let segments = match *p {
        hir::QPath::Resolved(_, path) => &path.segments,
        hir::QPath::TypeRelative(_, segment) => return segment.ident.to_string(),
        hir::QPath::LangItem(lang_item, ..) => return lang_item.name().to_string(),
    };

    let mut s = String::new();
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            s.push_str("::");
        }
        if seg.ident.name != kw::PathRoot {
            s.push_str(seg.ident.as_str());
        }
    }
    s
}

crate fn build_deref_target_impls(cx: &mut DocContext<'_>, items: &[Item], ret: &mut Vec<Item>) {
    let tcx = cx.tcx;

    for item in items {
        let target = match *item.kind {
            ItemKind::TypedefItem(ref t, true) => &t.type_,
            _ => continue,
        };

        if let Some(prim) = target.primitive_type() {
            let _prof_timer = cx.tcx.sess.prof.generic_activity("build_primitive_inherent_impls");
            for &did in prim.impls(tcx).iter().filter(|did| !did.is_local()) {
                inline::build_impl(cx, None, did, None, ret);
            }
        } else if let Type::Path { path } = target {
            let did = path.def_id();
            if !did.is_local() {
                inline::build_impls(cx, None, did, None, ret);
            }
        }
    }
}

crate fn name_from_pat(p: &hir::Pat<'_>) -> Symbol {
    use rustc_hir::*;
    debug!("trying to get a name from pattern: {:?}", p);

    Symbol::intern(&match p.kind {
        PatKind::Wild | PatKind::Struct(..) => return kw::Underscore,
        PatKind::Binding(_, _, ident, _) => return ident.name,
        PatKind::TupleStruct(ref p, ..) | PatKind::Path(ref p) => qpath_to_string(p),
        PatKind::Or(pats) => {
            pats.iter().map(|p| name_from_pat(p).to_string()).collect::<Vec<String>>().join(" | ")
        }
        PatKind::Tuple(elts, _) => format!(
            "({})",
            elts.iter().map(|p| name_from_pat(p).to_string()).collect::<Vec<String>>().join(", ")
        ),
        PatKind::Box(p) => return name_from_pat(&*p),
        PatKind::Ref(p, _) => return name_from_pat(&*p),
        PatKind::Lit(..) => {
            warn!(
                "tried to get argument name from PatKind::Lit, which is silly in function arguments"
            );
            return Symbol::intern("()");
        }
        PatKind::Range(..) => return kw::Underscore,
        PatKind::Slice(begin, ref mid, end) => {
            let begin = begin.iter().map(|p| name_from_pat(p).to_string());
            let mid = mid.as_ref().map(|p| format!("..{}", name_from_pat(&**p))).into_iter();
            let end = end.iter().map(|p| name_from_pat(p).to_string());
            format!("[{}]", begin.chain(mid).chain(end).collect::<Vec<_>>().join(", "))
        }
    })
}

crate fn print_const(cx: &DocContext<'_>, n: &ty::Const<'_>) -> String {
    match n.val() {
        ty::ConstKind::Unevaluated(ty::Unevaluated { def, substs: _, promoted }) => {
            let mut s = if let Some(def) = def.as_local() {
                let hir_id = cx.tcx.hir().local_def_id_to_hir_id(def.did);
                print_const_expr(cx.tcx, cx.tcx.hir().body_owned_by(hir_id))
            } else {
                inline::print_inlined_const(cx.tcx, def.did)
            };
            if let Some(promoted) = promoted {
                s.push_str(&format!("::{:?}", promoted))
            }
            s
        }
        _ => {
            let mut s = n.to_string();
            // array lengths are obviously usize
            if s.ends_with("_usize") {
                let n = s.len() - "_usize".len();
                s.truncate(n);
                if s.ends_with(": ") {
                    let n = s.len() - ": ".len();
                    s.truncate(n);
                }
            }
            s
        }
    }
}

crate fn print_const_expr(tcx: TyCtxt<'_>, body: hir::BodyId) -> String {
    let hir = tcx.hir();
    let value = &hir.body(body).value;

    let snippet = if !value.span.from_expansion() {
        tcx.sess.source_map().span_to_snippet(value.span).ok()
    } else {
        None
    };

    snippet.unwrap_or_else(|| rustc_hir_pretty::id_to_string(&hir, body.hir_id))
}

/// Given a type Path, resolve it to a Type using the TyCtxt
crate fn resolve_type(cx: &mut DocContext<'_>, path: Path) -> Type {
    debug!("resolve_type({:?})", path);

    match path.res {
        Res::PrimTy(p) => Primitive(PrimitiveType::from(p)),
        Res::SelfTy { .. } if path.segments.len() == 1 => Generic(kw::SelfUpper),
        Res::Def(DefKind::TyParam, _) if path.segments.len() == 1 => Generic(path.segments[0].name),
        _ => {
            let _ = register_res(cx, path.res);
            Type::Path { path }
        }
    }
}

/// If `res` has a documentation page associated, store it in the cache.
///
/// This is later used by [`href()`] to determine the HTML link for the item.
///
/// [`href()`]: crate::html::format::href
crate fn register_res(cx: &mut DocContext<'_>, res: Res) -> DefId {
    use DefKind::*;
    debug!("register_res({:?})", res);

    let (did, kind) = match res {
        // These should be added to the cache using `record_extern_fqn`.
        Res::Def(
            kind @ (AssocTy | AssocFn | AssocConst | Variant | Fn | TyAlias | Enum | Trait | Struct
            | Union | Mod | ForeignTy | Const | Static | Macro(..) | TraitAlias),
            i,
        ) => (i, kind.into()),
        // This is part of a trait definition; document the trait.
        Res::SelfTy { trait_: Some(trait_def_id), .. } => (trait_def_id, ItemType::Trait),
        // This is an inherent impl; it doesn't have its own page.
        Res::SelfTy { trait_: None, alias_to: Some((impl_def_id, _)) } => return impl_def_id,
        Res::SelfTy { trait_: None, alias_to: None }
        | Res::PrimTy(_)
        | Res::ToolMod
        | Res::SelfCtor(_)
        | Res::Local(_)
        | Res::NonMacroAttr(_)
        | Res::Err => return res.def_id(),
        Res::Def(
            TyParam | ConstParam | Ctor(..) | ExternCrate | Use | ForeignMod | AnonConst
            | InlineConst | OpaqueTy | Field | LifetimeParam | GlobalAsm | Impl | Closure
            | Generator,
            id,
        ) => return id,
    };
    if did.is_local() {
        return did;
    }
    inline::record_extern_fqn(cx, did, kind);
    if let ItemType::Trait = kind {
        inline::record_extern_trait(cx, did);
    }
    did
}

crate fn resolve_use_source(cx: &mut DocContext<'_>, path: Path) -> ImportSource {
    ImportSource {
        did: if path.res.opt_def_id().is_none() { None } else { Some(register_res(cx, path.res)) },
        path,
    }
}

crate fn enter_impl_trait<F, R>(cx: &mut DocContext<'_>, f: F) -> R
where
    F: FnOnce(&mut DocContext<'_>) -> R,
{
    let old_bounds = mem::take(&mut cx.impl_trait_bounds);
    let r = f(cx);
    assert!(cx.impl_trait_bounds.is_empty());
    cx.impl_trait_bounds = old_bounds;
    r
}

/// Checks for the existence of `hidden` in the attribute below if `flag` is `sym::hidden`:
///
/// ```
/// #[doc(hidden)]
/// pub fn foo() {}
/// ```
///
/// This function exists because it runs on `hir::Attributes` whereas the other is a
/// `clean::Attributes` method.
crate fn has_doc_flag(attrs: ty::Attributes<'_>, flag: Symbol) -> bool {
    attrs.iter().any(|attr| {
        attr.has_name(sym::doc)
            && attr.meta_item_list().map_or(false, |l| rustc_attr::list_contains_name(&l, flag))
    })
}

pub(super) fn display_macro_source(
    _cx: &mut DocContext<'_>,
    _name: Symbol,
    _def: &ast::MacroDef,
    _def_id: DefId,
    _vis: Visibility,
) -> String {
    unimplemented!("no rendering supported")
}
