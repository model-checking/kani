use self::MemberDescriptionFactory::*;
use self::RecursiveTypeDescription::*;

use super::namespace::mangled_name_of_instance;
use super::type_names::{compute_debuginfo_type_name, compute_debuginfo_vtable_name};
use super::utils::{
    create_DIArray, debug_context, get_namespace_for_item, is_node_local_to_unit, DIB,
};
use super::CrateDebugContext;

use crate::abi;
use crate::common::CodegenCx;
use crate::llvm;
use crate::llvm::debuginfo::{
    DIArray, DICompositeType, DIDescriptor, DIFile, DIFlags, DILexicalBlock, DIScope, DIType,
    DebugEmissionKind,
};
use crate::value::Value;

use cstr::cstr;
use rustc_codegen_ssa::debuginfo::type_names::cpp_like_debuginfo;
use rustc_codegen_ssa::traits::*;
use rustc_data_structures::fingerprint::Fingerprint;
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::stable_hasher::{HashStable, StableHasher};
use rustc_fs_util::path_to_c_string;
use rustc_hir::def::CtorKind;
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_index::vec::{Idx, IndexVec};
use rustc_middle::mir::{self, GeneratorLayout};
use rustc_middle::ty::layout::{self, IntegerExt, LayoutOf, PrimitiveExt, TyAndLayout};
use rustc_middle::ty::subst::GenericArgKind;
use rustc_middle::ty::{
    self, AdtKind, GeneratorSubsts, Instance, ParamEnv, Ty, TyCtxt, COMMON_VTABLE_ENTRIES,
};
use rustc_middle::{bug, span_bug};
use rustc_query_system::ich::NodeIdHashingMode;
use rustc_session::config::{self, DebugInfo};
use rustc_span::symbol::Symbol;
use rustc_span::FileNameDisplayPreference;
use rustc_span::{self, SourceFile, SourceFileHash, Span};
use rustc_target::abi::{Abi, Align, HasDataLayout, Integer, TagEncoding};
use rustc_target::abi::{Int, Pointer, F32, F64};
use rustc_target::abi::{Primitive, Size, VariantIdx, Variants};
use tracing::debug;

use libc::{c_longlong, c_uint};
use std::collections::hash_map::Entry;
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};
use std::iter;
use std::path::{Path, PathBuf};
use std::ptr;

impl PartialEq for llvm::Metadata {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for llvm::Metadata {}

impl Hash for llvm::Metadata {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        (self as *const Self).hash(hasher);
    }
}

impl fmt::Debug for llvm::Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self as *const Self).fmt(f)
    }
}

// From DWARF 5.
// See http://www.dwarfstd.org/ShowIssue.php?issue=140129.1.
const DW_LANG_RUST: c_uint = 0x1c;
#[allow(non_upper_case_globals)]
const DW_ATE_boolean: c_uint = 0x02;
#[allow(non_upper_case_globals)]
const DW_ATE_float: c_uint = 0x04;
#[allow(non_upper_case_globals)]
const DW_ATE_signed: c_uint = 0x05;
#[allow(non_upper_case_globals)]
const DW_ATE_unsigned: c_uint = 0x07;
#[allow(non_upper_case_globals)]
const DW_ATE_unsigned_char: c_uint = 0x08;

pub const UNKNOWN_LINE_NUMBER: c_uint = 0;
pub const UNKNOWN_COLUMN_NUMBER: c_uint = 0;

pub const NO_SCOPE_METADATA: Option<&DIScope> = None;

mod unique_type_id {
    use super::*;
    use rustc_arena::DroplessArena;

    #[derive(Copy, Hash, Eq, PartialEq, Clone)]
    pub(super) struct UniqueTypeId(u32);

    // The `&'static str`s in this type actually point into the arena.
    //
    // The `FxHashMap`+`Vec` pair could be replaced by `FxIndexSet`, but #75278
    // found that to regress performance up to 2% in some cases. This might be
    // revisited after further improvements to `indexmap`.
    #[derive(Default)]
    pub(super) struct TypeIdInterner {
        arena: DroplessArena,
        names: FxHashMap<&'static str, UniqueTypeId>,
        strings: Vec<&'static str>,
    }

    impl TypeIdInterner {
        #[inline]
        pub(super) fn intern(&mut self, string: &str) -> UniqueTypeId {
            if let Some(&name) = self.names.get(string) {
                return name;
            }

            let name = UniqueTypeId(self.strings.len() as u32);

            // `from_utf8_unchecked` is safe since we just allocated a `&str` which is known to be
            // UTF-8.
            let string: &str =
                unsafe { std::str::from_utf8_unchecked(self.arena.alloc_slice(string.as_bytes())) };
            // It is safe to extend the arena allocation to `'static` because we only access
            // these while the arena is still alive.
            let string: &'static str = unsafe { &*(string as *const str) };
            self.strings.push(string);
            self.names.insert(string, name);
            name
        }

        // Get the symbol as a string. `Symbol::as_str()` should be used in
        // preference to this function.
        pub(super) fn get(&self, symbol: UniqueTypeId) -> &str {
            self.strings[symbol.0 as usize]
        }
    }
}
use unique_type_id::*;

/// The `TypeMap` is where the `CrateDebugContext` holds the type metadata nodes
/// created so far. The metadata nodes are indexed by `UniqueTypeId`, and, for
/// faster lookup, also by `Ty`. The `TypeMap` is responsible for creating
/// `UniqueTypeId`s.
#[derive(Default)]
pub struct TypeMap<'ll, 'tcx> {
    /// The `UniqueTypeId`s created so far.
    unique_id_interner: TypeIdInterner,
    /// A map from `UniqueTypeId` to debuginfo metadata for that type. This is a 1:1 mapping.
    unique_id_to_metadata: FxHashMap<UniqueTypeId, &'ll DIType>,
    /// A map from types to debuginfo metadata. This is an N:1 mapping.
    type_to_metadata: FxHashMap<Ty<'tcx>, &'ll DIType>,
    /// A map from types to `UniqueTypeId`. This is an N:1 mapping.
    type_to_unique_id: FxHashMap<Ty<'tcx>, UniqueTypeId>,
}

impl<'ll, 'tcx> TypeMap<'ll, 'tcx> {
    /// Adds a Ty to metadata mapping to the TypeMap. The method will fail if
    /// the mapping already exists.
    fn register_type_with_metadata(&mut self, type_: Ty<'tcx>, metadata: &'ll DIType) {
        if self.type_to_metadata.insert(type_, metadata).is_some() {
            bug!("type metadata for `Ty` '{}' is already in the `TypeMap`!", type_);
        }
    }

    /// Removes a `Ty`-to-metadata mapping.
    /// This is useful when computing the metadata for a potentially
    /// recursive type (e.g., a function pointer of the form:
    ///
    ///     fn foo() -> impl Copy { foo }
    ///
    /// This kind of type cannot be properly represented
    /// via LLVM debuginfo. As a workaround,
    /// we register a temporary Ty to metadata mapping
    /// for the function before we compute its actual metadata.
    /// If the metadata computation ends up recursing back to the
    /// original function, it will use the temporary mapping
    /// for the inner self-reference, preventing us from
    /// recursing forever.
    ///
    /// This function is used to remove the temporary metadata
    /// mapping after we've computed the actual metadata.
    fn remove_type(&mut self, type_: Ty<'tcx>) {
        if self.type_to_metadata.remove(type_).is_none() {
            bug!("type metadata `Ty` '{}' is not in the `TypeMap`!", type_);
        }
    }

    /// Adds a `UniqueTypeId` to metadata mapping to the `TypeMap`. The method will
    /// fail if the mapping already exists.
    fn register_unique_id_with_metadata(
        &mut self,
        unique_type_id: UniqueTypeId,
        metadata: &'ll DIType,
    ) {
        if self.unique_id_to_metadata.insert(unique_type_id, metadata).is_some() {
            bug!(
                "type metadata for unique ID '{}' is already in the `TypeMap`!",
                self.get_unique_type_id_as_string(unique_type_id)
            );
        }
    }

    fn find_metadata_for_type(&self, type_: Ty<'tcx>) -> Option<&'ll DIType> {
        self.type_to_metadata.get(&type_).cloned()
    }

    fn find_metadata_for_unique_id(&self, unique_type_id: UniqueTypeId) -> Option<&'ll DIType> {
        self.unique_id_to_metadata.get(&unique_type_id).cloned()
    }

    /// Gets the string representation of a `UniqueTypeId`. This method will fail if
    /// the ID is unknown.
    fn get_unique_type_id_as_string(&self, unique_type_id: UniqueTypeId) -> &str {
        self.unique_id_interner.get(unique_type_id)
    }

    /// Gets the `UniqueTypeId` for the given type. If the `UniqueTypeId` for the given
    /// type has been requested before, this is just a table lookup. Otherwise, an
    /// ID will be generated and stored for later lookup.
    fn get_unique_type_id_of_type<'a>(
        &mut self,
        cx: &CodegenCx<'a, 'tcx>,
        type_: Ty<'tcx>,
    ) -> UniqueTypeId {
        // Let's see if we already have something in the cache.
        if let Some(unique_type_id) = self.type_to_unique_id.get(&type_).cloned() {
            return unique_type_id;
        }
        // If not, generate one.

        // The hasher we are using to generate the UniqueTypeId. We want
        // something that provides more than the 64 bits of the DefaultHasher.
        let mut hasher = StableHasher::new();
        let mut hcx = cx.tcx.create_stable_hashing_context();
        let type_ = cx.tcx.erase_regions(type_);
        hcx.while_hashing_spans(false, |hcx| {
            hcx.with_node_id_hashing_mode(NodeIdHashingMode::HashDefPath, |hcx| {
                type_.hash_stable(hcx, &mut hasher);
            });
        });
        let unique_type_id = hasher.finish::<Fingerprint>().to_hex();

        let key = self.unique_id_interner.intern(&unique_type_id);
        self.type_to_unique_id.insert(type_, key);

        key
    }

    /// Gets the `UniqueTypeId` for an enum variant. Enum variants are not really
    /// types of their own, so they need special handling. We still need a
    /// `UniqueTypeId` for them, since to debuginfo they *are* real types.
    fn get_unique_type_id_of_enum_variant<'a>(
        &mut self,
        cx: &CodegenCx<'a, 'tcx>,
        enum_type: Ty<'tcx>,
        variant_name: &str,
    ) -> UniqueTypeId {
        let enum_type_id = self.get_unique_type_id_of_type(cx, enum_type);
        let enum_variant_type_id =
            format!("{}::{}", self.get_unique_type_id_as_string(enum_type_id), variant_name);
        let interner_key = self.unique_id_interner.intern(&enum_variant_type_id);
        interner_key
    }

    /// Gets the unique type ID string for an enum variant part.
    /// Variant parts are not types and shouldn't really have their own ID,
    /// but it makes `set_members_of_composite_type()` simpler.
    fn get_unique_type_id_str_of_enum_variant_part(
        &mut self,
        enum_type_id: UniqueTypeId,
    ) -> String {
        format!("{}_variant_part", self.get_unique_type_id_as_string(enum_type_id))
    }
}

/// A description of some recursive type. It can either be already finished (as
/// with `FinalMetadata`) or it is not yet finished, but contains all information
/// needed to generate the missing parts of the description. See the
/// documentation section on Recursive Types at the top of this file for more
/// information.
enum RecursiveTypeDescription<'ll, 'tcx> {
    UnfinishedMetadata {
        unfinished_type: Ty<'tcx>,
        unique_type_id: UniqueTypeId,
        metadata_stub: &'ll DICompositeType,
        member_holding_stub: &'ll DICompositeType,
        member_description_factory: MemberDescriptionFactory<'ll, 'tcx>,
    },
    FinalMetadata(&'ll DICompositeType),
}

fn create_and_register_recursive_type_forward_declaration<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    unfinished_type: Ty<'tcx>,
    unique_type_id: UniqueTypeId,
    metadata_stub: &'ll DICompositeType,
    member_holding_stub: &'ll DICompositeType,
    member_description_factory: MemberDescriptionFactory<'ll, 'tcx>,
) -> RecursiveTypeDescription<'ll, 'tcx> {
    // Insert the stub into the `TypeMap` in order to allow for recursive references.
    let mut type_map = debug_context(cx).type_map.borrow_mut();
    type_map.register_unique_id_with_metadata(unique_type_id, metadata_stub);
    type_map.register_type_with_metadata(unfinished_type, metadata_stub);

    UnfinishedMetadata {
        unfinished_type,
        unique_type_id,
        metadata_stub,
        member_holding_stub,
        member_description_factory,
    }
}

impl<'ll, 'tcx> RecursiveTypeDescription<'ll, 'tcx> {
    /// Finishes up the description of the type in question (mostly by providing
    /// descriptions of the fields of the given type) and returns the final type
    /// metadata.
    fn finalize(&self, cx: &CodegenCx<'ll, 'tcx>) -> MetadataCreationResult<'ll> {
        match *self {
            FinalMetadata(metadata) => MetadataCreationResult::new(metadata, false),
            UnfinishedMetadata {
                unfinished_type,
                unique_type_id,
                metadata_stub,
                member_holding_stub,
                ref member_description_factory,
            } => {
                // Make sure that we have a forward declaration of the type in
                // the TypeMap so that recursive references are possible. This
                // will always be the case if the RecursiveTypeDescription has
                // been properly created through the
                // `create_and_register_recursive_type_forward_declaration()`
                // function.
                {
                    let type_map = debug_context(cx).type_map.borrow();
                    if type_map.find_metadata_for_unique_id(unique_type_id).is_none()
                        || type_map.find_metadata_for_type(unfinished_type).is_none()
                    {
                        bug!(
                            "Forward declaration of potentially recursive type \
                              '{:?}' was not found in TypeMap!",
                            unfinished_type
                        );
                    }
                }

                // ... then create the member descriptions ...
                let member_descriptions = member_description_factory.create_member_descriptions(cx);

                // ... and attach them to the stub to complete it.
                set_members_of_composite_type(
                    cx,
                    unfinished_type,
                    member_holding_stub,
                    member_descriptions,
                    None,
                );
                MetadataCreationResult::new(metadata_stub, true)
            }
        }
    }
}

/// Returns from the enclosing function if the type metadata with the given
/// unique ID can be found in the type map.
macro_rules! return_if_metadata_created_in_meantime {
    ($cx: expr, $unique_type_id: expr) => {
        if let Some(metadata) =
            debug_context($cx).type_map.borrow().find_metadata_for_unique_id($unique_type_id)
        {
            return MetadataCreationResult::new(metadata, true);
        }
    };
}

fn fixed_vec_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    unique_type_id: UniqueTypeId,
    array_or_slice_type: Ty<'tcx>,
    element_type: Ty<'tcx>,
    span: Span,
) -> MetadataCreationResult<'ll> {
    let element_type_metadata = type_metadata(cx, element_type, span);

    return_if_metadata_created_in_meantime!(cx, unique_type_id);

    let (size, align) = cx.size_and_align_of(array_or_slice_type);

    let upper_bound = match array_or_slice_type.kind() {
        ty::Array(_, len) => len.eval_usize(cx.tcx, ty::ParamEnv::reveal_all()) as c_longlong,
        _ => -1,
    };

    let subrange =
        unsafe { Some(llvm::LLVMRustDIBuilderGetOrCreateSubrange(DIB(cx), 0, upper_bound)) };

    let subscripts = create_DIArray(DIB(cx), &[subrange]);
    let metadata = unsafe {
        llvm::LLVMRustDIBuilderCreateArrayType(
            DIB(cx),
            size.bits(),
            align.bits() as u32,
            element_type_metadata,
            subscripts,
        )
    };

    MetadataCreationResult::new(metadata, false)
}

fn vec_slice_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    slice_ptr_type: Ty<'tcx>,
    element_type: Ty<'tcx>,
    unique_type_id: UniqueTypeId,
    span: Span,
) -> MetadataCreationResult<'ll> {
    let data_ptr_type = cx.tcx.mk_imm_ptr(element_type);

    let data_ptr_metadata = type_metadata(cx, data_ptr_type, span);

    return_if_metadata_created_in_meantime!(cx, unique_type_id);

    let slice_type_name = compute_debuginfo_type_name(cx.tcx, slice_ptr_type, true);

    let (pointer_size, pointer_align) = cx.size_and_align_of(data_ptr_type);
    let (usize_size, usize_align) = cx.size_and_align_of(cx.tcx.types.usize);

    let member_descriptions = vec![
        MemberDescription {
            name: "data_ptr".to_owned(),
            type_metadata: data_ptr_metadata,
            offset: Size::ZERO,
            size: pointer_size,
            align: pointer_align,
            flags: DIFlags::FlagZero,
            discriminant: None,
            source_info: None,
        },
        MemberDescription {
            name: "length".to_owned(),
            type_metadata: type_metadata(cx, cx.tcx.types.usize, span),
            offset: pointer_size,
            size: usize_size,
            align: usize_align,
            flags: DIFlags::FlagZero,
            discriminant: None,
            source_info: None,
        },
    ];

    let file_metadata = unknown_file_metadata(cx);

    let metadata = composite_type_metadata(
        cx,
        slice_ptr_type,
        &slice_type_name,
        unique_type_id,
        member_descriptions,
        NO_SCOPE_METADATA,
        file_metadata,
        span,
    );
    MetadataCreationResult::new(metadata, false)
}

fn subroutine_type_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    unique_type_id: UniqueTypeId,
    signature: ty::PolyFnSig<'tcx>,
    span: Span,
) -> MetadataCreationResult<'ll> {
    let signature =
        cx.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), signature);

    let signature_metadata: Vec<_> = iter::once(
        // return type
        match signature.output().kind() {
            ty::Tuple(tys) if tys.is_empty() => None,
            _ => Some(type_metadata(cx, signature.output(), span)),
        },
    )
    .chain(
        // regular arguments
        signature.inputs().iter().map(|argument_type| Some(type_metadata(cx, argument_type, span))),
    )
    .collect();

    return_if_metadata_created_in_meantime!(cx, unique_type_id);

    MetadataCreationResult::new(
        unsafe {
            llvm::LLVMRustDIBuilderCreateSubroutineType(
                DIB(cx),
                create_DIArray(DIB(cx), &signature_metadata[..]),
            )
        },
        false,
    )
}

// FIXME(1563): This is all a bit of a hack because 'trait pointer' is an ill-
// defined concept. For the case of an actual trait pointer (i.e., `Box<Trait>`,
// `&Trait`), `trait_object_type` should be the whole thing (e.g, `Box<Trait>`) and
// `trait_type` should be the actual trait (e.g., `Trait`). Where the trait is part
// of a DST struct, there is no `trait_object_type` and the results of this
// function will be a little bit weird.
fn trait_pointer_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    trait_type: Ty<'tcx>,
    trait_object_type: Option<Ty<'tcx>>,
    unique_type_id: UniqueTypeId,
) -> &'ll DIType {
    // The implementation provided here is a stub. It makes sure that the trait
    // type is assigned the correct name, size, namespace, and source location.
    // However, it does not describe the trait's methods.

    let (containing_scope, trait_type_name) = match trait_object_type {
        Some(trait_object_type) => match trait_object_type.kind() {
            ty::Adt(def, _) => (
                Some(get_namespace_for_item(cx, def.did)),
                compute_debuginfo_type_name(cx.tcx, trait_object_type, false),
            ),
            ty::RawPtr(_) | ty::Ref(..) => {
                (NO_SCOPE_METADATA, compute_debuginfo_type_name(cx.tcx, trait_object_type, true))
            }
            _ => {
                bug!(
                    "debuginfo: unexpected trait-object type in \
                      trait_pointer_metadata(): {:?}",
                    trait_object_type
                );
            }
        },

        // No object type, use the trait type directly (no scope here since the type
        // will be wrapped in the dyn$ synthetic type).
        None => (NO_SCOPE_METADATA, compute_debuginfo_type_name(cx.tcx, trait_type, true)),
    };

    let file_metadata = unknown_file_metadata(cx);

    let layout = cx.layout_of(cx.tcx.mk_mut_ptr(trait_type));

    assert_eq!(abi::FAT_PTR_ADDR, 0);
    assert_eq!(abi::FAT_PTR_EXTRA, 1);

    let data_ptr_field = layout.field(cx, 0);
    let vtable_field = layout.field(cx, 1);
    let member_descriptions = vec![
        MemberDescription {
            name: "pointer".to_owned(),
            type_metadata: type_metadata(
                cx,
                cx.tcx.mk_mut_ptr(cx.tcx.types.u8),
                rustc_span::DUMMY_SP,
            ),
            offset: layout.fields.offset(0),
            size: data_ptr_field.size,
            align: data_ptr_field.align.abi,
            flags: DIFlags::FlagArtificial,
            discriminant: None,
            source_info: None,
        },
        MemberDescription {
            name: "vtable".to_owned(),
            type_metadata: type_metadata(cx, vtable_field.ty, rustc_span::DUMMY_SP),
            offset: layout.fields.offset(1),
            size: vtable_field.size,
            align: vtable_field.align.abi,
            flags: DIFlags::FlagArtificial,
            discriminant: None,
            source_info: None,
        },
    ];

    composite_type_metadata(
        cx,
        trait_object_type.unwrap_or(trait_type),
        &trait_type_name,
        unique_type_id,
        member_descriptions,
        containing_scope,
        file_metadata,
        rustc_span::DUMMY_SP,
    )
}

pub fn type_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    t: Ty<'tcx>,
    usage_site_span: Span,
) -> &'ll DIType {
    // Get the unique type ID of this type.
    let unique_type_id = {
        let mut type_map = debug_context(cx).type_map.borrow_mut();
        // First, try to find the type in `TypeMap`. If we have seen it before, we
        // can exit early here.
        match type_map.find_metadata_for_type(t) {
            Some(metadata) => {
                return metadata;
            }
            None => {
                // The Ty is not in the `TypeMap` but maybe we have already seen
                // an equivalent type (e.g., only differing in region arguments).
                // In order to find out, generate the unique type ID and look
                // that up.
                let unique_type_id = type_map.get_unique_type_id_of_type(cx, t);
                match type_map.find_metadata_for_unique_id(unique_type_id) {
                    Some(metadata) => {
                        // There is already an equivalent type in the TypeMap.
                        // Register this Ty as an alias in the cache and
                        // return the cached metadata.
                        type_map.register_type_with_metadata(t, metadata);
                        return metadata;
                    }
                    None => {
                        // There really is no type metadata for this type, so
                        // proceed by creating it.
                        unique_type_id
                    }
                }
            }
        }
    };

    debug!("type_metadata: {:?}", t);

    let ptr_metadata = |ty: Ty<'tcx>| match *ty.kind() {
        ty::Slice(typ) => Ok(vec_slice_metadata(cx, t, typ, unique_type_id, usage_site_span)),
        ty::Str => Ok(vec_slice_metadata(cx, t, cx.tcx.types.u8, unique_type_id, usage_site_span)),
        ty::Dynamic(..) => Ok(MetadataCreationResult::new(
            trait_pointer_metadata(cx, ty, Some(t), unique_type_id),
            false,
        )),
        _ => {
            let pointee_metadata = type_metadata(cx, ty, usage_site_span);

            if let Some(metadata) =
                debug_context(cx).type_map.borrow().find_metadata_for_unique_id(unique_type_id)
            {
                return Err(metadata);
            }

            Ok(MetadataCreationResult::new(pointer_type_metadata(cx, t, pointee_metadata), false))
        }
    };

    let MetadataCreationResult { metadata, already_stored_in_typemap } = match *t.kind() {
        ty::Never | ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => {
            MetadataCreationResult::new(basic_type_metadata(cx, t), false)
        }
        ty::Tuple(elements) if elements.is_empty() => {
            MetadataCreationResult::new(basic_type_metadata(cx, t), false)
        }
        ty::Array(typ, _) | ty::Slice(typ) => {
            fixed_vec_metadata(cx, unique_type_id, t, typ, usage_site_span)
        }
        ty::Str => fixed_vec_metadata(cx, unique_type_id, t, cx.tcx.types.i8, usage_site_span),
        ty::Dynamic(..) => {
            MetadataCreationResult::new(trait_pointer_metadata(cx, t, None, unique_type_id), false)
        }
        ty::Foreign(..) => {
            MetadataCreationResult::new(foreign_type_metadata(cx, t, unique_type_id), false)
        }
        ty::RawPtr(ty::TypeAndMut { ty, .. }) | ty::Ref(_, ty, _) => match ptr_metadata(ty) {
            Ok(res) => res,
            Err(metadata) => return metadata,
        },
        ty::Adt(def, _) if def.is_box() => match ptr_metadata(t.boxed_ty()) {
            Ok(res) => res,
            Err(metadata) => return metadata,
        },
        ty::FnDef(..) | ty::FnPtr(_) => {
            if let Some(metadata) =
                debug_context(cx).type_map.borrow().find_metadata_for_unique_id(unique_type_id)
            {
                return metadata;
            }

            // It's possible to create a self-referential
            // type in Rust by using 'impl trait':
            //
            // fn foo() -> impl Copy { foo }
            //
            // See `TypeMap::remove_type` for more detals
            // about the workaround.

            let temp_type = {
                unsafe {
                    // The choice of type here is pretty arbitrary -
                    // anything reading the debuginfo for a recursive
                    // type is going to see *something* weird - the only
                    // question is what exactly it will see.
                    let name = "<recur_type>";
                    llvm::LLVMRustDIBuilderCreateBasicType(
                        DIB(cx),
                        name.as_ptr().cast(),
                        name.len(),
                        cx.size_of(t).bits(),
                        DW_ATE_unsigned,
                    )
                }
            };

            let type_map = &debug_context(cx).type_map;
            type_map.borrow_mut().register_type_with_metadata(t, temp_type);

            let fn_metadata =
                subroutine_type_metadata(cx, unique_type_id, t.fn_sig(cx.tcx), usage_site_span)
                    .metadata;

            type_map.borrow_mut().remove_type(t);

            // This is actually a function pointer, so wrap it in pointer DI.
            MetadataCreationResult::new(pointer_type_metadata(cx, t, fn_metadata), false)
        }
        ty::Closure(def_id, substs) => {
            let upvar_tys: Vec<_> = substs.as_closure().upvar_tys().collect();
            let containing_scope = get_namespace_for_item(cx, def_id);
            prepare_tuple_metadata(
                cx,
                t,
                &upvar_tys,
                unique_type_id,
                usage_site_span,
                Some(containing_scope),
            )
            .finalize(cx)
        }
        ty::Generator(def_id, substs, _) => {
            let upvar_tys: Vec<_> = substs
                .as_generator()
                .prefix_tys()
                .map(|t| cx.tcx.normalize_erasing_regions(ParamEnv::reveal_all(), t))
                .collect();
            prepare_enum_metadata(cx, t, def_id, unique_type_id, usage_site_span, upvar_tys)
                .finalize(cx)
        }
        ty::Adt(def, ..) => match def.adt_kind() {
            AdtKind::Struct => {
                prepare_struct_metadata(cx, t, unique_type_id, usage_site_span).finalize(cx)
            }
            AdtKind::Union => {
                prepare_union_metadata(cx, t, unique_type_id, usage_site_span).finalize(cx)
            }
            AdtKind::Enum => {
                prepare_enum_metadata(cx, t, def.did, unique_type_id, usage_site_span, vec![])
                    .finalize(cx)
            }
        },
        ty::Tuple(elements) => {
            let tys: Vec<_> = elements.iter().map(|k| k.expect_ty()).collect();
            prepare_tuple_metadata(cx, t, &tys, unique_type_id, usage_site_span, NO_SCOPE_METADATA)
                .finalize(cx)
        }
        // Type parameters from polymorphized functions.
        ty::Param(_) => MetadataCreationResult::new(param_type_metadata(cx, t), false),
        _ => bug!("debuginfo: unexpected type in type_metadata: {:?}", t),
    };

    {
        let mut type_map = debug_context(cx).type_map.borrow_mut();

        if already_stored_in_typemap {
            // Also make sure that we already have a `TypeMap` entry for the unique type ID.
            let metadata_for_uid = match type_map.find_metadata_for_unique_id(unique_type_id) {
                Some(metadata) => metadata,
                None => {
                    span_bug!(
                        usage_site_span,
                        "expected type metadata for unique \
                               type ID '{}' to already be in \
                               the `debuginfo::TypeMap` but it \
                               was not. (Ty = {})",
                        type_map.get_unique_type_id_as_string(unique_type_id),
                        t
                    );
                }
            };

            match type_map.find_metadata_for_type(t) {
                Some(metadata) => {
                    if metadata != metadata_for_uid {
                        span_bug!(
                            usage_site_span,
                            "mismatch between `Ty` and \
                                   `UniqueTypeId` maps in \
                                   `debuginfo::TypeMap`. \
                                   UniqueTypeId={}, Ty={}",
                            type_map.get_unique_type_id_as_string(unique_type_id),
                            t
                        );
                    }
                }
                None => {
                    type_map.register_type_with_metadata(t, metadata);
                }
            }
        } else {
            type_map.register_type_with_metadata(t, metadata);
            type_map.register_unique_id_with_metadata(unique_type_id, metadata);
        }
    }

    metadata
}

fn hex_encode(data: &[u8]) -> String {
    let mut hex_string = String::with_capacity(data.len() * 2);
    for byte in data.iter() {
        write!(&mut hex_string, "{:02x}", byte).unwrap();
    }
    hex_string
}

pub fn file_metadata<'ll>(cx: &CodegenCx<'ll, '_>, source_file: &SourceFile) -> &'ll DIFile {
    debug!("file_metadata: file_name: {:?}", source_file.name);

    let hash = Some(&source_file.src_hash);
    let file_name = Some(source_file.name.prefer_remapped().to_string());
    let directory = if source_file.is_real_file() && !source_file.is_imported() {
        Some(
            cx.sess()
                .opts
                .working_dir
                .to_string_lossy(FileNameDisplayPreference::Remapped)
                .to_string(),
        )
    } else {
        // If the path comes from an upstream crate we assume it has been made
        // independent of the compiler's working directory one way or another.
        None
    };
    file_metadata_raw(cx, file_name, directory, hash)
}

pub fn unknown_file_metadata<'ll>(cx: &CodegenCx<'ll, '_>) -> &'ll DIFile {
    file_metadata_raw(cx, None, None, None)
}

fn file_metadata_raw<'ll>(
    cx: &CodegenCx<'ll, '_>,
    file_name: Option<String>,
    directory: Option<String>,
    hash: Option<&SourceFileHash>,
) -> &'ll DIFile {
    let key = (file_name, directory);

    match debug_context(cx).created_files.borrow_mut().entry(key) {
        Entry::Occupied(o) => o.get(),
        Entry::Vacant(v) => {
            let (file_name, directory) = v.key();
            debug!("file_metadata: file_name: {:?}, directory: {:?}", file_name, directory);

            let file_name = file_name.as_deref().unwrap_or("<unknown>");
            let directory = directory.as_deref().unwrap_or("");

            let (hash_kind, hash_value) = match hash {
                Some(hash) => {
                    let kind = match hash.kind {
                        rustc_span::SourceFileHashAlgorithm::Md5 => llvm::ChecksumKind::MD5,
                        rustc_span::SourceFileHashAlgorithm::Sha1 => llvm::ChecksumKind::SHA1,
                        rustc_span::SourceFileHashAlgorithm::Sha256 => llvm::ChecksumKind::SHA256,
                    };
                    (kind, hex_encode(hash.hash_bytes()))
                }
                None => (llvm::ChecksumKind::None, String::new()),
            };

            let file_metadata = unsafe {
                llvm::LLVMRustDIBuilderCreateFile(
                    DIB(cx),
                    file_name.as_ptr().cast(),
                    file_name.len(),
                    directory.as_ptr().cast(),
                    directory.len(),
                    hash_kind,
                    hash_value.as_ptr().cast(),
                    hash_value.len(),
                )
            };

            v.insert(file_metadata);
            file_metadata
        }
    }
}

trait MsvcBasicName {
    fn msvc_basic_name(self) -> &'static str;
}

impl MsvcBasicName for ty::IntTy {
    fn msvc_basic_name(self) -> &'static str {
        match self {
            ty::IntTy::Isize => "ptrdiff_t",
            ty::IntTy::I8 => "__int8",
            ty::IntTy::I16 => "__int16",
            ty::IntTy::I32 => "__int32",
            ty::IntTy::I64 => "__int64",
            ty::IntTy::I128 => "__int128",
        }
    }
}

impl MsvcBasicName for ty::UintTy {
    fn msvc_basic_name(self) -> &'static str {
        match self {
            ty::UintTy::Usize => "size_t",
            ty::UintTy::U8 => "unsigned __int8",
            ty::UintTy::U16 => "unsigned __int16",
            ty::UintTy::U32 => "unsigned __int32",
            ty::UintTy::U64 => "unsigned __int64",
            ty::UintTy::U128 => "unsigned __int128",
        }
    }
}

impl MsvcBasicName for ty::FloatTy {
    fn msvc_basic_name(self) -> &'static str {
        match self {
            ty::FloatTy::F32 => "float",
            ty::FloatTy::F64 => "double",
        }
    }
}

fn basic_type_metadata<'ll, 'tcx>(cx: &CodegenCx<'ll, 'tcx>, t: Ty<'tcx>) -> &'ll DIType {
    debug!("basic_type_metadata: {:?}", t);

    // When targeting MSVC, emit MSVC style type names for compatibility with
    // .natvis visualizers (and perhaps other existing native debuggers?)
    let cpp_like_debuginfo = cpp_like_debuginfo(cx.tcx);

    let (name, encoding) = match t.kind() {
        ty::Never => ("!", DW_ATE_unsigned),
        ty::Tuple(elements) if elements.is_empty() => ("()", DW_ATE_unsigned),
        ty::Bool => ("bool", DW_ATE_boolean),
        ty::Char => ("char", DW_ATE_unsigned_char),
        ty::Int(int_ty) if cpp_like_debuginfo => (int_ty.msvc_basic_name(), DW_ATE_signed),
        ty::Uint(uint_ty) if cpp_like_debuginfo => (uint_ty.msvc_basic_name(), DW_ATE_unsigned),
        ty::Float(float_ty) if cpp_like_debuginfo => (float_ty.msvc_basic_name(), DW_ATE_float),
        ty::Int(int_ty) => (int_ty.name_str(), DW_ATE_signed),
        ty::Uint(uint_ty) => (uint_ty.name_str(), DW_ATE_unsigned),
        ty::Float(float_ty) => (float_ty.name_str(), DW_ATE_float),
        _ => bug!("debuginfo::basic_type_metadata - `t` is invalid type"),
    };

    let ty_metadata = unsafe {
        llvm::LLVMRustDIBuilderCreateBasicType(
            DIB(cx),
            name.as_ptr().cast(),
            name.len(),
            cx.size_of(t).bits(),
            encoding,
        )
    };

    if !cpp_like_debuginfo {
        return ty_metadata;
    }

    let typedef_name = match t.kind() {
        ty::Int(int_ty) => int_ty.name_str(),
        ty::Uint(uint_ty) => uint_ty.name_str(),
        ty::Float(float_ty) => float_ty.name_str(),
        _ => return ty_metadata,
    };

    let typedef_metadata = unsafe {
        llvm::LLVMRustDIBuilderCreateTypedef(
            DIB(cx),
            ty_metadata,
            typedef_name.as_ptr().cast(),
            typedef_name.len(),
            unknown_file_metadata(cx),
            0,
            None,
        )
    };

    typedef_metadata
}

fn foreign_type_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    t: Ty<'tcx>,
    unique_type_id: UniqueTypeId,
) -> &'ll DIType {
    debug!("foreign_type_metadata: {:?}", t);

    let name = compute_debuginfo_type_name(cx.tcx, t, false);
    create_struct_stub(cx, t, &name, unique_type_id, NO_SCOPE_METADATA, DIFlags::FlagZero)
}

fn pointer_type_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    pointer_type: Ty<'tcx>,
    pointee_type_metadata: &'ll DIType,
) -> &'ll DIType {
    let (pointer_size, pointer_align) = cx.size_and_align_of(pointer_type);
    let name = compute_debuginfo_type_name(cx.tcx, pointer_type, false);
    unsafe {
        llvm::LLVMRustDIBuilderCreatePointerType(
            DIB(cx),
            pointee_type_metadata,
            pointer_size.bits(),
            pointer_align.bits() as u32,
            0, // Ignore DWARF address space.
            name.as_ptr().cast(),
            name.len(),
        )
    }
}

fn param_type_metadata<'ll, 'tcx>(cx: &CodegenCx<'ll, 'tcx>, t: Ty<'tcx>) -> &'ll DIType {
    debug!("param_type_metadata: {:?}", t);
    let name = format!("{:?}", t);
    unsafe {
        llvm::LLVMRustDIBuilderCreateBasicType(
            DIB(cx),
            name.as_ptr().cast(),
            name.len(),
            Size::ZERO.bits(),
            DW_ATE_unsigned,
        )
    }
}

pub fn compile_unit_metadata<'ll, 'tcx>(
    tcx: TyCtxt<'tcx>,
    codegen_unit_name: &str,
    debug_context: &CrateDebugContext<'ll, 'tcx>,
) -> &'ll DIDescriptor {
    let mut name_in_debuginfo = match tcx.sess.local_crate_source_file {
        Some(ref path) => path.clone(),
        None => PathBuf::from(tcx.crate_name(LOCAL_CRATE).as_str()),
    };

    // To avoid breaking split DWARF, we need to ensure that each codegen unit
    // has a unique `DW_AT_name`. This is because there's a remote chance that
    // different codegen units for the same module will have entirely
    // identical DWARF entries for the purpose of the DWO ID, which would
    // violate Appendix F ("Split Dwarf Object Files") of the DWARF 5
    // specification. LLVM uses the algorithm specified in section 7.32 "Type
    // Signature Computation" to compute the DWO ID, which does not include
    // any fields that would distinguish compilation units. So we must embed
    // the codegen unit name into the `DW_AT_name`. (Issue #88521.)
    //
    // Additionally, the OSX linker has an idiosyncrasy where it will ignore
    // some debuginfo if multiple object files with the same `DW_AT_name` are
    // linked together.
    //
    // As a workaround for these two issues, we generate unique names for each
    // object file. Those do not correspond to an actual source file but that
    // is harmless.
    name_in_debuginfo.push("@");
    name_in_debuginfo.push(codegen_unit_name);

    debug!("compile_unit_metadata: {:?}", name_in_debuginfo);
    let rustc_producer =
        format!("rustc version {}", option_env!("CFG_VERSION").expect("CFG_VERSION"),);
    // FIXME(#41252) Remove "clang LLVM" if we can get GDB and LLVM to play nice.
    let producer = format!("clang LLVM ({})", rustc_producer);

    let name_in_debuginfo = name_in_debuginfo.to_string_lossy();
    let work_dir = tcx.sess.opts.working_dir.to_string_lossy(FileNameDisplayPreference::Remapped);
    let flags = "\0";
    let output_filenames = tcx.output_filenames(());
    let split_name = if tcx.sess.target_can_use_split_dwarf() {
        output_filenames
            .split_dwarf_path(
                tcx.sess.split_debuginfo(),
                tcx.sess.opts.debugging_opts.split_dwarf_kind,
                Some(codegen_unit_name),
            )
            // We get a path relative to the working directory from split_dwarf_path
            .map(|f| tcx.sess.source_map().path_mapping().map_prefix(f).0)
    } else {
        None
    }
    .unwrap_or_default();
    let split_name = split_name.to_str().unwrap();

    // FIXME(#60020):
    //
    //    This should actually be
    //
    //        let kind = DebugEmissionKind::from_generic(tcx.sess.opts.debuginfo);
    //
    //    That is, we should set LLVM's emission kind to `LineTablesOnly` if
    //    we are compiling with "limited" debuginfo. However, some of the
    //    existing tools relied on slightly more debuginfo being generated than
    //    would be the case with `LineTablesOnly`, and we did not want to break
    //    these tools in a "drive-by fix", without a good idea or plan about
    //    what limited debuginfo should exactly look like. So for now we keep
    //    the emission kind as `FullDebug`.
    //
    //    See https://github.com/rust-lang/rust/issues/60020 for details.
    let kind = DebugEmissionKind::FullDebug;
    assert!(tcx.sess.opts.debuginfo != DebugInfo::None);

    unsafe {
        let compile_unit_file = llvm::LLVMRustDIBuilderCreateFile(
            debug_context.builder,
            name_in_debuginfo.as_ptr().cast(),
            name_in_debuginfo.len(),
            work_dir.as_ptr().cast(),
            work_dir.len(),
            llvm::ChecksumKind::None,
            ptr::null(),
            0,
        );

        let unit_metadata = llvm::LLVMRustDIBuilderCreateCompileUnit(
            debug_context.builder,
            DW_LANG_RUST,
            compile_unit_file,
            producer.as_ptr().cast(),
            producer.len(),
            tcx.sess.opts.optimize != config::OptLevel::No,
            flags.as_ptr().cast(),
            0,
            // NB: this doesn't actually have any perceptible effect, it seems. LLVM will instead
            // put the path supplied to `MCSplitDwarfFile` into the debug info of the final
            // output(s).
            split_name.as_ptr().cast(),
            split_name.len(),
            kind,
            0,
            tcx.sess.opts.debugging_opts.split_dwarf_inlining,
        );

        if tcx.sess.opts.debugging_opts.profile {
            let cu_desc_metadata =
                llvm::LLVMRustMetadataAsValue(debug_context.llcontext, unit_metadata);
            let default_gcda_path = &output_filenames.with_extension("gcda");
            let gcda_path =
                tcx.sess.opts.debugging_opts.profile_emit.as_ref().unwrap_or(default_gcda_path);

            let gcov_cu_info = [
                path_to_mdstring(debug_context.llcontext, &output_filenames.with_extension("gcno")),
                path_to_mdstring(debug_context.llcontext, gcda_path),
                cu_desc_metadata,
            ];
            let gcov_metadata = llvm::LLVMMDNodeInContext(
                debug_context.llcontext,
                gcov_cu_info.as_ptr(),
                gcov_cu_info.len() as c_uint,
            );

            let llvm_gcov_ident = cstr!("llvm.gcov");
            llvm::LLVMAddNamedMetadataOperand(
                debug_context.llmod,
                llvm_gcov_ident.as_ptr(),
                gcov_metadata,
            );
        }

        // Insert `llvm.ident` metadata on the wasm targets since that will
        // get hooked up to the "producer" sections `processed-by` information.
        if tcx.sess.target.is_like_wasm {
            let name_metadata = llvm::LLVMMDStringInContext(
                debug_context.llcontext,
                rustc_producer.as_ptr().cast(),
                rustc_producer.as_bytes().len() as c_uint,
            );
            llvm::LLVMAddNamedMetadataOperand(
                debug_context.llmod,
                cstr!("llvm.ident").as_ptr(),
                llvm::LLVMMDNodeInContext(debug_context.llcontext, &name_metadata, 1),
            );
        }

        return unit_metadata;
    };

    fn path_to_mdstring<'ll>(llcx: &'ll llvm::Context, path: &Path) -> &'ll Value {
        let path_str = path_to_c_string(path);
        unsafe {
            llvm::LLVMMDStringInContext(
                llcx,
                path_str.as_ptr(),
                path_str.as_bytes().len() as c_uint,
            )
        }
    }
}

struct MetadataCreationResult<'ll> {
    metadata: &'ll DIType,
    already_stored_in_typemap: bool,
}

impl<'ll> MetadataCreationResult<'ll> {
    fn new(metadata: &'ll DIType, already_stored_in_typemap: bool) -> Self {
        MetadataCreationResult { metadata, already_stored_in_typemap }
    }
}

#[derive(Debug)]
struct SourceInfo<'ll> {
    file: &'ll DIFile,
    line: u32,
}

/// Description of a type member, which can either be a regular field (as in
/// structs or tuples) or an enum variant.
#[derive(Debug)]
struct MemberDescription<'ll> {
    name: String,
    type_metadata: &'ll DIType,
    offset: Size,
    size: Size,
    align: Align,
    flags: DIFlags,
    discriminant: Option<u64>,
    source_info: Option<SourceInfo<'ll>>,
}

impl<'ll> MemberDescription<'ll> {
    fn into_metadata(
        self,
        cx: &CodegenCx<'ll, '_>,
        composite_type_metadata: &'ll DIScope,
    ) -> &'ll DIType {
        let (file, line) = self
            .source_info
            .map(|info| (info.file, info.line))
            .unwrap_or_else(|| (unknown_file_metadata(cx), UNKNOWN_LINE_NUMBER));
        unsafe {
            llvm::LLVMRustDIBuilderCreateVariantMemberType(
                DIB(cx),
                composite_type_metadata,
                self.name.as_ptr().cast(),
                self.name.len(),
                file,
                line,
                self.size.bits(),
                self.align.bits() as u32,
                self.offset.bits(),
                self.discriminant.map(|v| cx.const_u64(v)),
                self.flags,
                self.type_metadata,
            )
        }
    }
}

/// A factory for `MemberDescription`s. It produces a list of member descriptions
/// for some record-like type. `MemberDescriptionFactory`s are used to defer the
/// creation of type member descriptions in order to break cycles arising from
/// recursive type definitions.
enum MemberDescriptionFactory<'ll, 'tcx> {
    StructMDF(StructMemberDescriptionFactory<'tcx>),
    TupleMDF(TupleMemberDescriptionFactory<'tcx>),
    EnumMDF(EnumMemberDescriptionFactory<'ll, 'tcx>),
    UnionMDF(UnionMemberDescriptionFactory<'tcx>),
    VariantMDF(VariantMemberDescriptionFactory<'tcx>),
}

impl<'ll, 'tcx> MemberDescriptionFactory<'ll, 'tcx> {
    fn create_member_descriptions(&self, cx: &CodegenCx<'ll, 'tcx>) -> Vec<MemberDescription<'ll>> {
        match *self {
            StructMDF(ref this) => this.create_member_descriptions(cx),
            TupleMDF(ref this) => this.create_member_descriptions(cx),
            EnumMDF(ref this) => this.create_member_descriptions(cx),
            UnionMDF(ref this) => this.create_member_descriptions(cx),
            VariantMDF(ref this) => this.create_member_descriptions(cx),
        }
    }
}

//=-----------------------------------------------------------------------------
// Structs
//=-----------------------------------------------------------------------------

/// Creates `MemberDescription`s for the fields of a struct.
struct StructMemberDescriptionFactory<'tcx> {
    ty: Ty<'tcx>,
    variant: &'tcx ty::VariantDef,
    span: Span,
}

impl<'tcx> StructMemberDescriptionFactory<'tcx> {
    fn create_member_descriptions<'ll>(
        &self,
        cx: &CodegenCx<'ll, 'tcx>,
    ) -> Vec<MemberDescription<'ll>> {
        let layout = cx.layout_of(self.ty);
        self.variant
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let name = if self.variant.ctor_kind == CtorKind::Fn {
                    format!("__{}", i)
                } else {
                    f.name.to_string()
                };
                let field = layout.field(cx, i);
                MemberDescription {
                    name,
                    type_metadata: type_metadata(cx, field.ty, self.span),
                    offset: layout.fields.offset(i),
                    size: field.size,
                    align: field.align.abi,
                    flags: DIFlags::FlagZero,
                    discriminant: None,
                    source_info: None,
                }
            })
            .collect()
    }
}

fn prepare_struct_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    struct_type: Ty<'tcx>,
    unique_type_id: UniqueTypeId,
    span: Span,
) -> RecursiveTypeDescription<'ll, 'tcx> {
    let struct_name = compute_debuginfo_type_name(cx.tcx, struct_type, false);

    let (struct_def_id, variant) = match struct_type.kind() {
        ty::Adt(def, _) => (def.did, def.non_enum_variant()),
        _ => bug!("prepare_struct_metadata on a non-ADT"),
    };

    let containing_scope = get_namespace_for_item(cx, struct_def_id);

    let struct_metadata_stub = create_struct_stub(
        cx,
        struct_type,
        &struct_name,
        unique_type_id,
        Some(containing_scope),
        DIFlags::FlagZero,
    );

    create_and_register_recursive_type_forward_declaration(
        cx,
        struct_type,
        unique_type_id,
        struct_metadata_stub,
        struct_metadata_stub,
        StructMDF(StructMemberDescriptionFactory { ty: struct_type, variant, span }),
    )
}

//=-----------------------------------------------------------------------------
// Tuples
//=-----------------------------------------------------------------------------

/// Returns names of captured upvars for closures and generators.
///
/// Here are some examples:
///  - `name__field1__field2` when the upvar is captured by value.
///  - `_ref__name__field` when the upvar is captured by reference.
fn closure_saved_names_of_captured_variables(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<String> {
    let body = tcx.optimized_mir(def_id);

    body.var_debug_info
        .iter()
        .filter_map(|var| {
            let is_ref = match var.value {
                mir::VarDebugInfoContents::Place(place) if place.local == mir::Local::new(1) => {
                    // The projection is either `[.., Field, Deref]` or `[.., Field]`. It
                    // implies whether the variable is captured by value or by reference.
                    matches!(place.projection.last().unwrap(), mir::ProjectionElem::Deref)
                }
                _ => return None,
            };
            let prefix = if is_ref { "_ref__" } else { "" };
            Some(prefix.to_owned() + var.name.as_str())
        })
        .collect::<Vec<_>>()
}

/// Creates `MemberDescription`s for the fields of a tuple.
struct TupleMemberDescriptionFactory<'tcx> {
    ty: Ty<'tcx>,
    component_types: Vec<Ty<'tcx>>,
    span: Span,
}

impl<'tcx> TupleMemberDescriptionFactory<'tcx> {
    fn create_member_descriptions<'ll>(
        &self,
        cx: &CodegenCx<'ll, 'tcx>,
    ) -> Vec<MemberDescription<'ll>> {
        let mut capture_names = match *self.ty.kind() {
            ty::Generator(def_id, ..) | ty::Closure(def_id, ..) => {
                Some(closure_saved_names_of_captured_variables(cx.tcx, def_id).into_iter())
            }
            _ => None,
        };
        let layout = cx.layout_of(self.ty);
        self.component_types
            .iter()
            .enumerate()
            .map(|(i, &component_type)| {
                let (size, align) = cx.size_and_align_of(component_type);
                let name = if let Some(names) = capture_names.as_mut() {
                    names.next().unwrap()
                } else {
                    format!("__{}", i)
                };
                MemberDescription {
                    name,
                    type_metadata: type_metadata(cx, component_type, self.span),
                    offset: layout.fields.offset(i),
                    size,
                    align,
                    flags: DIFlags::FlagZero,
                    discriminant: None,
                    source_info: None,
                }
            })
            .collect()
    }
}

fn prepare_tuple_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    tuple_type: Ty<'tcx>,
    component_types: &[Ty<'tcx>],
    unique_type_id: UniqueTypeId,
    span: Span,
    containing_scope: Option<&'ll DIScope>,
) -> RecursiveTypeDescription<'ll, 'tcx> {
    let tuple_name = compute_debuginfo_type_name(cx.tcx, tuple_type, false);

    let struct_stub = create_struct_stub(
        cx,
        tuple_type,
        &tuple_name[..],
        unique_type_id,
        containing_scope,
        DIFlags::FlagZero,
    );

    create_and_register_recursive_type_forward_declaration(
        cx,
        tuple_type,
        unique_type_id,
        struct_stub,
        struct_stub,
        TupleMDF(TupleMemberDescriptionFactory {
            ty: tuple_type,
            component_types: component_types.to_vec(),
            span,
        }),
    )
}

//=-----------------------------------------------------------------------------
// Unions
//=-----------------------------------------------------------------------------

struct UnionMemberDescriptionFactory<'tcx> {
    layout: TyAndLayout<'tcx>,
    variant: &'tcx ty::VariantDef,
    span: Span,
}

impl<'tcx> UnionMemberDescriptionFactory<'tcx> {
    fn create_member_descriptions<'ll>(
        &self,
        cx: &CodegenCx<'ll, 'tcx>,
    ) -> Vec<MemberDescription<'ll>> {
        self.variant
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let field = self.layout.field(cx, i);
                MemberDescription {
                    name: f.name.to_string(),
                    type_metadata: type_metadata(cx, field.ty, self.span),
                    offset: Size::ZERO,
                    size: field.size,
                    align: field.align.abi,
                    flags: DIFlags::FlagZero,
                    discriminant: None,
                    source_info: None,
                }
            })
            .collect()
    }
}

fn prepare_union_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    union_type: Ty<'tcx>,
    unique_type_id: UniqueTypeId,
    span: Span,
) -> RecursiveTypeDescription<'ll, 'tcx> {
    let union_name = compute_debuginfo_type_name(cx.tcx, union_type, false);

    let (union_def_id, variant) = match union_type.kind() {
        ty::Adt(def, _) => (def.did, def.non_enum_variant()),
        _ => bug!("prepare_union_metadata on a non-ADT"),
    };

    let containing_scope = get_namespace_for_item(cx, union_def_id);

    let union_metadata_stub =
        create_union_stub(cx, union_type, &union_name, unique_type_id, containing_scope);

    create_and_register_recursive_type_forward_declaration(
        cx,
        union_type,
        unique_type_id,
        union_metadata_stub,
        union_metadata_stub,
        UnionMDF(UnionMemberDescriptionFactory { layout: cx.layout_of(union_type), variant, span }),
    )
}

//=-----------------------------------------------------------------------------
// Enums
//=-----------------------------------------------------------------------------

// FIXME(eddyb) maybe precompute this? Right now it's computed once
// per generator monomorphization, but it doesn't depend on substs.
fn generator_layout_and_saved_local_names<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
) -> (&'tcx GeneratorLayout<'tcx>, IndexVec<mir::GeneratorSavedLocal, Option<Symbol>>) {
    let body = tcx.optimized_mir(def_id);
    let generator_layout = body.generator_layout().unwrap();
    let mut generator_saved_local_names = IndexVec::from_elem(None, &generator_layout.field_tys);

    let state_arg = mir::Local::new(1);
    for var in &body.var_debug_info {
        let place = if let mir::VarDebugInfoContents::Place(p) = var.value { p } else { continue };
        if place.local != state_arg {
            continue;
        }
        match place.projection[..] {
            [
                // Deref of the `Pin<&mut Self>` state argument.
                mir::ProjectionElem::Field(..),
                mir::ProjectionElem::Deref,
                // Field of a variant of the state.
                mir::ProjectionElem::Downcast(_, variant),
                mir::ProjectionElem::Field(field, _),
            ] => {
                let name = &mut generator_saved_local_names
                    [generator_layout.variant_fields[variant][field]];
                if name.is_none() {
                    name.replace(var.name);
                }
            }
            _ => {}
        }
    }
    (generator_layout, generator_saved_local_names)
}

/// Describes the members of an enum value; an enum is described as a union of
/// structs in DWARF. This `MemberDescriptionFactory` provides the description for
/// the members of this union; so for every variant of the given enum, this
/// factory will produce one `MemberDescription` (all with no name and a fixed
/// offset of zero bytes).
struct EnumMemberDescriptionFactory<'ll, 'tcx> {
    enum_type: Ty<'tcx>,
    layout: TyAndLayout<'tcx>,
    tag_type_metadata: Option<&'ll DIType>,
    common_members: Vec<Option<&'ll DIType>>,
    span: Span,
}

impl<'ll, 'tcx> EnumMemberDescriptionFactory<'ll, 'tcx> {
    fn create_member_descriptions(&self, cx: &CodegenCx<'ll, 'tcx>) -> Vec<MemberDescription<'ll>> {
        let generator_variant_info_data = match *self.enum_type.kind() {
            ty::Generator(def_id, ..) => {
                Some(generator_layout_and_saved_local_names(cx.tcx, def_id))
            }
            _ => None,
        };

        let variant_info_for = |index: VariantIdx| match *self.enum_type.kind() {
            ty::Adt(adt, _) => VariantInfo::Adt(&adt.variants[index]),
            ty::Generator(def_id, _, _) => {
                let (generator_layout, generator_saved_local_names) =
                    generator_variant_info_data.as_ref().unwrap();
                VariantInfo::Generator {
                    def_id,
                    generator_layout: *generator_layout,
                    generator_saved_local_names,
                    variant_index: index,
                }
            }
            _ => bug!(),
        };

        // While LLVM supports generating debuginfo for variant types (enums), it doesn't support
        // lowering that debuginfo to CodeView records for msvc targets. So if we are targeting
        // msvc, then we need to use a different, fallback encoding of the debuginfo.
        let fallback = cpp_like_debuginfo(cx.tcx);
        // This will always find the metadata in the type map.
        let self_metadata = type_metadata(cx, self.enum_type, self.span);

        match self.layout.variants {
            Variants::Single { index } => {
                if let ty::Adt(adt, _) = self.enum_type.kind() {
                    if adt.variants.is_empty() {
                        return vec![];
                    }
                }

                let variant_info = variant_info_for(index);
                let (variant_type_metadata, member_description_factory) =
                    describe_enum_variant(cx, self.layout, variant_info, self_metadata, self.span);

                let member_descriptions = member_description_factory.create_member_descriptions(cx);

                set_members_of_composite_type(
                    cx,
                    self.enum_type,
                    variant_type_metadata,
                    member_descriptions,
                    Some(&self.common_members),
                );
                vec![MemberDescription {
                    name: variant_info.variant_name(),
                    type_metadata: variant_type_metadata,
                    offset: Size::ZERO,
                    size: self.layout.size,
                    align: self.layout.align.abi,
                    flags: DIFlags::FlagZero,
                    discriminant: None,
                    source_info: variant_info.source_info(cx),
                }]
            }
            Variants::Multiple {
                tag_encoding: TagEncoding::Direct,
                tag_field,
                ref variants,
                ..
            } => {
                let fallback_discr_variant = if fallback {
                    // For MSVC, we generate a union of structs for each variant and an
                    // explicit discriminant field roughly equivalent to the following C:
                    // ```c
                    // union enum$<{name}> {
                    //   struct {variant 0 name} {
                    //     <variant 0 fields>
                    //   } variant0;
                    //   <other variant structs>
                    //   {name} discriminant;
                    // }
                    // ```
                    // The natvis in `intrinsic.natvis` then matches on `this.discriminant` to
                    // determine which variant is active and then displays it.
                    let enum_layout = self.layout;
                    let offset = enum_layout.fields.offset(tag_field);
                    let discr_ty = enum_layout.field(cx, tag_field).ty;
                    let (size, align) = cx.size_and_align_of(discr_ty);
                    Some(MemberDescription {
                        name: "discriminant".into(),
                        type_metadata: self.tag_type_metadata.unwrap(),
                        offset,
                        size,
                        align,
                        flags: DIFlags::FlagZero,
                        discriminant: None,
                        source_info: None,
                    })
                } else {
                    None
                };

                variants
                    .iter_enumerated()
                    .map(|(i, _)| {
                        let variant = self.layout.for_variant(cx, i);
                        let variant_info = variant_info_for(i);
                        let (variant_type_metadata, member_desc_factory) = describe_enum_variant(
                            cx,
                            variant,
                            variant_info,
                            self_metadata,
                            self.span,
                        );

                        let member_descriptions =
                            member_desc_factory.create_member_descriptions(cx);

                        set_members_of_composite_type(
                            cx,
                            self.enum_type,
                            variant_type_metadata,
                            member_descriptions,
                            Some(&self.common_members),
                        );

                        MemberDescription {
                            name: if fallback {
                                format!("variant{}", i.as_u32())
                            } else {
                                variant_info.variant_name()
                            },
                            type_metadata: variant_type_metadata,
                            offset: Size::ZERO,
                            size: self.layout.size,
                            align: self.layout.align.abi,
                            flags: DIFlags::FlagZero,
                            discriminant: Some(
                                self.layout.ty.discriminant_for_variant(cx.tcx, i).unwrap().val
                                    as u64,
                            ),
                            source_info: variant_info.source_info(cx),
                        }
                    })
                    .chain(fallback_discr_variant.into_iter())
                    .collect()
            }
            Variants::Multiple {
                tag_encoding:
                    TagEncoding::Niche { ref niche_variants, niche_start, dataful_variant },
                tag,
                ref variants,
                tag_field,
            } => {
                let calculate_niche_value = |i: VariantIdx| {
                    if i == dataful_variant {
                        None
                    } else {
                        let value = (i.as_u32() as u128)
                            .wrapping_sub(niche_variants.start().as_u32() as u128)
                            .wrapping_add(niche_start);
                        let value = tag.value.size(cx).truncate(value);
                        // NOTE(eddyb) do *NOT* remove this assert, until
                        // we pass the full 128-bit value to LLVM, otherwise
                        // truncation will be silent and remain undetected.
                        assert_eq!(value as u64 as u128, value);
                        Some(value as u64)
                    }
                };

                // For MSVC, we will generate a union of two fields, one for the dataful variant
                // and one that just points to the discriminant. We also create an enum that
                // contains tag values for the non-dataful variants and make the discriminant field
                // that type. We then use natvis to render the enum type correctly in Windbg/VS.
                // This will generate debuginfo roughly equivalent to the following C:
                // ```c
                // union enum$<{name}, {min niche}, {max niche}, {dataful variant name}> {
                //   struct <dataful variant name> {
                //     <fields in dataful variant>
                //   } dataful_variant;
                //   enum Discriminant$ {
                //     <non-dataful variants>
                //   } discriminant;
                // }
                // ```
                // The natvis in `intrinsic.natvis` matches on the type name `enum$<*, *, *, *>`
                // and evaluates `this.discriminant`. If the value is between the min niche and max
                // niche, then the enum is in the dataful variant and `this.dataful_variant` is
                // rendered. Otherwise, the enum is in one of the non-dataful variants. In that
                // case, we just need to render the name of the `this.discriminant` enum.
                if fallback {
                    let dataful_variant_layout = self.layout.for_variant(cx, dataful_variant);

                    let mut discr_enum_ty = tag.value.to_ty(cx.tcx);
                    // If the niche is the NULL value of a reference, then `discr_enum_ty` will be a RawPtr.
                    // CodeView doesn't know what to do with enums whose base type is a pointer so we fix this up
                    // to just be `usize`.
                    if let ty::RawPtr(_) = discr_enum_ty.kind() {
                        discr_enum_ty = cx.tcx.types.usize;
                    }

                    let tags: Vec<_> = variants
                        .iter_enumerated()
                        .filter_map(|(variant_idx, _)| {
                            calculate_niche_value(variant_idx).map(|tag| {
                                let variant = variant_info_for(variant_idx);
                                let name = variant.variant_name();

                                Some(unsafe {
                                    llvm::LLVMRustDIBuilderCreateEnumerator(
                                        DIB(cx),
                                        name.as_ptr().cast(),
                                        name.len(),
                                        tag as i64,
                                        !discr_enum_ty.is_signed(),
                                    )
                                })
                            })
                        })
                        .collect();

                    let discr_enum = unsafe {
                        llvm::LLVMRustDIBuilderCreateEnumerationType(
                            DIB(cx),
                            self_metadata,
                            "Discriminant$".as_ptr().cast(),
                            "Discriminant$".len(),
                            unknown_file_metadata(cx),
                            UNKNOWN_LINE_NUMBER,
                            tag.value.size(cx).bits(),
                            tag.value.align(cx).abi.bits() as u32,
                            create_DIArray(DIB(cx), &tags),
                            type_metadata(cx, discr_enum_ty, self.span),
                            true,
                        )
                    };

                    let variant_info = variant_info_for(dataful_variant);
                    let (variant_type_metadata, member_desc_factory) = describe_enum_variant(
                        cx,
                        dataful_variant_layout,
                        variant_info,
                        self_metadata,
                        self.span,
                    );

                    let member_descriptions = member_desc_factory.create_member_descriptions(cx);

                    set_members_of_composite_type(
                        cx,
                        self.enum_type,
                        variant_type_metadata,
                        member_descriptions,
                        Some(&self.common_members),
                    );

                    let (size, align) =
                        cx.size_and_align_of(dataful_variant_layout.field(cx, tag_field).ty);

                    vec![
                        MemberDescription {
                            // Name the dataful variant so that we can identify it for natvis
                            name: "dataful_variant".to_string(),
                            type_metadata: variant_type_metadata,
                            offset: Size::ZERO,
                            size: self.layout.size,
                            align: self.layout.align.abi,
                            flags: DIFlags::FlagZero,
                            discriminant: None,
                            source_info: variant_info.source_info(cx),
                        },
                        MemberDescription {
                            name: "discriminant".into(),
                            type_metadata: discr_enum,
                            offset: dataful_variant_layout.fields.offset(tag_field),
                            size,
                            align,
                            flags: DIFlags::FlagZero,
                            discriminant: None,
                            source_info: None,
                        },
                    ]
                } else {
                    variants
                        .iter_enumerated()
                        .map(|(i, _)| {
                            let variant = self.layout.for_variant(cx, i);
                            let variant_info = variant_info_for(i);
                            let (variant_type_metadata, member_desc_factory) =
                                describe_enum_variant(
                                    cx,
                                    variant,
                                    variant_info,
                                    self_metadata,
                                    self.span,
                                );

                            let member_descriptions =
                                member_desc_factory.create_member_descriptions(cx);

                            set_members_of_composite_type(
                                cx,
                                self.enum_type,
                                variant_type_metadata,
                                member_descriptions,
                                Some(&self.common_members),
                            );

                            let niche_value = calculate_niche_value(i);

                            MemberDescription {
                                name: variant_info.variant_name(),
                                type_metadata: variant_type_metadata,
                                offset: Size::ZERO,
                                size: self.layout.size,
                                align: self.layout.align.abi,
                                flags: DIFlags::FlagZero,
                                discriminant: niche_value,
                                source_info: variant_info.source_info(cx),
                            }
                        })
                        .collect()
                }
            }
        }
    }
}

// Creates `MemberDescription`s for the fields of a single enum variant.
struct VariantMemberDescriptionFactory<'tcx> {
    /// Cloned from the `layout::Struct` describing the variant.
    offsets: Vec<Size>,
    args: Vec<(String, Ty<'tcx>)>,
    span: Span,
}

impl<'tcx> VariantMemberDescriptionFactory<'tcx> {
    fn create_member_descriptions<'ll>(
        &self,
        cx: &CodegenCx<'ll, 'tcx>,
    ) -> Vec<MemberDescription<'ll>> {
        self.args
            .iter()
            .enumerate()
            .map(|(i, &(ref name, ty))| {
                let (size, align) = cx.size_and_align_of(ty);
                MemberDescription {
                    name: name.to_string(),
                    type_metadata: type_metadata(cx, ty, self.span),
                    offset: self.offsets[i],
                    size,
                    align,
                    flags: DIFlags::FlagZero,
                    discriminant: None,
                    source_info: None,
                }
            })
            .collect()
    }
}

#[derive(Copy, Clone)]
enum VariantInfo<'a, 'tcx> {
    Adt(&'tcx ty::VariantDef),
    Generator {
        def_id: DefId,
        generator_layout: &'tcx GeneratorLayout<'tcx>,
        generator_saved_local_names: &'a IndexVec<mir::GeneratorSavedLocal, Option<Symbol>>,
        variant_index: VariantIdx,
    },
}

impl<'tcx> VariantInfo<'_, 'tcx> {
    fn map_struct_name<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        match self {
            VariantInfo::Adt(variant) => f(variant.name.as_str()),
            VariantInfo::Generator { variant_index, .. } => {
                f(&GeneratorSubsts::variant_name(*variant_index))
            }
        }
    }

    fn variant_name(&self) -> String {
        match self {
            VariantInfo::Adt(variant) => variant.name.to_string(),
            VariantInfo::Generator { variant_index, .. } => {
                // Since GDB currently prints out the raw discriminant along
                // with every variant, make each variant name be just the value
                // of the discriminant. The struct name for the variant includes
                // the actual variant description.
                format!("{}", variant_index.as_usize())
            }
        }
    }

    fn field_name(&self, i: usize) -> String {
        let field_name = match *self {
            VariantInfo::Adt(variant) if variant.ctor_kind != CtorKind::Fn => {
                Some(variant.fields[i].name)
            }
            VariantInfo::Generator {
                generator_layout,
                generator_saved_local_names,
                variant_index,
                ..
            } => {
                generator_saved_local_names
                    [generator_layout.variant_fields[variant_index][i.into()]]
            }
            _ => None,
        };
        field_name.map(|name| name.to_string()).unwrap_or_else(|| format!("__{}", i))
    }

    fn source_info<'ll>(&self, cx: &CodegenCx<'ll, 'tcx>) -> Option<SourceInfo<'ll>> {
        if let VariantInfo::Generator { def_id, variant_index, .. } = self {
            let span =
                cx.tcx.generator_layout(*def_id).unwrap().variant_source_info[*variant_index].span;
            if !span.is_dummy() {
                let loc = cx.lookup_debug_loc(span.lo());
                return Some(SourceInfo { file: file_metadata(cx, &loc.file), line: loc.line });
            }
        }
        None
    }
}

/// Returns a tuple of (1) `type_metadata_stub` of the variant, (2) a
/// `MemberDescriptionFactory` for producing the descriptions of the
/// fields of the variant. This is a rudimentary version of a full
/// `RecursiveTypeDescription`.
fn describe_enum_variant<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    layout: layout::TyAndLayout<'tcx>,
    variant: VariantInfo<'_, 'tcx>,
    containing_scope: &'ll DIScope,
    span: Span,
) -> (&'ll DICompositeType, MemberDescriptionFactory<'ll, 'tcx>) {
    let metadata_stub = variant.map_struct_name(|variant_name| {
        let unique_type_id = debug_context(cx)
            .type_map
            .borrow_mut()
            .get_unique_type_id_of_enum_variant(cx, layout.ty, variant_name);
        create_struct_stub(
            cx,
            layout.ty,
            variant_name,
            unique_type_id,
            Some(containing_scope),
            DIFlags::FlagZero,
        )
    });

    let offsets = (0..layout.fields.count()).map(|i| layout.fields.offset(i)).collect();
    let args = (0..layout.fields.count())
        .map(|i| (variant.field_name(i), layout.field(cx, i).ty))
        .collect();

    let member_description_factory =
        VariantMDF(VariantMemberDescriptionFactory { offsets, args, span });

    (metadata_stub, member_description_factory)
}

fn prepare_enum_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    enum_type: Ty<'tcx>,
    enum_def_id: DefId,
    unique_type_id: UniqueTypeId,
    span: Span,
    outer_field_tys: Vec<Ty<'tcx>>,
) -> RecursiveTypeDescription<'ll, 'tcx> {
    let tcx = cx.tcx;
    let enum_name = compute_debuginfo_type_name(tcx, enum_type, false);

    let containing_scope = get_namespace_for_item(cx, enum_def_id);
    // FIXME: This should emit actual file metadata for the enum, but we
    // currently can't get the necessary information when it comes to types
    // imported from other crates. Formerly we violated the ODR when performing
    // LTO because we emitted debuginfo for the same type with varying file
    // metadata, so as a workaround we pretend that the type comes from
    // <unknown>
    let file_metadata = unknown_file_metadata(cx);

    let discriminant_type_metadata = |discr: Primitive| {
        let enumerators_metadata: Vec<_> = match enum_type.kind() {
            ty::Adt(def, _) => iter::zip(def.discriminants(tcx), &def.variants)
                .map(|((_, discr), v)| {
                    let name = v.name.as_str();
                    let is_unsigned = match discr.ty.kind() {
                        ty::Int(_) => false,
                        ty::Uint(_) => true,
                        _ => bug!("non integer discriminant"),
                    };
                    unsafe {
                        Some(llvm::LLVMRustDIBuilderCreateEnumerator(
                            DIB(cx),
                            name.as_ptr().cast(),
                            name.len(),
                            // FIXME: what if enumeration has i128 discriminant?
                            discr.val as i64,
                            is_unsigned,
                        ))
                    }
                })
                .collect(),
            ty::Generator(_, substs, _) => substs
                .as_generator()
                .variant_range(enum_def_id, tcx)
                .map(|variant_index| {
                    debug_assert_eq!(tcx.types.u32, substs.as_generator().discr_ty(tcx));
                    let name = GeneratorSubsts::variant_name(variant_index);
                    unsafe {
                        Some(llvm::LLVMRustDIBuilderCreateEnumerator(
                            DIB(cx),
                            name.as_ptr().cast(),
                            name.len(),
                            // Generators use u32 as discriminant type, verified above.
                            variant_index.as_u32().into(),
                            true, // IsUnsigned
                        ))
                    }
                })
                .collect(),
            _ => bug!(),
        };

        let disr_type_key = (enum_def_id, discr);
        let cached_discriminant_type_metadata =
            debug_context(cx).created_enum_disr_types.borrow().get(&disr_type_key).cloned();
        match cached_discriminant_type_metadata {
            Some(discriminant_type_metadata) => discriminant_type_metadata,
            None => {
                let (discriminant_size, discriminant_align) = (discr.size(cx), discr.align(cx));
                let discriminant_base_type_metadata =
                    type_metadata(cx, discr.to_ty(tcx), rustc_span::DUMMY_SP);

                let item_name;
                let discriminant_name = match enum_type.kind() {
                    ty::Adt(..) => {
                        item_name = tcx.item_name(enum_def_id);
                        item_name.as_str()
                    }
                    ty::Generator(..) => enum_name.as_str(),
                    _ => bug!(),
                };

                let discriminant_type_metadata = unsafe {
                    llvm::LLVMRustDIBuilderCreateEnumerationType(
                        DIB(cx),
                        containing_scope,
                        discriminant_name.as_ptr().cast(),
                        discriminant_name.len(),
                        file_metadata,
                        UNKNOWN_LINE_NUMBER,
                        discriminant_size.bits(),
                        discriminant_align.abi.bits() as u32,
                        create_DIArray(DIB(cx), &enumerators_metadata),
                        discriminant_base_type_metadata,
                        true,
                    )
                };

                debug_context(cx)
                    .created_enum_disr_types
                    .borrow_mut()
                    .insert(disr_type_key, discriminant_type_metadata);

                discriminant_type_metadata
            }
        }
    };

    let layout = cx.layout_of(enum_type);

    if let (Abi::Scalar(_), Variants::Multiple { tag_encoding: TagEncoding::Direct, tag, .. }) =
        (layout.abi, &layout.variants)
    {
        return FinalMetadata(discriminant_type_metadata(tag.value));
    }

    // While LLVM supports generating debuginfo for variant types (enums), it doesn't support
    // lowering that debuginfo to CodeView records for msvc targets. So if we are targeting
    // msvc, then we need to use a different encoding of the debuginfo.
    if cpp_like_debuginfo(tcx) {
        let discriminant_type_metadata = match layout.variants {
            Variants::Single { .. } => None,
            Variants::Multiple { tag_encoding: TagEncoding::Niche { .. }, tag, .. }
            | Variants::Multiple { tag_encoding: TagEncoding::Direct, tag, .. } => {
                Some(discriminant_type_metadata(tag.value))
            }
        };

        let enum_metadata = {
            let type_map = debug_context(cx).type_map.borrow();
            let unique_type_id_str = type_map.get_unique_type_id_as_string(unique_type_id);

            unsafe {
                llvm::LLVMRustDIBuilderCreateUnionType(
                    DIB(cx),
                    None,
                    enum_name.as_ptr().cast(),
                    enum_name.len(),
                    file_metadata,
                    UNKNOWN_LINE_NUMBER,
                    layout.size.bits(),
                    layout.align.abi.bits() as u32,
                    DIFlags::FlagZero,
                    None,
                    0, // RuntimeLang
                    unique_type_id_str.as_ptr().cast(),
                    unique_type_id_str.len(),
                )
            }
        };

        return create_and_register_recursive_type_forward_declaration(
            cx,
            enum_type,
            unique_type_id,
            enum_metadata,
            enum_metadata,
            EnumMDF(EnumMemberDescriptionFactory {
                enum_type,
                layout,
                tag_type_metadata: discriminant_type_metadata,
                common_members: vec![],
                span,
            }),
        );
    }

    let discriminator_name = match enum_type.kind() {
        ty::Generator(..) => "__state",
        _ => "",
    };
    let discriminator_metadata = match layout.variants {
        // A single-variant enum has no discriminant.
        Variants::Single { .. } => None,

        Variants::Multiple { tag_encoding: TagEncoding::Niche { .. }, tag, tag_field, .. } => {
            // Find the integer type of the correct size.
            let size = tag.value.size(cx);
            let align = tag.value.align(cx);

            let tag_type = match tag.value {
                Int(t, _) => t,
                F32 => Integer::I32,
                F64 => Integer::I64,
                Pointer => cx.data_layout().ptr_sized_integer(),
            }
            .to_ty(cx.tcx, false);

            let tag_metadata = basic_type_metadata(cx, tag_type);
            unsafe {
                Some(llvm::LLVMRustDIBuilderCreateMemberType(
                    DIB(cx),
                    containing_scope,
                    discriminator_name.as_ptr().cast(),
                    discriminator_name.len(),
                    file_metadata,
                    UNKNOWN_LINE_NUMBER,
                    size.bits(),
                    align.abi.bits() as u32,
                    layout.fields.offset(tag_field).bits(),
                    DIFlags::FlagArtificial,
                    tag_metadata,
                ))
            }
        }

        Variants::Multiple { tag_encoding: TagEncoding::Direct, tag, tag_field, .. } => {
            let discr_type = tag.value.to_ty(cx.tcx);
            let (size, align) = cx.size_and_align_of(discr_type);

            let discr_metadata = basic_type_metadata(cx, discr_type);
            unsafe {
                Some(llvm::LLVMRustDIBuilderCreateMemberType(
                    DIB(cx),
                    containing_scope,
                    discriminator_name.as_ptr().cast(),
                    discriminator_name.len(),
                    file_metadata,
                    UNKNOWN_LINE_NUMBER,
                    size.bits(),
                    align.bits() as u32,
                    layout.fields.offset(tag_field).bits(),
                    DIFlags::FlagArtificial,
                    discr_metadata,
                ))
            }
        }
    };

    let outer_fields = match layout.variants {
        Variants::Single { .. } => vec![],
        Variants::Multiple { .. } => {
            let tuple_mdf = TupleMemberDescriptionFactory {
                ty: enum_type,
                component_types: outer_field_tys,
                span,
            };
            tuple_mdf
                .create_member_descriptions(cx)
                .into_iter()
                .map(|desc| Some(desc.into_metadata(cx, containing_scope)))
                .collect()
        }
    };

    let variant_part_unique_type_id_str = debug_context(cx)
        .type_map
        .borrow_mut()
        .get_unique_type_id_str_of_enum_variant_part(unique_type_id);
    let empty_array = create_DIArray(DIB(cx), &[]);
    let name = "";
    let variant_part = unsafe {
        llvm::LLVMRustDIBuilderCreateVariantPart(
            DIB(cx),
            containing_scope,
            name.as_ptr().cast(),
            name.len(),
            file_metadata,
            UNKNOWN_LINE_NUMBER,
            layout.size.bits(),
            layout.align.abi.bits() as u32,
            DIFlags::FlagZero,
            discriminator_metadata,
            empty_array,
            variant_part_unique_type_id_str.as_ptr().cast(),
            variant_part_unique_type_id_str.len(),
        )
    };

    let struct_wrapper = {
        // The variant part must be wrapped in a struct according to DWARF.
        // All fields except the discriminant (including `outer_fields`)
        // should be put into structures inside the variant part, which gives
        // an equivalent layout but offers us much better integration with
        // debuggers.
        let type_array = create_DIArray(DIB(cx), &[Some(variant_part)]);

        let type_map = debug_context(cx).type_map.borrow();
        let unique_type_id_str = type_map.get_unique_type_id_as_string(unique_type_id);

        unsafe {
            llvm::LLVMRustDIBuilderCreateStructType(
                DIB(cx),
                Some(containing_scope),
                enum_name.as_ptr().cast(),
                enum_name.len(),
                file_metadata,
                UNKNOWN_LINE_NUMBER,
                layout.size.bits(),
                layout.align.abi.bits() as u32,
                DIFlags::FlagZero,
                None,
                type_array,
                0,
                None,
                unique_type_id_str.as_ptr().cast(),
                unique_type_id_str.len(),
            )
        }
    };

    create_and_register_recursive_type_forward_declaration(
        cx,
        enum_type,
        unique_type_id,
        struct_wrapper,
        variant_part,
        EnumMDF(EnumMemberDescriptionFactory {
            enum_type,
            layout,
            tag_type_metadata: None,
            common_members: outer_fields,
            span,
        }),
    )
}

/// Creates debug information for a composite type, that is, anything that
/// results in a LLVM struct.
///
/// Examples of Rust types to use this are: structs, tuples, boxes, vecs, and enums.
fn composite_type_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    composite_type: Ty<'tcx>,
    composite_type_name: &str,
    composite_type_unique_id: UniqueTypeId,
    member_descriptions: Vec<MemberDescription<'ll>>,
    containing_scope: Option<&'ll DIScope>,

    // Ignore source location information as long as it
    // can't be reconstructed for non-local crates.
    _file_metadata: &'ll DIFile,
    _definition_span: Span,
) -> &'ll DICompositeType {
    // Create the (empty) struct metadata node ...
    let composite_type_metadata = create_struct_stub(
        cx,
        composite_type,
        composite_type_name,
        composite_type_unique_id,
        containing_scope,
        DIFlags::FlagZero,
    );
    // ... and immediately create and add the member descriptions.
    set_members_of_composite_type(
        cx,
        composite_type,
        composite_type_metadata,
        member_descriptions,
        None,
    );

    composite_type_metadata
}

fn set_members_of_composite_type<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    composite_type: Ty<'tcx>,
    composite_type_metadata: &'ll DICompositeType,
    member_descriptions: Vec<MemberDescription<'ll>>,
    common_members: Option<&Vec<Option<&'ll DIType>>>,
) {
    // In some rare cases LLVM metadata uniquing would lead to an existing type
    // description being used instead of a new one created in
    // create_struct_stub. This would cause a hard to trace assertion in
    // DICompositeType::SetTypeArray(). The following check makes sure that we
    // get a better error message if this should happen again due to some
    // regression.
    {
        let mut composite_types_completed =
            debug_context(cx).composite_types_completed.borrow_mut();
        if !composite_types_completed.insert(composite_type_metadata) {
            bug!(
                "debuginfo::set_members_of_composite_type() - \
                  Already completed forward declaration re-encountered."
            );
        }
    }

    let mut member_metadata: Vec<_> = member_descriptions
        .into_iter()
        .map(|desc| Some(desc.into_metadata(cx, composite_type_metadata)))
        .collect();
    if let Some(other_members) = common_members {
        member_metadata.extend(other_members.iter());
    }

    let type_params = compute_type_parameters(cx, composite_type);
    unsafe {
        let type_array = create_DIArray(DIB(cx), &member_metadata);
        llvm::LLVMRustDICompositeTypeReplaceArrays(
            DIB(cx),
            composite_type_metadata,
            Some(type_array),
            Some(type_params),
        );
    }
}

/// Computes the type parameters for a type, if any, for the given metadata.
fn compute_type_parameters<'ll, 'tcx>(cx: &CodegenCx<'ll, 'tcx>, ty: Ty<'tcx>) -> &'ll DIArray {
    if let ty::Adt(def, substs) = *ty.kind() {
        if substs.types().next().is_some() {
            let generics = cx.tcx.generics_of(def.did);
            let names = get_parameter_names(cx, generics);
            let template_params: Vec<_> = iter::zip(substs, names)
                .filter_map(|(kind, name)| {
                    if let GenericArgKind::Type(ty) = kind.unpack() {
                        let actual_type =
                            cx.tcx.normalize_erasing_regions(ParamEnv::reveal_all(), ty);
                        let actual_type_metadata =
                            type_metadata(cx, actual_type, rustc_span::DUMMY_SP);
                        let name = name.as_str();
                        Some(unsafe {
                            Some(llvm::LLVMRustDIBuilderCreateTemplateTypeParameter(
                                DIB(cx),
                                None,
                                name.as_ptr().cast(),
                                name.len(),
                                actual_type_metadata,
                            ))
                        })
                    } else {
                        None
                    }
                })
                .collect();

            return create_DIArray(DIB(cx), &template_params);
        }
    }
    return create_DIArray(DIB(cx), &[]);

    fn get_parameter_names(cx: &CodegenCx<'_, '_>, generics: &ty::Generics) -> Vec<Symbol> {
        let mut names = generics
            .parent
            .map_or_else(Vec::new, |def_id| get_parameter_names(cx, cx.tcx.generics_of(def_id)));
        names.extend(generics.params.iter().map(|param| param.name));
        names
    }
}

/// A convenience wrapper around `LLVMRustDIBuilderCreateStructType()`. Does not do
/// any caching, does not add any fields to the struct. This can be done later
/// with `set_members_of_composite_type()`.
fn create_struct_stub<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    struct_type: Ty<'tcx>,
    struct_type_name: &str,
    unique_type_id: UniqueTypeId,
    containing_scope: Option<&'ll DIScope>,
    flags: DIFlags,
) -> &'ll DICompositeType {
    let (struct_size, struct_align) = cx.size_and_align_of(struct_type);

    let type_map = debug_context(cx).type_map.borrow();
    let unique_type_id = type_map.get_unique_type_id_as_string(unique_type_id);

    let metadata_stub = unsafe {
        // `LLVMRustDIBuilderCreateStructType()` wants an empty array. A null
        // pointer will lead to hard to trace and debug LLVM assertions
        // later on in `llvm/lib/IR/Value.cpp`.
        let empty_array = create_DIArray(DIB(cx), &[]);

        llvm::LLVMRustDIBuilderCreateStructType(
            DIB(cx),
            containing_scope,
            struct_type_name.as_ptr().cast(),
            struct_type_name.len(),
            unknown_file_metadata(cx),
            UNKNOWN_LINE_NUMBER,
            struct_size.bits(),
            struct_align.bits() as u32,
            flags,
            None,
            empty_array,
            0,
            None,
            unique_type_id.as_ptr().cast(),
            unique_type_id.len(),
        )
    };

    metadata_stub
}

fn create_union_stub<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    union_type: Ty<'tcx>,
    union_type_name: &str,
    unique_type_id: UniqueTypeId,
    containing_scope: &'ll DIScope,
) -> &'ll DICompositeType {
    let (union_size, union_align) = cx.size_and_align_of(union_type);

    let type_map = debug_context(cx).type_map.borrow();
    let unique_type_id = type_map.get_unique_type_id_as_string(unique_type_id);

    let metadata_stub = unsafe {
        // `LLVMRustDIBuilderCreateUnionType()` wants an empty array. A null
        // pointer will lead to hard to trace and debug LLVM assertions
        // later on in `llvm/lib/IR/Value.cpp`.
        let empty_array = create_DIArray(DIB(cx), &[]);

        llvm::LLVMRustDIBuilderCreateUnionType(
            DIB(cx),
            Some(containing_scope),
            union_type_name.as_ptr().cast(),
            union_type_name.len(),
            unknown_file_metadata(cx),
            UNKNOWN_LINE_NUMBER,
            union_size.bits(),
            union_align.bits() as u32,
            DIFlags::FlagZero,
            Some(empty_array),
            0, // RuntimeLang
            unique_type_id.as_ptr().cast(),
            unique_type_id.len(),
        )
    };

    metadata_stub
}

/// Creates debug information for the given global variable.
///
/// Adds the created metadata nodes directly to the crate's IR.
pub fn create_global_var_metadata<'ll>(cx: &CodegenCx<'ll, '_>, def_id: DefId, global: &'ll Value) {
    if cx.dbg_cx.is_none() {
        return;
    }

    // Only create type information if full debuginfo is enabled
    if cx.sess().opts.debuginfo != DebugInfo::Full {
        return;
    }

    let tcx = cx.tcx;

    // We may want to remove the namespace scope if we're in an extern block (see
    // https://github.com/rust-lang/rust/pull/46457#issuecomment-351750952).
    let var_scope = get_namespace_for_item(cx, def_id);
    let span = tcx.def_span(def_id);

    let (file_metadata, line_number) = if !span.is_dummy() {
        let loc = cx.lookup_debug_loc(span.lo());
        (file_metadata(cx, &loc.file), loc.line)
    } else {
        (unknown_file_metadata(cx), UNKNOWN_LINE_NUMBER)
    };

    let is_local_to_unit = is_node_local_to_unit(cx, def_id);
    let variable_type = Instance::mono(cx.tcx, def_id).ty(cx.tcx, ty::ParamEnv::reveal_all());
    let type_metadata = type_metadata(cx, variable_type, span);
    let var_name = tcx.item_name(def_id);
    let var_name = var_name.as_str();
    let linkage_name = mangled_name_of_instance(cx, Instance::mono(tcx, def_id)).name;
    // When empty, linkage_name field is omitted,
    // which is what we want for no_mangle statics
    let linkage_name = if var_name == linkage_name { "" } else { linkage_name };

    let global_align = cx.align_of(variable_type);

    unsafe {
        llvm::LLVMRustDIBuilderCreateStaticVariable(
            DIB(cx),
            Some(var_scope),
            var_name.as_ptr().cast(),
            var_name.len(),
            linkage_name.as_ptr().cast(),
            linkage_name.len(),
            file_metadata,
            line_number,
            type_metadata,
            is_local_to_unit,
            global,
            None,
            global_align.bytes() as u32,
        );
    }
}

/// Generates LLVM debuginfo for a vtable.
fn vtable_type_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    ty: Ty<'tcx>,
    poly_trait_ref: Option<ty::PolyExistentialTraitRef<'tcx>>,
) -> &'ll DIType {
    let tcx = cx.tcx;

    let vtable_entries = if let Some(poly_trait_ref) = poly_trait_ref {
        let trait_ref = poly_trait_ref.with_self_ty(tcx, ty);
        let trait_ref = tcx.erase_regions(trait_ref);

        tcx.vtable_entries(trait_ref)
    } else {
        COMMON_VTABLE_ENTRIES
    };

    // FIXME: We describe the vtable as an array of *const () pointers. The length of the array is
    //        correct - but we could create a more accurate description, e.g. by describing it
    //        as a struct where each field has a name that corresponds to the name of the method
    //        it points to.
    //        However, this is not entirely straightforward because there might be multiple
    //        methods with the same name if the vtable is for multiple traits. So for now we keep
    //        things simple instead of adding some ad-hoc disambiguation scheme.
    let vtable_type = tcx.mk_array(tcx.mk_imm_ptr(tcx.types.unit), vtable_entries.len() as u64);

    type_metadata(cx, vtable_type, rustc_span::DUMMY_SP)
}

/// Creates debug information for the given vtable, which is for the
/// given type.
///
/// Adds the created metadata nodes directly to the crate's IR.
pub fn create_vtable_metadata<'ll, 'tcx>(
    cx: &CodegenCx<'ll, 'tcx>,
    ty: Ty<'tcx>,
    poly_trait_ref: Option<ty::PolyExistentialTraitRef<'tcx>>,
    vtable: &'ll Value,
) {
    if cx.dbg_cx.is_none() {
        return;
    }

    // Only create type information if full debuginfo is enabled
    if cx.sess().opts.debuginfo != DebugInfo::Full {
        return;
    }

    let vtable_name = compute_debuginfo_vtable_name(cx.tcx, ty, poly_trait_ref);
    let vtable_type = vtable_type_metadata(cx, ty, poly_trait_ref);

    unsafe {
        let linkage_name = "";
        llvm::LLVMRustDIBuilderCreateStaticVariable(
            DIB(cx),
            NO_SCOPE_METADATA,
            vtable_name.as_ptr().cast(),
            vtable_name.len(),
            linkage_name.as_ptr().cast(),
            linkage_name.len(),
            unknown_file_metadata(cx),
            UNKNOWN_LINE_NUMBER,
            vtable_type,
            true,
            vtable,
            None,
            0,
        );
    }
}

/// Creates an "extension" of an existing `DIScope` into another file.
pub fn extend_scope_to_file<'ll>(
    cx: &CodegenCx<'ll, '_>,
    scope_metadata: &'ll DIScope,
    file: &SourceFile,
) -> &'ll DILexicalBlock {
    let file_metadata = file_metadata(cx, file);
    unsafe { llvm::LLVMRustDIBuilderCreateLexicalBlockFile(DIB(cx), scope_metadata, file_metadata) }
}
