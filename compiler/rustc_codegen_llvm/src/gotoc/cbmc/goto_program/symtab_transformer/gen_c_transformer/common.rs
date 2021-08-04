use super::super::super::Type;

/// Create a string representation of type for use as variable name suffix.
pub fn type_to_string(typ: &Type) -> String {
    match typ {
        Type::Array { typ, size } => format!("array_of_{}_{}", size, type_to_string(typ.as_ref())),
        Type::Bool => format!("bool"),
        Type::CBitField { typ, .. } => format!("cbitfield_of_{}", type_to_string(typ.as_ref())),
        Type::CInteger(_) => format!("c_int"),
        Type::Code { .. } => format!("code"),
        Type::Constructor => format!("constructor"),
        Type::Double => format!("double"),
        Type::Empty => format!("empty"),
        Type::FlexibleArray { typ } => format!("flexarray_of_{}", type_to_string(typ.as_ref())),
        Type::Float => format!("float"),
        Type::IncompleteStruct { tag } => tag.clone(),
        Type::IncompleteUnion { tag } => tag.clone(),
        Type::InfiniteArray { typ } => {
            format!("infinite_array_of_{}", type_to_string(typ.as_ref()))
        }
        Type::Pointer { typ } => format!("pointer_to_{}", type_to_string(typ.as_ref())),
        Type::Signedbv { width } => format!("signed_bv_{}", width),
        Type::Struct { tag, .. } => format!("struct_{}", tag),
        Type::StructTag(tag) => format!("struct_{}", tag),
        Type::Union { tag, .. } => format!("union_{}", tag),
        Type::UnionTag(tag) => format!("union_{}", tag),
        Type::Unsignedbv { width } => format!("unsigned_bv_{}", width),
        Type::VariadicCode { .. } => format!("variadic_code"),
        Type::Vector { typ, .. } => format!("vec_of_{}", type_to_string(typ.as_ref())),
    }
}
