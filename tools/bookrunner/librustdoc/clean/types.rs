// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use std::cell::RefCell;
use std::default::Default;
use std::hash::Hash;
use std::lazy::SyncOnceCell as OnceCell;
use std::rc::Rc;
use std::sync::Arc;
use std::{slice, vec};

use arrayvec::ArrayVec;

use rustc_ast::attr;
use rustc_ast::util::comments::beautify_doc_string;
use rustc_ast::{self as ast, AttrStyle};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::thin_vec::ThinVec;
use rustc_hir as hir;
use rustc_hir::def::{CtorKind, Res};
use rustc_hir::def_id::{CrateNum, DefId, DefIndex, LOCAL_CRATE};
use rustc_hir::lang_items::LangItem;
use rustc_hir::{BodyId, Mutability};
use rustc_index::vec::IndexVec;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::hygiene::MacroKind;
use rustc_span::source_map::DUMMY_SP;
use rustc_span::symbol::{kw, sym, Ident, Symbol};
use rustc_span::{self};
use rustc_target::abi::VariantIdx;
use rustc_target::spec::abi::Abi;

use crate::clean::cfg::Cfg;
use crate::clean::external_path;
use crate::clean::inline;
use crate::clean::Clean;
use crate::core::DocContext;
use crate::formats::cache::Cache;
use crate::formats::item_type::ItemType;

crate use self::FnRetTy::*;
crate use self::ItemKind::*;
crate use self::Type::{
    Array, BareFunction, BorrowedRef, DynTrait, Generic, ImplTrait, Infer, Primitive, QPath,
    RawPointer, Slice, Tuple,
};
crate use self::Visibility::{Inherited, Public};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
crate enum ItemId {
    /// A "normal" item that uses a [`DefId`] for identification.
    DefId(DefId),
    /// Identifier that is used for auto traits.
    Auto { trait_: DefId, for_: DefId },
    /// Identifier that is used for blanket implementations.
    Blanket { impl_id: DefId, for_: DefId },
    /// Identifier for primitive types.
    Primitive(PrimitiveType, CrateNum),
}

impl ItemId {
    #[inline]
    crate fn is_local(self) -> bool {
        match self {
            ItemId::Auto { for_: id, .. }
            | ItemId::Blanket { for_: id, .. }
            | ItemId::DefId(id) => id.is_local(),
            ItemId::Primitive(_, krate) => krate == LOCAL_CRATE,
        }
    }

    #[inline]
    #[track_caller]
    crate fn expect_def_id(self) -> DefId {
        self.as_def_id()
            .unwrap_or_else(|| panic!("ItemId::expect_def_id: `{:?}` isn't a DefId", self))
    }

    #[inline]
    crate fn as_def_id(self) -> Option<DefId> {
        match self {
            ItemId::DefId(id) => Some(id),
            _ => None,
        }
    }

    #[inline]
    crate fn krate(self) -> CrateNum {
        match self {
            ItemId::Auto { for_: id, .. }
            | ItemId::Blanket { for_: id, .. }
            | ItemId::DefId(id) => id.krate,
            ItemId::Primitive(_, krate) => krate,
        }
    }

    #[inline]
    crate fn index(self) -> Option<DefIndex> {
        match self {
            ItemId::DefId(id) => Some(id.index),
            _ => None,
        }
    }
}

impl From<DefId> for ItemId {
    fn from(id: DefId) -> Self {
        Self::DefId(id)
    }
}

/// The crate currently being documented.
#[derive(Clone, Debug)]
crate struct Crate {
    crate module: Item,
    /// Only here so that they can be filtered through the rustdoc passes.
    crate external_traits: Rc<RefCell<FxHashMap<DefId, TraitWithExtraInfo>>>,
}

// `Crate` is frequently moved by-value. Make sure it doesn't unintentionally get bigger.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(Crate, 64);

/// This struct is used to wrap additional information added by rustdoc on a `trait` item.
#[derive(Clone, Debug)]
crate struct TraitWithExtraInfo {
    crate trait_: Trait,
    crate is_notable: bool,
}

#[derive(Copy, Clone, Debug)]
crate struct ExternalCrate {
    crate crate_num: CrateNum,
}

impl ExternalCrate {}

/// Anything with a source location and set of attributes and, optionally, a
/// name. That is, anything that can be documented. This doesn't correspond
/// directly to the AST's concept of an item; it's a strict superset.
#[derive(Clone, Debug)]
crate struct Item {
    /// The name of this item.
    /// Optional because not every item has a name, e.g. impls.
    crate name: Option<Symbol>,
    crate attrs: Box<Attributes>,
    crate visibility: Visibility,
    /// Information about this item that is specific to what kind of item it is.
    /// E.g., struct vs enum vs function.
    crate kind: Box<ItemKind>,
    crate def_id: ItemId,

    crate cfg: Option<Arc<Cfg>>,
}

// `Item` is used a lot. Make sure it doesn't unintentionally get bigger.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(Item, 56);

crate fn rustc_span(def_id: DefId, tcx: TyCtxt<'_>) -> Span {
    Span::new(def_id.as_local().map_or_else(
        || tcx.def_span(def_id),
        |local| {
            let hir = tcx.hir();
            hir.span_with_body(hir.local_def_id_to_hir_id(local))
        },
    ))
}

impl Item {
    crate fn span(&self, tcx: TyCtxt<'_>) -> Span {
        let kind = match &*self.kind {
            ItemKind::StrippedItem(k) => k,
            _ => &*self.kind,
        };
        match kind {
            ItemKind::ModuleItem(Module { span, .. }) => *span,
            ItemKind::ImplItem(Impl { kind: ImplKind::Auto, .. }) => Span::dummy(),
            ItemKind::ImplItem(Impl { kind: ImplKind::Blanket(_), .. }) => {
                if let ItemId::Blanket { impl_id, .. } = self.def_id {
                    rustc_span(impl_id, tcx)
                } else {
                    panic!("blanket impl item has non-blanket ID")
                }
            }
            _ => {
                self.def_id.as_def_id().map(|did| rustc_span(did, tcx)).unwrap_or_else(Span::dummy)
            }
        }
    }

    crate fn attr_span(&self, tcx: TyCtxt<'_>) -> rustc_span::Span {
        crate::passes::span_of_attrs(&self.attrs).unwrap_or_else(|| self.span(tcx).inner())
    }

    /// Convenience wrapper around [`Self::from_def_id_and_parts`] which converts
    /// `hir_id` to a [`DefId`]
    pub(crate) fn from_hir_id_and_parts(
        hir_id: hir::HirId,
        name: Option<Symbol>,
        kind: ItemKind,
        cx: &mut DocContext<'_>,
    ) -> Item {
        Item::from_def_id_and_parts(cx.tcx.hir().local_def_id(hir_id).to_def_id(), name, kind, cx)
    }

    pub(crate) fn from_def_id_and_parts(
        def_id: DefId,
        name: Option<Symbol>,
        kind: ItemKind,
        cx: &mut DocContext<'_>,
    ) -> Item {
        let ast_attrs = cx.tcx.get_attrs(def_id);

        Self::from_def_id_and_attrs_and_parts(
            def_id,
            name,
            kind,
            box ast_attrs.clean(cx),
            cx,
            ast_attrs.cfg(cx.tcx, &cx.cache.hidden_cfg),
        )
    }

    pub(crate) fn from_def_id_and_attrs_and_parts(
        def_id: DefId,
        name: Option<Symbol>,
        kind: ItemKind,
        attrs: Box<Attributes>,
        cx: &mut DocContext<'_>,
        cfg: Option<Arc<Cfg>>,
    ) -> Item {
        trace!("name={:?}, def_id={:?}", name, def_id);

        Item {
            def_id: def_id.into(),
            kind: box kind,
            name,
            attrs,
            visibility: cx.tcx.visibility(def_id).clean(cx),
            cfg,
        }
    }

    crate fn is_stripped(&self) -> bool {
        match *self.kind {
            StrippedItem(..) => true,
            ImportItem(ref i) => !i.should_be_displayed,
            _ => false,
        }
    }

    /// Returns a documentation-level item type from the item.
    crate fn type_(&self) -> ItemType {
        ItemType::from(self)
    }
}

#[derive(Clone, Debug)]
crate enum ItemKind {
    ExternCrateItem {
        /// The crate's name, *not* the name it's imported as.
        src: Option<Symbol>,
    },
    ImportItem(Import),
    StructItem(Struct),
    UnionItem(Union),
    EnumItem(Enum),
    FunctionItem(Function),
    ModuleItem(Module),
    TypedefItem(Typedef, bool /* is associated type */),
    OpaqueTyItem(OpaqueTy),
    StaticItem(Static),
    ConstantItem(Constant),
    TraitItem(Trait),
    TraitAliasItem(TraitAlias),
    ImplItem(Impl),
    /// A method signature only. Used for required methods in traits (ie,
    /// non-default-methods).
    TyMethodItem(Function),
    /// A method with a body.
    MethodItem(Function, Option<hir::Defaultness>),
    StructFieldItem(Type),
    VariantItem(Variant),
    /// `fn`s from an extern block
    ForeignFunctionItem(Function),
    /// `static`s from an extern block
    ForeignStaticItem(Static),
    /// `type`s from an extern block
    ForeignTypeItem,
    MacroItem(Macro),
    ProcMacroItem(ProcMacro),
    AssocConstItem(Type, Option<ConstantKind>),
    /// An associated item in a trait or trait impl.
    ///
    /// The bounds may be non-empty if there is a `where` clause.
    /// The `Option<Type>` is the default concrete type (e.g. `trait Trait { type Target = usize; }`)
    AssocTypeItem(Vec<GenericBound>, Option<Type>),
    /// An item that has been stripped by a rustdoc pass
    StrippedItem(Box<ItemKind>),
}

impl ItemKind {}

#[derive(Clone, Debug)]
crate struct Module {
    crate items: Vec<Item>,
    crate span: Span,
}

crate struct ListAttributesIter<'a> {
    attrs: slice::Iter<'a, ast::Attribute>,
    current_list: vec::IntoIter<ast::NestedMetaItem>,
    name: Symbol,
}

impl<'a> Iterator for ListAttributesIter<'a> {
    type Item = ast::NestedMetaItem;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(nested) = self.current_list.next() {
            return Some(nested);
        }

        for attr in &mut self.attrs {
            if let Some(list) = attr.meta_item_list() {
                if attr.has_name(self.name) {
                    self.current_list = list.into_iter();
                    if let Some(nested) = self.current_list.next() {
                        return Some(nested);
                    }
                }
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let lower = self.current_list.len();
        (lower, None)
    }
}

crate trait AttributesExt {
    /// Finds an attribute as List and returns the list of attributes nested inside.
    fn lists(&self, name: Symbol) -> ListAttributesIter<'_>;

    fn span(&self) -> Option<rustc_span::Span>;

    fn inner_docs(&self) -> bool;

    fn other_attrs(&self) -> Vec<ast::Attribute>;

    fn cfg(&self, tcx: TyCtxt<'_>, hidden_cfg: &FxHashSet<Cfg>) -> Option<Arc<Cfg>>;
}

impl AttributesExt for [ast::Attribute] {
    fn lists(&self, name: Symbol) -> ListAttributesIter<'_> {
        ListAttributesIter { attrs: self.iter(), current_list: Vec::new().into_iter(), name }
    }

    /// Return the span of the first doc-comment, if it exists.
    fn span(&self) -> Option<rustc_span::Span> {
        self.iter().find(|attr| attr.doc_str().is_some()).map(|attr| attr.span)
    }

    /// Returns whether the first doc-comment is an inner attribute.
    ///
    //// If there are no doc-comments, return true.
    /// FIXME(#78591): Support both inner and outer attributes on the same item.
    fn inner_docs(&self) -> bool {
        self.iter().find(|a| a.doc_str().is_some()).map_or(true, |a| a.style == AttrStyle::Inner)
    }

    fn other_attrs(&self) -> Vec<ast::Attribute> {
        self.iter().filter(|attr| attr.doc_str().is_none()).cloned().collect()
    }

    fn cfg(&self, tcx: TyCtxt<'_>, hidden_cfg: &FxHashSet<Cfg>) -> Option<Arc<Cfg>> {
        let sess = tcx.sess;
        let doc_cfg_active = tcx.features().doc_cfg;
        let doc_auto_cfg_active = tcx.features().doc_auto_cfg;

        fn single<T: IntoIterator>(it: T) -> Option<T::Item> {
            let mut iter = it.into_iter();
            let item = iter.next()?;
            if iter.next().is_some() {
                return None;
            }
            Some(item)
        }

        let mut cfg = if doc_cfg_active || doc_auto_cfg_active {
            let mut doc_cfg = self
                .iter()
                .filter(|attr| attr.has_name(sym::doc))
                .flat_map(|attr| attr.meta_item_list().unwrap_or_else(Vec::new))
                .filter(|attr| attr.has_name(sym::cfg))
                .peekable();
            if doc_cfg.peek().is_some() && doc_cfg_active {
                doc_cfg
                    .filter_map(|attr| Cfg::parse(attr.meta_item()?).ok())
                    .fold(Cfg::True, |cfg, new_cfg| cfg & new_cfg)
            } else if doc_auto_cfg_active {
                self.iter()
                    .filter(|attr| attr.has_name(sym::cfg))
                    .filter_map(|attr| single(attr.meta_item_list()?))
                    .filter_map(|attr| Cfg::parse(attr.meta_item()?).ok())
                    .filter(|cfg| !hidden_cfg.contains(cfg))
                    .fold(Cfg::True, |cfg, new_cfg| cfg & new_cfg)
            } else {
                Cfg::True
            }
        } else {
            Cfg::True
        };

        for attr in self.iter() {
            // #[doc]
            if attr.doc_str().is_none() && attr.has_name(sym::doc) {
                // #[doc(...)]
                if let Some(list) = attr.meta().as_ref().and_then(|mi| mi.meta_item_list()) {
                    for item in list {
                        // #[doc(hidden)]
                        if !item.has_name(sym::cfg) {
                            continue;
                        }
                        // #[doc(cfg(...))]
                        if let Some(cfg_mi) = item
                            .meta_item()
                            .and_then(|item| rustc_expand::config::parse_cfg(item, sess))
                        {
                            match Cfg::parse(cfg_mi) {
                                Ok(new_cfg) => cfg &= new_cfg,
                                Err(e) => {
                                    sess.span_err(e.span, e.msg);
                                }
                            }
                        }
                    }
                }
            }
        }

        // treat #[target_feature(enable = "feat")] attributes as if they were
        // #[doc(cfg(target_feature = "feat"))] attributes as well
        for attr in self.lists(sym::target_feature) {
            if attr.has_name(sym::enable) {
                if let Some(feat) = attr.value_str() {
                    let meta = attr::mk_name_value_item_str(
                        Ident::with_dummy_span(sym::target_feature),
                        feat,
                        DUMMY_SP,
                    );
                    if let Ok(feat_cfg) = Cfg::parse(&meta) {
                        cfg &= feat_cfg;
                    }
                }
            }
        }

        if cfg == Cfg::True { None } else { Some(Arc::new(cfg)) }
    }
}

crate trait NestedAttributesExt {
    /// Returns `true` if the attribute list contains a specific `word`
    fn has_word(self, word: Symbol) -> bool
    where
        Self: std::marker::Sized,
    {
        <Self as NestedAttributesExt>::get_word_attr(self, word).is_some()
    }

    /// Returns `Some(attr)` if the attribute list contains 'attr'
    /// corresponding to a specific `word`
    fn get_word_attr(self, word: Symbol) -> Option<ast::NestedMetaItem>;
}

impl<I> NestedAttributesExt for I
where
    I: IntoIterator<Item = ast::NestedMetaItem>,
{
    fn get_word_attr(self, word: Symbol) -> Option<ast::NestedMetaItem> {
        self.into_iter().find(|attr| attr.is_word() && attr.has_name(word))
    }
}

/// A portion of documentation, extracted from a `#[doc]` attribute.
///
/// Each variant contains the line number within the complete doc-comment where the fragment
/// starts, as well as the Span where the corresponding doc comment or attribute is located.
///
/// Included files are kept separate from inline doc comments so that proper line-number
/// information can be given when a doctest fails. Sugared doc comments and "raw" doc comments are
/// kept separate because of issue #42760.
#[derive(Clone, PartialEq, Eq, Debug)]
crate struct DocFragment {
    crate span: rustc_span::Span,
    /// The module this doc-comment came from.
    ///
    /// This allows distinguishing between the original documentation and a pub re-export.
    /// If it is `None`, the item was not re-exported.
    crate parent_module: Option<DefId>,
    crate doc: Symbol,
    crate kind: DocFragmentKind,
    crate indent: usize,
}

// `DocFragment` is used a lot. Make sure it doesn't unintentionally get bigger.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(DocFragment, 32);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
crate enum DocFragmentKind {
    /// A doc fragment created from a `///` or `//!` doc comment.
    SugaredDoc,
    /// A doc fragment created from a "raw" `#[doc=""]` attribute.
    RawDoc,
}

/// The goal of this function is to apply the `DocFragment` transformation that is required when
/// transforming into the final Markdown, which is applying the computed indent to each line in
/// each doc fragment (a `DocFragment` can contain multiple lines in case of `#[doc = ""]`).
///
/// Note: remove the trailing newline where appropriate
fn add_doc_fragment(out: &mut String, frag: &DocFragment) {
    let s = frag.doc.as_str();
    let mut iter = s.lines();
    if s == "" {
        out.push('\n');
        return;
    }
    while let Some(line) = iter.next() {
        if line.chars().any(|c| !c.is_whitespace()) {
            assert!(line.len() >= frag.indent);
            out.push_str(&line[frag.indent..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
}

/// Collapse a collection of [`DocFragment`]s into one string,
/// handling indentation and newlines as needed.
crate fn collapse_doc_fragments(doc_strings: &[DocFragment]) -> String {
    let mut acc = String::new();
    for frag in doc_strings {
        add_doc_fragment(&mut acc, frag);
    }
    acc.pop();
    acc
}

/// A link that has not yet been rendered.
///
/// This link will be turned into a rendered link by [`Item::links`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct ItemLink {
    /// The original link written in the markdown
    pub(crate) link: String,
    /// The link text displayed in the HTML.
    ///
    /// This may not be the same as `link` if there was a disambiguator
    /// in an intra-doc link (e.g. \[`fn@f`\])
    pub(crate) link_text: String,
    pub(crate) did: DefId,
}

pub(crate) struct RenderedLink {
    /// The text the link was original written as.
    ///
    /// This could potentially include disambiguators and backticks.
    pub(crate) original_text: String,
    /// The text to display in the HTML
    pub(crate) new_text: String,
    /// The URL to put in the `href`
    pub(crate) href: String,
}

/// The attributes on an [`Item`], including attributes like `#[derive(...)]` and `#[inline]`,
/// as well as doc comments.
#[derive(Clone, Debug, Default)]
crate struct Attributes {
    crate doc_strings: Vec<DocFragment>,
    crate other_attrs: Vec<ast::Attribute>,
}

impl Attributes {
    crate fn has_doc_flag(&self, flag: Symbol) -> bool {
        for attr in &self.other_attrs {
            if !attr.has_name(sym::doc) {
                continue;
            }

            if let Some(items) = attr.meta_item_list() {
                if items.iter().filter_map(|i| i.meta_item()).any(|it| it.has_name(flag)) {
                    return true;
                }
            }
        }

        false
    }

    crate fn from_ast(
        attrs: &[ast::Attribute],
        additional_attrs: Option<(&[ast::Attribute], DefId)>,
    ) -> Attributes {
        let mut doc_strings: Vec<DocFragment> = vec![];
        let clean_attr = |(attr, parent_module): (&ast::Attribute, Option<DefId>)| {
            if let Some((value, kind)) = attr.doc_str_and_comment_kind() {
                trace!("got doc_str={:?}", value);
                let value = beautify_doc_string(value, kind);
                let kind = if attr.is_doc_comment() {
                    DocFragmentKind::SugaredDoc
                } else {
                    DocFragmentKind::RawDoc
                };

                let frag =
                    DocFragment { span: attr.span, doc: value, kind, parent_module, indent: 0 };

                doc_strings.push(frag);

                None
            } else {
                Some(attr.clone())
            }
        };

        // Additional documentation should be shown before the original documentation
        let other_attrs = additional_attrs
            .into_iter()
            .map(|(attrs, id)| attrs.iter().map(move |attr| (attr, Some(id))))
            .flatten()
            .chain(attrs.iter().map(|attr| (attr, None)))
            .filter_map(clean_attr)
            .collect();

        Attributes { doc_strings, other_attrs }
    }
    /// Finds all `doc` attributes as NameValues and returns their corresponding values, joined
    /// with newlines.
    crate fn collapsed_doc_value(&self) -> Option<String> {
        if self.doc_strings.is_empty() {
            None
        } else {
            Some(collapse_doc_fragments(&self.doc_strings))
        }
    }
}

impl PartialEq for Attributes {
    fn eq(&self, rhs: &Self) -> bool {
        self.doc_strings == rhs.doc_strings
            && self
                .other_attrs
                .iter()
                .map(|attr| attr.id)
                .eq(rhs.other_attrs.iter().map(|attr| attr.id))
    }
}

impl Eq for Attributes {}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum GenericBound {
    TraitBound(PolyTrait, hir::TraitBoundModifier),
    Outlives(Lifetime),
}

impl GenericBound {
    crate fn maybe_sized(cx: &mut DocContext<'_>) -> GenericBound {
        let did = cx.tcx.require_lang_item(LangItem::Sized, None);
        let empty = cx.tcx.intern_substs(&[]);
        let path = external_path(cx, did, false, vec![], empty);
        inline::record_extern_fqn(cx, did, ItemType::Trait);
        GenericBound::TraitBound(
            PolyTrait { trait_: path, generic_params: Vec::new() },
            hir::TraitBoundModifier::Maybe,
        )
    }

    crate fn is_sized_bound(&self, cx: &DocContext<'_>) -> bool {
        use rustc_hir::TraitBoundModifier as TBM;
        if let GenericBound::TraitBound(PolyTrait { ref trait_, .. }, TBM::None) = *self {
            if Some(trait_.def_id()) == cx.tcx.lang_items().sized_trait() {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct Lifetime(pub(crate) Symbol);

impl Lifetime {
    crate fn statik() -> Lifetime {
        Lifetime(kw::StaticLifetime)
    }

    crate fn elided() -> Lifetime {
        Lifetime(kw::UnderscoreLifetime)
    }
}

#[derive(Clone, Debug)]
crate enum WherePredicate {
    BoundPredicate { ty: Type, bounds: Vec<GenericBound>, bound_params: Vec<Lifetime> },
    RegionPredicate { lifetime: Lifetime, bounds: Vec<GenericBound> },
    EqPredicate { lhs: Type, rhs: Term },
}

impl WherePredicate {
    crate fn get_bounds(&self) -> Option<&[GenericBound]> {
        match *self {
            WherePredicate::BoundPredicate { ref bounds, .. } => Some(bounds),
            WherePredicate::RegionPredicate { ref bounds, .. } => Some(bounds),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum GenericParamDefKind {
    Lifetime { outlives: Vec<Lifetime> },
    Type { did: DefId, bounds: Vec<GenericBound>, default: Option<Box<Type>>, synthetic: bool },
    Const { did: DefId, ty: Box<Type>, default: Option<Box<String>> },
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct GenericParamDef {
    crate name: Symbol,
    crate kind: GenericParamDefKind,
}

// `GenericParamDef` is used in many places. Make sure it doesn't unintentionally get bigger.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(GenericParamDef, 56);

// maybe use a Generic enum and use Vec<Generic>?
#[derive(Clone, Debug, Default)]
crate struct Generics {
    crate params: Vec<GenericParamDef>,
    crate where_predicates: Vec<WherePredicate>,
}

#[derive(Clone, Debug)]
crate struct Function {
    crate decl: FnDecl,
    crate generics: Generics,
    crate header: hir::FnHeader,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct FnDecl {
    crate inputs: Arguments,
    crate output: FnRetTy,
    crate c_variadic: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct Arguments {
    crate values: Vec<Argument>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct Argument {
    crate type_: Type,
    crate name: Symbol,
    /// This field is used to represent "const" arguments from the `rustc_legacy_const_generics`
    /// feature. More information in <https://github.com/rust-lang/rust/issues/83167>.
    crate is_const: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum FnRetTy {
    Return(Type),
    DefaultReturn,
}

#[derive(Clone, Debug)]
crate struct Trait {
    crate unsafety: hir::Unsafety,
    crate items: Vec<Item>,
    crate generics: Generics,
    crate bounds: Vec<GenericBound>,
    crate is_auto: bool,
}

#[derive(Clone, Debug)]
crate struct TraitAlias {
    crate generics: Generics,
    crate bounds: Vec<GenericBound>,
}

/// A trait reference, which may have higher ranked lifetimes.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct PolyTrait {
    crate trait_: Path,
    crate generic_params: Vec<GenericParamDef>,
}

/// Rustdoc's representation of types, mostly based on the [`hir::Ty`].
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum Type {
    /// A named type, which could be a trait.
    ///
    /// This is mostly Rustdoc's version of [`hir::Path`].
    /// It has to be different because Rustdoc's [`PathSegment`] can contain cleaned generics.
    Path { path: Path },
    /// A `dyn Trait` object: `dyn for<'a> Trait<'a> + Send + 'static`
    DynTrait(Vec<PolyTrait>, Option<Lifetime>),
    /// A type parameter.
    Generic(Symbol),
    /// A primitive (aka, builtin) type.
    Primitive(PrimitiveType),
    /// A function pointer: `extern "ABI" fn(...) -> ...`
    BareFunction(Box<BareFunctionDecl>),
    /// A tuple type: `(i32, &str)`.
    Tuple(Vec<Type>),
    /// A slice type (does *not* include the `&`): `[i32]`
    Slice(Box<Type>),
    /// An array type.
    ///
    /// The `String` field is a stringified version of the array's length parameter.
    Array(Box<Type>, String),
    /// A raw pointer type: `*const i32`, `*mut i32`
    RawPointer(Mutability, Box<Type>),
    /// A reference type: `&i32`, `&'a mut Foo`
    BorrowedRef { lifetime: Option<Lifetime>, mutability: Mutability, type_: Box<Type> },

    /// A qualified path to an associated item: `<Type as Trait>::Name`
    QPath {
        name: Symbol,
        self_type: Box<Type>,
        /// FIXME: This is a hack that should be removed; see [this discussion][1].
        ///
        /// [1]: https://github.com/rust-lang/rust/pull/85479#discussion_r635729093
        self_def_id: Option<DefId>,
        trait_: Path,
    },

    /// A type that is inferred: `_`
    Infer,

    /// An `impl Trait`: `impl TraitA + TraitB + ...`
    ImplTrait(Vec<GenericBound>),
}

// `Type` is used a lot. Make sure it doesn't unintentionally get bigger.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(Type, 72);

impl Type {
    crate fn primitive_type(&self) -> Option<PrimitiveType> {
        match *self {
            Primitive(p) | BorrowedRef { type_: box Primitive(p), .. } => Some(p),
            Slice(..) | BorrowedRef { type_: box Slice(..), .. } => Some(PrimitiveType::Slice),
            Array(..) | BorrowedRef { type_: box Array(..), .. } => Some(PrimitiveType::Array),
            Tuple(ref tys) => {
                if tys.is_empty() {
                    Some(PrimitiveType::Unit)
                } else {
                    Some(PrimitiveType::Tuple)
                }
            }
            RawPointer(..) => Some(PrimitiveType::RawPointer),
            BareFunction(..) => Some(PrimitiveType::Fn),
            _ => None,
        }
    }

    crate fn generics(&self) -> Option<Vec<&Type>> {
        match self {
            Type::Path { path, .. } => path.generics(),
            _ => None,
        }
    }

    crate fn projection(&self) -> Option<(&Type, DefId, Symbol)> {
        let (self_, trait_, name) = match self {
            QPath { self_type, trait_, name, .. } => (self_type, trait_, name),
            _ => return None,
        };
        Some((&self_, trait_.def_id(), *name))
    }

    fn inner_def_id(&self, cache: Option<&Cache>) -> Option<DefId> {
        let t: PrimitiveType = match *self {
            Type::Path { ref path } => return Some(path.def_id()),
            DynTrait(ref bounds, _) => return Some(bounds[0].trait_.def_id()),
            Primitive(p) => return cache.and_then(|c| c.primitive_locations.get(&p).cloned()),
            BorrowedRef { type_: box Generic(..), .. } => PrimitiveType::Reference,
            BorrowedRef { ref type_, .. } => return type_.inner_def_id(cache),
            Tuple(ref tys) => {
                if tys.is_empty() {
                    PrimitiveType::Unit
                } else {
                    PrimitiveType::Tuple
                }
            }
            BareFunction(..) => PrimitiveType::Fn,
            Slice(..) => PrimitiveType::Slice,
            Array(..) => PrimitiveType::Array,
            RawPointer(..) => PrimitiveType::RawPointer,
            QPath { ref self_type, .. } => return self_type.inner_def_id(cache),
            Generic(_) | Infer | ImplTrait(_) => return None,
        };
        cache.and_then(|c| Primitive(t).def_id(c))
    }

    /// Use this method to get the [DefId] of a [clean] AST node, including [PrimitiveType]s.
    ///
    /// See [`Self::def_id_no_primitives`] for more.
    ///
    /// [clean]: crate::clean
    crate fn def_id(&self, cache: &Cache) -> Option<DefId> {
        self.inner_def_id(Some(cache))
    }
}

/// A primitive (aka, builtin) type.
///
/// This represents things like `i32`, `str`, etc.
///
/// N.B. This has to be different from [`hir::PrimTy`] because it also includes types that aren't
/// paths, like [`Self::Unit`].
#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
crate enum PrimitiveType {
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Char,
    Bool,
    Str,
    Slice,
    Array,
    Tuple,
    Unit,
    RawPointer,
    Reference,
    Fn,
    Never,
}

impl PrimitiveType {
    crate fn impls(&self, tcx: TyCtxt<'_>) -> &'static ArrayVec<DefId, 4> {
        Self::all_impls(tcx).get(self).expect("missing impl for primitive type")
    }

    crate fn all_impls(tcx: TyCtxt<'_>) -> &'static FxHashMap<PrimitiveType, ArrayVec<DefId, 4>> {
        static CELL: OnceCell<FxHashMap<PrimitiveType, ArrayVec<DefId, 4>>> = OnceCell::new();

        CELL.get_or_init(move || {
            use self::PrimitiveType::*;

            let single = |a: Option<DefId>| a.into_iter().collect();
            let both = |a: Option<DefId>, b: Option<DefId>| -> ArrayVec<_, 4> {
                a.into_iter().chain(b).collect()
            };

            let lang_items = tcx.lang_items();
            map! {
                Isize => single(lang_items.isize_impl()),
                I8 => single(lang_items.i8_impl()),
                I16 => single(lang_items.i16_impl()),
                I32 => single(lang_items.i32_impl()),
                I64 => single(lang_items.i64_impl()),
                I128 => single(lang_items.i128_impl()),
                Usize => single(lang_items.usize_impl()),
                U8 => single(lang_items.u8_impl()),
                U16 => single(lang_items.u16_impl()),
                U32 => single(lang_items.u32_impl()),
                U64 => single(lang_items.u64_impl()),
                U128 => single(lang_items.u128_impl()),
                F32 => both(lang_items.f32_impl(), lang_items.f32_runtime_impl()),
                F64 => both(lang_items.f64_impl(), lang_items.f64_runtime_impl()),
                Char => single(lang_items.char_impl()),
                Bool => single(lang_items.bool_impl()),
                Str => both(lang_items.str_impl(), lang_items.str_alloc_impl()),
                Slice => {
                    lang_items
                        .slice_impl()
                        .into_iter()
                        .chain(lang_items.slice_u8_impl())
                        .chain(lang_items.slice_alloc_impl())
                        .chain(lang_items.slice_u8_alloc_impl())
                        .collect()
                },
                Array => single(lang_items.array_impl()),
                Tuple => ArrayVec::new(),
                Unit => ArrayVec::new(),
                RawPointer => {
                    lang_items
                        .const_ptr_impl()
                        .into_iter()
                        .chain(lang_items.mut_ptr_impl())
                        .chain(lang_items.const_slice_ptr_impl())
                        .chain(lang_items.mut_slice_ptr_impl())
                        .collect()
                },
                Reference => ArrayVec::new(),
                Fn => ArrayVec::new(),
                Never => ArrayVec::new(),
            }
        })
    }

    crate fn as_sym(&self) -> Symbol {
        use PrimitiveType::*;
        match self {
            Isize => sym::isize,
            I8 => sym::i8,
            I16 => sym::i16,
            I32 => sym::i32,
            I64 => sym::i64,
            I128 => sym::i128,
            Usize => sym::usize,
            U8 => sym::u8,
            U16 => sym::u16,
            U32 => sym::u32,
            U64 => sym::u64,
            U128 => sym::u128,
            F32 => sym::f32,
            F64 => sym::f64,
            Str => sym::str,
            Bool => sym::bool,
            Char => sym::char,
            Array => sym::array,
            Slice => sym::slice,
            Tuple => sym::tuple,
            Unit => sym::unit,
            RawPointer => sym::pointer,
            Reference => sym::reference,
            Fn => kw::Fn,
            Never => sym::never,
        }
    }
}

impl From<ast::IntTy> for PrimitiveType {
    fn from(int_ty: ast::IntTy) -> PrimitiveType {
        match int_ty {
            ast::IntTy::Isize => PrimitiveType::Isize,
            ast::IntTy::I8 => PrimitiveType::I8,
            ast::IntTy::I16 => PrimitiveType::I16,
            ast::IntTy::I32 => PrimitiveType::I32,
            ast::IntTy::I64 => PrimitiveType::I64,
            ast::IntTy::I128 => PrimitiveType::I128,
        }
    }
}

impl From<ast::UintTy> for PrimitiveType {
    fn from(uint_ty: ast::UintTy) -> PrimitiveType {
        match uint_ty {
            ast::UintTy::Usize => PrimitiveType::Usize,
            ast::UintTy::U8 => PrimitiveType::U8,
            ast::UintTy::U16 => PrimitiveType::U16,
            ast::UintTy::U32 => PrimitiveType::U32,
            ast::UintTy::U64 => PrimitiveType::U64,
            ast::UintTy::U128 => PrimitiveType::U128,
        }
    }
}

impl From<ast::FloatTy> for PrimitiveType {
    fn from(float_ty: ast::FloatTy) -> PrimitiveType {
        match float_ty {
            ast::FloatTy::F32 => PrimitiveType::F32,
            ast::FloatTy::F64 => PrimitiveType::F64,
        }
    }
}

impl From<ty::IntTy> for PrimitiveType {
    fn from(int_ty: ty::IntTy) -> PrimitiveType {
        match int_ty {
            ty::IntTy::Isize => PrimitiveType::Isize,
            ty::IntTy::I8 => PrimitiveType::I8,
            ty::IntTy::I16 => PrimitiveType::I16,
            ty::IntTy::I32 => PrimitiveType::I32,
            ty::IntTy::I64 => PrimitiveType::I64,
            ty::IntTy::I128 => PrimitiveType::I128,
        }
    }
}

impl From<ty::UintTy> for PrimitiveType {
    fn from(uint_ty: ty::UintTy) -> PrimitiveType {
        match uint_ty {
            ty::UintTy::Usize => PrimitiveType::Usize,
            ty::UintTy::U8 => PrimitiveType::U8,
            ty::UintTy::U16 => PrimitiveType::U16,
            ty::UintTy::U32 => PrimitiveType::U32,
            ty::UintTy::U64 => PrimitiveType::U64,
            ty::UintTy::U128 => PrimitiveType::U128,
        }
    }
}

impl From<ty::FloatTy> for PrimitiveType {
    fn from(float_ty: ty::FloatTy) -> PrimitiveType {
        match float_ty {
            ty::FloatTy::F32 => PrimitiveType::F32,
            ty::FloatTy::F64 => PrimitiveType::F64,
        }
    }
}

impl From<hir::PrimTy> for PrimitiveType {
    fn from(prim_ty: hir::PrimTy) -> PrimitiveType {
        match prim_ty {
            hir::PrimTy::Int(int_ty) => int_ty.into(),
            hir::PrimTy::Uint(uint_ty) => uint_ty.into(),
            hir::PrimTy::Float(float_ty) => float_ty.into(),
            hir::PrimTy::Str => PrimitiveType::Str,
            hir::PrimTy::Bool => PrimitiveType::Bool,
            hir::PrimTy::Char => PrimitiveType::Char,
        }
    }
}

#[derive(Copy, Clone, Debug)]
crate enum Visibility {
    /// `pub`
    Public,
    /// Visibility inherited from parent.
    ///
    /// For example, this is the visibility of private items and of enum variants.
    Inherited,
    /// `pub(crate)`, `pub(super)`, or `pub(in path::to::somewhere)`
    Restricted(DefId),
}

#[derive(Clone, Debug)]
crate struct Struct {
    crate struct_type: CtorKind,
    crate generics: Generics,
    crate fields: Vec<Item>,
    crate fields_stripped: bool,
}

#[derive(Clone, Debug)]
crate struct Union {
    crate generics: Generics,
    crate fields: Vec<Item>,
    crate fields_stripped: bool,
}

/// This is a more limited form of the standard Struct, different in that
/// it lacks the things most items have (name, id, parameterization). Found
/// only as a variant in an enum.
#[derive(Clone, Debug)]
crate struct VariantStruct {
    crate struct_type: CtorKind,
    crate fields: Vec<Item>,
    crate fields_stripped: bool,
}

#[derive(Clone, Debug)]
crate struct Enum {
    crate variants: IndexVec<VariantIdx, Item>,
    crate generics: Generics,
    crate variants_stripped: bool,
}

#[derive(Clone, Debug)]
crate enum Variant {
    CLike,
    Tuple(Vec<Item>),
    Struct(VariantStruct),
}

/// Small wrapper around [`rustc_span::Span`] that adds helper methods
/// and enforces calling [`rustc_span::Span::source_callsite()`].
#[derive(Copy, Clone, Debug)]
crate struct Span(rustc_span::Span);

impl Span {
    /// Wraps a [`rustc_span::Span`]. In case this span is the result of a macro expansion, the
    /// span will be updated to point to the macro invocation instead of the macro definition.
    ///
    /// (See rust-lang/rust#39726)
    crate fn new(sp: rustc_span::Span) -> Self {
        Self(sp.source_callsite())
    }

    crate fn inner(&self) -> rustc_span::Span {
        self.0
    }

    crate fn dummy() -> Self {
        Self(rustc_span::DUMMY_SP)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct Path {
    crate res: Res,
    crate segments: Vec<PathSegment>,
}

impl Path {
    crate fn def_id(&self) -> DefId {
        self.res.def_id()
    }

    crate fn generics(&self) -> Option<Vec<&Type>> {
        self.segments.last().and_then(|seg| {
            if let GenericArgs::AngleBracketed { ref args, .. } = seg.args {
                Some(
                    args.iter()
                        .filter_map(|arg| match arg {
                            GenericArg::Type(ty) => Some(ty),
                            _ => None,
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum GenericArg {
    Lifetime(Lifetime),
    Type(Type),
    Const(Box<Constant>),
    Infer,
}

// `GenericArg` can occur many times in a single `Path`, so make sure it
// doesn't increase in size unexpectedly.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(GenericArg, 80);

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum GenericArgs {
    AngleBracketed { args: Vec<GenericArg>, bindings: ThinVec<TypeBinding> },
    Parenthesized { inputs: Vec<Type>, output: Option<Box<Type>> },
}

// `GenericArgs` is in every `PathSegment`, so its size can significantly
// affect rustdoc's memory usage.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(GenericArgs, 40);

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct PathSegment {
    crate name: Symbol,
    crate args: GenericArgs,
}

// `PathSegment` usually occurs multiple times in every `Path`, so its size can
// significantly affect rustdoc's memory usage.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(PathSegment, 48);

#[derive(Clone, Debug)]
crate struct Typedef {
    crate type_: Type,
    crate generics: Generics,
    /// `type_` can come from either the HIR or from metadata. If it comes from HIR, it may be a type
    /// alias instead of the final type. This will always have the final type, regardless of whether
    /// `type_` came from HIR or from metadata.
    ///
    /// If `item_type.is_none()`, `type_` is guarenteed to come from metadata (and therefore hold the
    /// final type).
    crate item_type: Option<Type>,
}

#[derive(Clone, Debug)]
crate struct OpaqueTy {
    crate bounds: Vec<GenericBound>,
    crate generics: Generics,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct BareFunctionDecl {
    crate unsafety: hir::Unsafety,
    crate generic_params: Vec<GenericParamDef>,
    crate decl: FnDecl,
    crate abi: Abi,
}

#[derive(Clone, Debug)]
crate struct Static {
    crate type_: Type,
    crate mutability: Mutability,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
crate struct Constant {
    crate type_: Type,
    crate kind: ConstantKind,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
crate enum Term {
    Type(Type),
    Constant(Constant),
}

impl Term {
    crate fn ty(&self) -> Option<&Type> {
        if let Term::Type(ty) = self { Some(ty) } else { None }
    }
}

impl From<Type> for Term {
    fn from(ty: Type) -> Self {
        Term::Type(ty)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
crate enum ConstantKind {
    /// This is the wrapper around `ty::Const` for a non-local constant. Because it doesn't have a
    /// `BodyId`, we need to handle it on its own.
    ///
    /// Note that `ty::Const` includes generic parameters, and may not always be uniquely identified
    /// by a DefId. So this field must be different from `Extern`.
    TyConst { expr: String },
    /// A constant (expression) that's not an item or associated item. These are usually found
    /// nested inside types (e.g., array lengths) or expressions (e.g., repeat counts), and also
    /// used to define explicit discriminant values for enum variants.
    Anonymous { body: BodyId },
    /// A constant from a different crate.
    Extern { def_id: DefId },
    /// `const FOO: u32 = ...;`
    Local { def_id: DefId, body: BodyId },
}

#[derive(Clone, Debug)]
crate struct Impl {
    crate generics: Generics,
    crate trait_: Option<Path>,
    crate for_: Type,
    crate items: Vec<Item>,
    crate polarity: ty::ImplPolarity,
    crate kind: ImplKind,
}

#[derive(Clone, Debug)]
crate enum ImplKind {
    Normal,
    Auto,
    Blanket(Box<Type>),
}

impl ImplKind {
    crate fn is_blanket(&self) -> bool {
        matches!(self, ImplKind::Blanket(_))
    }
}

#[derive(Clone, Debug)]
crate struct Import {
    crate kind: ImportKind,
    crate source: ImportSource,
    crate should_be_displayed: bool,
}

impl Import {
    crate fn new_simple(name: Symbol, source: ImportSource, should_be_displayed: bool) -> Self {
        Self { kind: ImportKind::Simple(name), source, should_be_displayed }
    }

    crate fn new_glob(source: ImportSource, should_be_displayed: bool) -> Self {
        Self { kind: ImportKind::Glob, source, should_be_displayed }
    }
}

#[derive(Clone, Debug)]
crate enum ImportKind {
    // use source as str;
    Simple(Symbol),
    // use source::*;
    Glob,
}

#[derive(Clone, Debug)]
crate struct ImportSource {
    crate path: Path,
    crate did: Option<DefId>,
}

#[derive(Clone, Debug)]
crate struct Macro {
    crate source: String,
}

#[derive(Clone, Debug)]
crate struct ProcMacro {
    crate kind: MacroKind,
    crate helpers: Vec<Symbol>,
}

/// An type binding on an associated type (e.g., `A = Bar` in `Foo<A = Bar>` or
/// `A: Send + Sync` in `Foo<A: Send + Sync>`).
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate struct TypeBinding {
    crate name: Symbol,
    crate kind: TypeBindingKind,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
crate enum TypeBindingKind {
    Equality { term: Term },
    Constraint { bounds: Vec<GenericBound> },
}

/// The type, lifetime, or constant that a private type alias's parameter should be
/// replaced with when expanding a use of that type alias.
///
/// For example:
///
/// ```
/// type PrivAlias<T> = Vec<T>;
///
/// pub fn public_fn() -> PrivAlias<i32> { vec![] }
/// ```
///
/// `public_fn`'s docs will show it as returning `Vec<i32>`, since `PrivAlias` is private.
/// [`SubstParam`] is used to record that `T` should be mapped to `i32`.
crate enum SubstParam {
    Type(Type),
    Lifetime(Lifetime),
    Constant(Constant),
}

impl SubstParam {
    crate fn as_ty(&self) -> Option<&Type> {
        if let Self::Type(ty) = self { Some(ty) } else { None }
    }

    crate fn as_lt(&self) -> Option<&Lifetime> {
        if let Self::Lifetime(lt) = self { Some(lt) } else { None }
    }
}
