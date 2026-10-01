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

impl GotocCtx<'_, '_> {
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
    fn peel_ptr_wrappers(&self, mut expr: Expr) -> Expr {
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
}

pub fn span_err(tcx: TyCtxt, span: Span, msg: String) {
    tcx.dcx().span_err(rustc_internal::internal(tcx, span), msg);
}
