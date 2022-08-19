// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::codegen::TypeExt;
use crate::codegen_cprover_gotoc::codegen::typ::{is_pointer, pointee_type};
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Expr, ExprValue, Location, Stmt, Symbol, SymbolTable, Type};
use cbmc::{btree_string_map, InternedString};
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{Instance, Ty};
use tracing::debug;

// Should move into rvalue
//make this a member function
pub fn slice_fat_ptr(typ: Type, data: Expr, len: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("len", len)], symbol_table)
}

pub fn dynamic_fat_ptr(typ: Type, data: Expr, vtable: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("vtable", vtable)], symbol_table)
}

impl<'tcx> GotocCtx<'tcx> {
    /// Generates an expression `(ptr as usize) % align_of(T) == 0`
    /// to determine if a pointer `ptr` with pointee type `T` is aligned.
    pub fn is_ptr_aligned(&mut self, typ: Ty<'tcx>, ptr: Expr) -> Expr {
        // Ensure `typ` is a pointer, then extract the pointee type
        assert!(is_pointer(typ));
        let pointee_type = pointee_type(typ).unwrap();
        // Obtain the alignment for the pointee type `T`
        let layout = self.layout_of(pointee_type);
        let align = Expr::int_constant(layout.align.abi.bytes(), Type::size_t());
        // Cast the pointer to `usize` and return the alignment expression
        let cast_ptr = ptr.cast_to(Type::size_t());
        let zero = Type::size_t().zero();
        cast_ptr.rem(align).eq(zero)
    }

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

impl<'tcx> GotocCtx<'tcx> {
    /// Given an "instance" find the crate it came from
    pub fn get_crate(&self, instance: Instance<'tcx>) -> String {
        self.tcx.crate_name(instance.def_id().krate).to_string()
    }
}

impl<'tcx> GotocCtx<'tcx> {
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
        RAW_PTR_FROM_BOX.iter().fold(box_expr, |expr, name| expr.member(name, &self.symbol_table))
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
    fn assert_is_rust_phantom_data_like(&self, t: &Type) {
        // TODO: A `std::marker::PhantomData` appears to be an empty struct, in the cases we've seen.
        // Is there something smarter we can do here?
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 0);
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

    /// Best effort check if the struct represents a `std::ptr::NonNull<T>`.
    ///
    /// This assumes the following structure. Any changes to this will break this code.
    /// ```
    /// pub struct NonNull<T: ?Sized> {
    ///    pointer: *const T,
    /// }
    /// ```
    fn assert_is_non_null_like(&self, t: &Type) {
        assert!(t.is_struct_like());
        let components = t.lookup_components(&self.symbol_table).unwrap();
        assert_eq!(components.len(), 1);
        let component = components.first().unwrap();
        assert_eq!(component.name().to_string().as_str(), "pointer");
        assert!(component.typ().is_pointer() || component.typ().is_rust_fat_ptr(&self.symbol_table))
    }
}

impl<'tcx> GotocCtx<'tcx> {
    /// Iterates over the list of local declarations of the current function and
    /// returns the corresponding symbol from the symbol table, if the base name matches the target.
    pub fn lookup_local_decl_by_name(&mut self, target: &str) -> Option<&Symbol> {
        let mir = self.current_fn().mir();
        let ldecls = &mir.local_decls;
        let sym_name = ldecls
            .indices()
            .into_iter()
            .find(|lc| self.codegen_var_base_name(&lc) == target)
            .map_or_else(|| None, |lc| Some(self.codegen_var_name(&lc)))?;
        self.symbol_table.lookup(sym_name)
    }
}
