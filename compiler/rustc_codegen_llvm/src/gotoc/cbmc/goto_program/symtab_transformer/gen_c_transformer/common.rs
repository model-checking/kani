use super::super::super::Type;

/// Create a string representation of type for use as variable name suffix.
pub fn type_to_string(typ: &Type) -> String {
    match typ {
        Type::Array { typ, size } => format!("array_of_{}_{}", size, type_to_string(typ.as_ref())),
        Type::Bool => format!("bool"),
        Type::CBitField { width, typ } => format!("cbitfield_of_{}_{}", width, type_to_string(typ.as_ref())),
        Type::CInteger(int_kind) => format!("c_int_{:?}", int_kind),
        Type::Code { parameters, return_type } => {
            let parameter_string = parameters.iter()
                .map(|param| param.typ())
                .map(type_to_string)
                .collect::<Vec<_>>()
                .join("_");
            let return_string = type_to_string(ret_type.as_ref());
            format!("code_from_{}_to_{}", parameter_string, return_string)
        }
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
        Type::StructTag(tag) => format!("struct_tag_{}", tag),
        Type::Union { tag, .. } => format!("union_{}", tag),
        Type::UnionTag(tag) => format!("union_tag_{}", tag),
        Type::Unsignedbv { width } => format!("unsigned_bv_{}", width),
        Type::VariadicCode { parameters, return_type } => {
            let parameter_string = parameters.iter()
                .map(|param| param.typ())
                .map(type_to_string)
                .collect::<Vec<_>>()
                .join("_");
            let return_string = type_to_string(ret_type.as_ref());
            format!("variadic_code_from_{}_to_{}", parameter_string, return_string)
        },
        Type::Vector { size, typ } => format!("vec_of_{}_{}", size, type_to_string(typ.as_ref())),
    }
}
