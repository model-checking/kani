// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::codegen::TypeExt;
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Expr, ExprValue, Location, SymbolTable, Type};
use cbmc::{InternedString, btree_string_map};
use rustc_middle::ty::TyCtxt;
use rustc_public::rustc_internal;
use rustc_public::ty::Span;
use tracing::debug;

// Should move into rvalue
//make this a member function
pub fn slice_fat_ptr(typ: Type, data: Expr, len: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("len", len)], symbol_table)
}

pub fn dynamic_fat_ptr(typ: Type, data: Expr, vtable: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("vtable", vtable)], symbol_table)
}

impl GotocCtx<'_, '_> {
    pub fn unsupported_msg(item: &str, url: Option<&str>) -> String {
        let mut s = format!("{item} is not currently supported by Kani");
        if let Some(url) = url {
            s.push_str(". Please post your example at ");
            s.push_str(url);
        }
        s
    }

    /// Tries to extract a string message from an `Expr`.  If the expression is
    /// a pointer to a variable that represents a string literal (as created in
    /// `codegen_slice_value`), this will return the string constant. Otherwise,
    /// return `None`.
    pub fn extract_const_message(&self, arg: &Expr) -> Option<String> {
        match arg.value() {
            ExprValue::Struct { values } => match &values[0].value() {
                ExprValue::Typecast(expr) => match expr.value() {
                    ExprValue::AddressOf(address) => match address.value() {
                        ExprValue::Symbol { identifier } => {
                            // lookup the string literal in the goto context
                            let name = self.str_literals.get(identifier).unwrap();
                            Some(name.clone())
                        }
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    /// Store an occurrence of a concurrent construct that was treated as a sequential operation.
    ///
    /// Kani does not currently support concurrency and the compiler assumes that when generating
    /// code for some specialized concurrent constructs that this is the case. We store all types of
    /// operations that had this special handling and print a warning at the end of the compilation.
    pub fn store_concurrent_construct(&mut self, operation_name: &str, loc: Location) {
        debug!(op=?operation_name, location=?loc.short_string(), "store_seq_construct");

        // Save this occurrence so we can emit a warning in the compilation report.
        let key: InternedString = operation_name.into();
        let entry = self.concurrent_constructs.entry(key).or_default();
        entry.push(loc);
    }
}

/// Members traverse path to get to the raw pointer of a box (b.0.pointer.pointer).
const RAW_PTR_FROM_BOX: [&str; 3] = ["0", "pointer", "pointer"];

impl GotocCtx<'_, '_> {
    /// Dereference a boxed type `std::boxed::Box<T>` to get a `*T`.
    ///
    /// WARNING: This is based on a manual inspection of how boxed types are currently
    /// a) implemented by the rust standard library
    /// b) codegenned by Kani.
    /// If either of those change, this will almost certainly stop working.
    pub fn deref_box(&self, box_expr: Expr) -> Expr {
        // Internally, a Boxed type is stored as a chain of structs.
        //
        // This code has to match the exact structure from the std library version that is
        // supported to access the raw pointer. If either rustc or Kani changes how boxed types are
        // represented, this will need to be updated.
        self.assert_is_rust_box_like(box_expr.typ());
        let expr = RAW_PTR_FROM_BOX
            .iter()
            .fold(box_expr, |expr, name| expr.member(name, &self.symbol_table));
        // `NonNull`'s `pointer` field holds a `pattern_type!(*const T is !null)`, which is itself
        // codegenned as a struct, so the chain above stops one level above the raw pointer.
        self.peel_ptr_wrappers(expr)
    }

    /// `Box<T>` initializer
    ///
    /// Traverse over the Box representation and only initialize the raw_ptr field. All other
    /// members are left uninitialized.
    /// `boxed_type` is the type of the resulting expression
    pub fn box_value(&self, boxed_value: Expr, boxed_type: Type) -> Expr {
        self.assert_is_rust_box_like(&boxed_type);
        tracing::debug!(?boxed_type, ?boxed_value, "box_value");
        let mut inner_type = boxed_type;
        let type_members = RAW_PTR_FROM_BOX
            .iter()
            .map(|name| {
                let outer_type = inner_type.clone();
                inner_type = outer_type.lookup_field_type(name, &self.symbol_table).unwrap();
                (*name, outer_type)
            })
            .collect::<Vec<_>>();

        // `inner_type` is now the innermost field's type, which wraps the raw pointer in a
        // pattern-type struct. Rebuild that wrapping so the value matches the field.
        let boxed_value = self.codegen_ptr_in_wrappers(inner_type, boxed_value);

        type_members.iter().rfold(boxed_value, |value, (name, typ)| {
            Expr::struct_expr_with_nondet_fields(
                typ.clone(),
                btree_string_map![(*name, value),],
                &self.symbol_table,
            )
        })
    }

    /// Best effort check if the struct represents a rust `std::alloc::Global`
    fn assert_is_rust_global_alloc_like(&self, t: &Type) {
        // TODO: A `std::alloc::Global` appears to be an empty struct, in the cases we've seen.
        // Is there something smarter we can do here?
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 0);
    }

    /// Best effort check if the struct represents a rust `std::marker::PhantomData`
    pub fn assert_is_rust_phantom_data_like(&self, t: &Type) {
        // TODO: A `std::marker::PhantomData` appears to be an empty struct, in the cases we've seen.
        // Is there something smarter we can do here?
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 0);
    }

    /// Rebuild the chain of single-field struct wrappers described by `typ` around `ptr_expr`,
    /// casting the pointer to the type found at the innermost level.
    ///
    /// A `NonNull<T>` holds a `pattern_type!(*const T is !null)`, and Kani codegens both the
    /// `NonNull` and the pattern type as single-field structs, so the raw pointer sits two levels
    /// deep. Recursing keeps this independent of how many wrappers the standard library uses.
    pub fn codegen_ptr_in_wrappers(&self, typ: Type, ptr_expr: Expr) -> Expr {
        // A fat pointer is itself a two-field struct; it is the pointer, not a wrapper around one.
        if !typ.is_struct_like() || typ.is_rust_fat_ptr(&self.symbol_table) {
            return ptr_expr.cast_to(typ);
        }
        let components = typ.lookup_components(&self.symbol_table).unwrap();
        let fields: Vec<_> = components.iter().filter(|c| !c.is_padding()).collect();
        assert_eq!(
            fields.len(),
            1,
            "Expected a single-field struct wrapping a pointer, but found {typ:?}"
        );
        let inner = self.codegen_ptr_in_wrappers(fields[0].typ(), ptr_expr);
        Expr::struct_expr_from_values(typ, vec![inner], &self.symbol_table)
    }

    /// Extract the vtable pointer that `metadata_expr`, a `DynMetadata`, holds in its
    /// `_vtable_ptr` field, and cast it to `vtable_typ`.
    ///
    /// This is the inverse of [`Self::codegen_ptr_in_wrappers`]: peel off the single-field struct
    /// wrappers until the raw pointer is reached.
    pub fn codegen_ptr_out_of_wrappers(&self, metadata_expr: Expr, vtable_typ: Type) -> Expr {
        let expr = self.peel_ptr_wrappers(metadata_expr.member("_vtable_ptr", &self.symbol_table));
        expr.cast_to(vtable_typ)
    }

    /// Peel off the single-field struct wrappers around `expr` until the pointer is reached.
    ///
    /// Returns `expr` unchanged when it is already a pointer, so this is a no-op for layouts
    /// that do not wrap.
    pub fn peel_ptr_wrappers(&self, mut expr: Expr) -> Expr {
        while expr.typ().is_struct_like() && !expr.typ().is_rust_fat_ptr(&self.symbol_table) {
            let components = expr.typ().lookup_components(&self.symbol_table).unwrap();
            let fields: Vec<_> = components.iter().filter(|c| !c.is_padding()).collect();
            assert_eq!(
                fields.len(),
                1,
                "Expected a single-field struct wrapping a pointer, but found {:?}",
                expr.typ()
            );
            expr = expr.member(fields[0].name(), &self.symbol_table);
        }
        expr
    }

    /// Best effort check if the struct represents a Rust `Box`. May return false positives.
    fn assert_is_rust_box_like(&self, t: &Type) {
        // struct std::boxed::Box<[u8; 8]>::15334369982748499855
        // {
        //   // 1
        //   struct std::alloc::Global::13633191317886109837 1;
        //   // 0
        //   struct std::ptr::Unique<[u8; 8]>::14713681870393313245 0;
        // };
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 2);
        for c in components {
            match c.name().to_string().as_str() {
                "0" => self.assert_is_rust_unique_pointer_like(&c.typ()),
                "1" => self.assert_is_rust_global_alloc_like(&c.typ()),
                _ => panic!("Unexpected component {} in {t:?}", c.name()),
            }
        }
    }

    /// Checks if the struct represents a Rust `std::ptr::Unique`
    fn assert_is_rust_unique_pointer_like(&self, t: &Type) {
        // struct std::ptr::Unique<[u8; 8]>::14713681870393313245
        // {
        //   // _marker
        //   struct std::marker::PhantomData<[u8; 8]>::18073278521438838603 _marker;
        //   // pointer
        //   NonNull<T> pointer;
        // };
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 2);
        for c in components {
            match c.name().to_string().as_str() {
                "_marker" => self.assert_is_rust_phantom_data_like(&c.typ()),
                "pointer" => self.assert_is_non_null_like(&c.typ()),
                _ => panic!("Unexpected component {} in {t:?}", c.name()),
            }
        }
    }

    /// Whether `t` is a pointer, or a chain of single-field struct wrappers around one.
    ///
    /// Recursing keeps this independent of how many wrappers the standard library uses, and
    /// mirrors the chain that [`Self::codegen_ptr_in_wrappers`] rebuilds.
    fn is_wrapped_pointer(&self, t: &Type) -> bool {
        if t.is_pointer() || t.is_rust_fat_ptr(&self.symbol_table) {
            return true;
        }
        if !t.is_struct_like() {
            return false;
        }
        let Some(components) = t.lookup_components(&self.symbol_table) else {
            return false;
        };
        let fields: Vec<_> = components.iter().filter(|c| !c.is_padding()).collect();
        fields.len() == 1 && self.is_wrapped_pointer(&fields[0].typ())
    }

    /// Best effort check if the struct represents a `std::ptr::NonNull<T>`.
    ///
    /// This assumes the following structure. Any changes to this will break this code.
    /// ```
    /// pub struct NonNull<T: ?Sized> {
    ///    pointer: pattern_type!(*const T is !null),
    /// }
    /// ```
    /// The `pointer` field is not a bare pointer: the pattern type is itself codegenned as a
    /// single-field struct, so the pointer sits one level further down, c.f.
    /// [`Self::codegen_ptr_in_wrappers`].
    fn assert_is_non_null_like(&self, t: &Type) {
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 1);
        let component = components.first().unwrap();
        assert_eq!(component.name().to_string().as_str(), "pointer");
        assert!(
            self.is_wrapped_pointer(&component.typ()),
            "Expected the `pointer` field of {t:?} to hold a pointer, but found {:?}",
            component.typ()
        )
    }
}

pub fn span_err(tcx: TyCtxt, span: Span, msg: String) {
    tcx.dcx().span_err(rustc_internal::internal(tcx, span), msg);
}
