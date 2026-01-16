// Copyright (c) 2026 Kirky.X
//! Generic type parsing for advanced generic support
//!
//! This module provides utilities for parsing and handling generic types
//! in function signatures, including complex constraints and nested generics.

use syn::{GenericParam, Generics, Lifetime, PathArguments, Type};

/// Information about a generic parameter, including its bounds and lifetimes.
#[derive(Debug, Clone)]
pub struct GenericParamInfo {
    pub name: String,
    pub bounds: Vec<GenericBound>,
    pub lifetimes: Vec<Lifetime>,
}

/// Generic bound information - either a trait or lifetime constraint.
#[derive(Debug, Clone, PartialEq)]
pub enum GenericBound {
    Trait(String),
    Lifetime(String),
}

/// Parse generic parameters from function signature.
///
/// Returns a vector of GenericParamInfo for each generic parameter found.
pub fn parse_generics(generics: &Generics) -> Vec<GenericParamInfo> {
    let mut params = Vec::new();

    for param in &generics.params {
        match param {
            GenericParam::Type(type_param) => {
                let name = type_param.ident.to_string();
                let bounds = parse_type_bounds(&type_param.bounds);
                params.push(GenericParamInfo {
                    name,
                    bounds,
                    lifetimes: Vec::new(),
                });
            }
            GenericParam::Lifetime(lifetime_param) => {
                let name = lifetime_param.lifetime.to_string();
                let bounds = parse_lifetime_bounds(&lifetime_param.bounds);
                params.push(GenericParamInfo {
                    name,
                    bounds,
                    lifetimes: vec![lifetime_param.lifetime.clone()],
                });
            }
            GenericParam::Const(_) => {
                // Skip const generics for now
            }
        }
    }

    params
}

/// Parse type bounds
fn parse_type_bounds(
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
) -> Vec<GenericBound> {
    bounds
        .iter()
        .filter_map(|bound| {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                let path = &trait_bound.path;
                Some(GenericBound::Trait(path_to_string(path)))
            } else {
                None
            }
        })
        .collect()
}

/// Parse lifetime bounds
fn parse_lifetime_bounds(
    bounds: &syn::punctuated::Punctuated<Lifetime, syn::token::Plus>,
) -> Vec<GenericBound> {
    bounds
        .iter()
        .map(|lt| GenericBound::Lifetime(lt.to_string()))
        .collect()
}

/// Convert syn::Path to string
fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Check if a type contains generic parameters.
///
/// Returns true if the type is a generic type like `Option<T>` or `Vec<T>`.
pub fn is_generic_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                !matches!(segment.arguments, PathArguments::None)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Extract the inner type from a generic wrapper.
///
/// For example, `Option<String>` returns `String`, `Vec<i32>` returns `i32`.
/// Returns `None` if the type is not a generic wrapper.
pub fn extract_inner_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty.clone());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Generate where clause TokenStream for generic constraints.
///
/// Returns the existing where clause or an empty TokenStream if none exists.
pub fn generate_where_clause(generics: &Generics) -> proc_macro2::TokenStream {
    if generics.where_clause.is_some() {
        let where_clause = &generics.where_clause;
        quote::quote! { #where_clause }
    } else {
        quote::quote! {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_parse_generics() {
        let generics: Generics = parse_str("<T: Clone + Send, U: Serialize>").unwrap();
        let params = parse_generics(&generics);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "T");
        assert_eq!(params[0].bounds.len(), 2);
    }

    #[test]
    fn test_is_generic_type() {
        let option_type: Type = parse_str("Option<String>").unwrap();
        assert!(is_generic_type(&option_type));

        let simple_type: Type = parse_str("String").unwrap();
        assert!(!is_generic_type(&simple_type));
    }

    #[test]
    fn test_extract_inner_type() {
        let option_type: Type = parse_str("Option<String>").unwrap();
        let inner = extract_inner_type(&option_type);
        assert!(inner.is_some());
        assert_eq!(inner.unwrap().to_token_stream().to_string(), "String");
    }
}
