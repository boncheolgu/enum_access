#![recursion_limit = "128"]

extern crate quote;
#[allow(unused_imports)]
#[macro_use]
extern crate syn;
extern crate syn_util;
#[macro_use]
extern crate synstructure;
extern crate proc_macro2;

use proc_macro2::{Span, TokenStream};
use syn::{
    AttrStyle, Attribute, Field, Fields, GenericParam, Ident, Lifetime, Lit, Meta, NestedMeta,
    Type, TypeParam, VisPublic, Visibility,
};
use syn_util::contains_attribute;
use synstructure::{BindStyle, BindingInfo, Structure};

macro_rules! ident {
    ($id:expr) => {
        Ident::new($id, Span::call_site())
    };
    ($fmt:expr, $($args:tt)+) => {
        Ident::new(&format!($fmt, $($args)*), Span::call_site())
    };
}

decl_derive!([EnumAccess, attributes(enum_alias, enum_ignore, enum_access, enum_inner_struct)] => impl_enum_accessor);
decl_derive!([EnumDisplay, attributes(enum_display)] => impl_enum_display);

fn impl_enum_accessor(mut s: Structure) -> TokenStream {
    let name = &s.ast().ident;
    let (impl_generics, ty_generics, where_clause) = s.ast().generics.split_for_impl();

    s.binding_name(|bi, i| bi.ident.clone().unwrap_or_else(|| ident!("binding{}", i)));

    let mut s_mut = s.clone();
    s_mut.bind_with(|_| BindStyle::RefMut);

    let inner_body = impl_enum_inner_struct(&s);

    let accessors = get_accessor_list(&s.ast().attrs);

    let accessor_body = accessors.iter().flat_map(|(kind, ident)| {
        let ty = ident_type(&s, ident);

        if kind == "get" {
            let body = impl_enum_get(&s, ident);
            let get = ident;

            let body_mut = impl_enum_get(&s_mut, ident);
            let get_mut = ident!("{}_mut", ident);

            Some(quote! {
                #[allow(unused_variables, dead_code)]
                impl #impl_generics #name #ty_generics #where_clause {
                    fn #get (&self) -> &#ty {
                        match self { #body }
                    }

                    fn #get_mut (&mut self) -> &mut #ty {
                        match self { #body_mut }
                    }
                }
            })
        } else if kind == "get_some" {
            let body = impl_enum_get_some(&s, ident);
            let get = ident;

            let body_mut = impl_enum_get_some(&s_mut, ident);
            let get_mut = ident!("{}_mut", ident);

            Some(quote! {
                #[allow(unused_variables, dead_code)]
                impl #impl_generics #name #ty_generics #where_clause {
                    fn #get (&self) -> Option<&#ty> {
                        match self { #body }
                    }

                    fn #get_mut (&mut self) -> Option<&mut #ty> {
                        match self { #body_mut }
                    }
                }
            })
        } else if kind == "iter" {
            let body = impl_enum_iter(&s, ident);
            let iter = ident;

            let body_mut = impl_enum_iter(&s_mut, ident);
            let iter_mut = ident!("{}_mut", ident);

            Some(quote! {
                #[allow(unused_variables, dead_code)]
                impl #impl_generics #name #ty_generics #where_clause {
                    fn #iter (&self) -> Vec<&#ty> {
                        match *self { #body }
                    }

                    fn #iter_mut (&mut self) -> Vec<&mut #ty> {
                        match *self { #body_mut }
                    }
                }
            })
        } else {
            unreachable!("unspecified attribute given: {}.", kind);
        }
    });

    quote!( #(#accessor_body)* #inner_body )
}

fn impl_enum_display(mut s: Structure) -> TokenStream {
    s.binding_name(|bi, i| bi.ident.clone().unwrap_or_else(|| ident!("binding{}", i)));

    let body = s.each_variant(|v| {
        for attr in v.ast().attrs {
            if attr.style == AttrStyle::Outer {
                if let Some(meta) = attr.interpret_meta() {
                    if meta.name() == "enum_display" {
                        if let Meta::List(meta_list) = meta {
                            let meta_list: Vec<_> = meta_list
                                .nested
                                .iter()
                                .map(|x| {
                                    if let NestedMeta::Literal(Lit::Int(lit_int)) = x {
                                        let bi = ident!("binding{}", lit_int.value());
                                        quote!(#bi)
                                    } else {
                                        quote!(#x)
                                    }
                                })
                                .collect();
                            return quote!(write!(f, #(#meta_list),*));
                        }
                    }
                }
            }
        }

        quote!(write!(f, ""))
    });

    s.gen_impl(quote! {
        use std::fmt::{Display, Error, Formatter};
        use std::result::Result;

        gen impl Display for @Self {
            #[allow(unused_variables)]
            fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
                match *self { #body }
            }
        }
    })
}

fn ident_of(bi: &BindingInfo, ident: &Ident) -> bool {
    if contains_attribute(&bi.ast().attrs, &["enum_ignore"]) {
        return false;
    }

    &bi.binding == ident
        || get_attribute_list(&bi.ast().attrs)
            .iter()
            .any(|(k, v)| k == "enum_alias" && v == ident)
}

fn ident_type<'a>(s: &'a Structure, ident: &Ident) -> &'a Type {
    let bindings: Vec<Vec<_>> = s
        .variants()
        .iter()
        .map(|v| {
            v.bindings()
                .iter()
                .filter_map(|bi| {
                    if ident_of(bi, ident) {
                        Some(&bi.ast().ty)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    let mut bindings = bindings.concat();
    bindings.dedup();
    assert!(
        bindings.len() == 1,
        "\'{}\' fields should have a single type; [{}].",
        ident,
        quote!(#(#bindings),*)
    );
    bindings.remove(0)
}

fn impl_enum_get(s: &Structure, ident: &Ident) -> TokenStream {
    s.each_variant(|v| {
        let bindings: Vec<_> = v
            .bindings()
            .iter()
            .filter(|bi| ident_of(bi, ident))
            .collect();

        assert!(
            !bindings.is_empty(),
            "\'{}\' has no field named \"{}\".",
            v.ast().ident,
            ident
        );

        let bi = &bindings[0];
        quote! { #bi }
    })
}

fn impl_enum_get_some(s: &Structure, ident: &Ident) -> TokenStream {
    s.each_variant(|v| {
        let bindings: Vec<_> = v
            .bindings()
            .iter()
            .filter(|bi| ident_of(bi, ident))
            .collect();

        match bindings.len() {
            0 => quote! { None },
            1 => {
                let bi = &bindings[0];
                quote! { Some(#bi) }
            }
            _ => {
                panic!(
                    "\'{}\' should have at most one field named \"{}\".",
                    v.ast().ident,
                    ident
                );
            }
        }
    })
}

fn impl_enum_iter(s: &Structure, ident: &Ident) -> TokenStream {
    s.each_variant(|v| {
        let bindings: Vec<_> = v
            .bindings()
            .iter()
            .filter(|bi| ident_of(bi, ident))
            .collect();

        quote! { vec![#(#bindings,)*] }
    })
}

fn get_attribute_list(attrs: &[Attribute]) -> Vec<(Ident, Ident)> {
    let mut result = Vec::new();

    for attr in attrs {
        if attr.style != AttrStyle::Outer {
            continue;
        }

        if let Some(meta) = attr.interpret_meta() {
            if let Meta::List(meta_list) = meta {
                for meta in &meta_list.nested {
                    match *meta {
                        NestedMeta::Meta(Meta::Word(ref ident)) => {
                            result.push((meta_list.ident.clone(), ident.clone()));
                        }
                        _ => continue,
                    }
                }
            }
        }
    }

    result
}

fn get_accessor_list(attrs: &[Attribute]) -> Vec<(Ident, Ident)> {
    let mut result = Vec::new();

    for attr in attrs {
        if attr.style != AttrStyle::Outer {
            continue;
        }

        if let Some(meta) = attr.interpret_meta() {
            if meta.name() != "enum_access" {
                continue;
            }

            if let Meta::List(meta_list) = meta {
                for meta in &meta_list.nested {
                    match meta {
                        NestedMeta::Meta(Meta::List(meta_list)) => {
                            for meta in &meta_list.nested {
                                match *meta {
                                    NestedMeta::Meta(Meta::Word(ref ident)) => {
                                        result.push((meta_list.ident.clone(), ident.clone()));
                                    }
                                    _ => continue,
                                }
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }
    }

    result
}

fn contains_type_generics(ty: &Type, type_param: &TypeParam) -> bool {
    match ty {
        Type::Slice(type_slice) => contains_type_generics(&*type_slice.elem, type_param),
        Type::Array(type_array) => contains_type_generics(&*type_array.elem, type_param),
        Type::Ptr(type_ptr) => contains_type_generics(&*type_ptr.elem, type_param),
        Type::Reference(type_reference) => {
            contains_type_generics(&*type_reference.elem, type_param)
        }
        Type::Tuple(type_tuple) => type_tuple
            .elems
            .iter()
            .any(|ty| contains_type_generics(ty, type_param)),
        Type::Path(type_path) => {
            let type_ident = &type_param.ident;
            quote!(#type_path).to_string() == quote!(#type_ident).to_string()
        }
        _ => false,
    }
}

fn contains_lifetime_generics(ty: &Type, lifetime: &Lifetime) -> bool {
    match ty {
        Type::Slice(type_slice) => contains_lifetime_generics(&*type_slice.elem, lifetime),
        Type::Array(type_array) => contains_lifetime_generics(&*type_array.elem, lifetime),
        Type::Tuple(type_tuple) => type_tuple
            .elems
            .iter()
            .any(|ty| contains_lifetime_generics(ty, lifetime)),
        Type::Reference(type_reference) => type_reference.lifetime.as_ref() == Some(lifetime),
        _ => false,
    }
}

fn impl_enum_inner_struct(s: &Structure) -> TokenStream {
    let inners = s
        .variants()
        .iter()
        .filter(|v| contains_attribute(v.ast().attrs, &["enum_inner_struct"]))
        .map(|v| {
            let name = &s.ast().ident;
            let variant_name = &v.ast().ident;
            let inner_name = ident!("{}{}Inner", name, variant_name);
            let mut fields = v.ast().fields.clone();

            fn clear_field(field: &mut Field) {
                field.vis = Visibility::Public(
                    VisPublic {
                        pub_token: Token!(pub)(Span::call_site())
                    }
                );
                field.attrs.clear();
            }

            match &mut fields {
                Fields::Named(fields_named) => fields_named.named.iter_mut().for_each(|field| {
                    clear_field(field);
                }),
                Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.iter_mut().for_each(|field| {
                    clear_field(field);
                }),
                _ => {}
            }

            let bindings = v.bindings();

            let (impl_generics, ty_generics, where_clause) = s.ast().generics.split_for_impl();
            let inner_generics: Vec<_> = s.ast().generics.params.iter().filter(|param| {
                bindings.iter().any(|bi| {
                    match param {
                        GenericParam::Type(type_param) => {
                            contains_type_generics(&bi.ast().ty, type_param)
                        }
                        GenericParam::Lifetime(lifetime_def) => {
                            contains_lifetime_generics(&bi.ast().ty, &lifetime_def.lifetime)
                        }
                        _ => false,
                    }
                })
            }).collect();

            let inner_ty_generics: Vec<_> = inner_generics.iter().filter_map(|param| {
                match param {
                    GenericParam::Type(type_param) => {
                        let type_ident = &type_param.ident;
                        Some(quote!(#type_ident))
                    },
                    GenericParam::Lifetime(lifetime_def) => {
                        let lifetime = &lifetime_def.lifetime;
                        Some(quote!(#lifetime))
                    },
                    _ => None,
                }
            }).collect();
            let inner_ty_generics = quote!( < #(#inner_ty_generics),* > );

            let inner_impl_generics = quote!( < #(#inner_generics),* > );

            if let Fields::Named(_) = v.ast().fields {
                quote! {
                    pub struct #inner_name #inner_impl_generics #where_clause #fields

                    impl #impl_generics From<#name #ty_generics> for #inner_name #inner_ty_generics #where_clause {
                        fn from(x: #name #ty_generics) -> Self {
                            match x {
                                #name::#variant_name{#(#bindings),*} => #inner_name{#(#bindings),*},
                                _ => panic!("cannot converted to {}.", stringify!(#inner_name)),
                            }
                        }
                    }

                    impl #impl_generics From<#inner_name #inner_ty_generics> for #name #ty_generics #where_clause {
                        fn from(x: #inner_name #inner_ty_generics) -> Self {
                            let #inner_name{#(#bindings),*} = x;
                            #name :: #variant_name {#(#bindings),*}
                        }
                    }
                }
            } else {
                quote! {
                    pub struct #inner_name #inner_impl_generics #where_clause #fields ;

                    impl #impl_generics From<#name #ty_generics> for #inner_name #inner_ty_generics #where_clause {
                        fn from(x: #name #ty_generics) -> Self {
                            match x {
                                #name::#variant_name(#(#bindings),*) => #inner_name(#(#bindings),*),
                                _ => panic!("cannot converted to {}.", stringify!(#inner_name)),
                            }
                        }
                    }

                    impl #impl_generics From<#inner_name #inner_ty_generics> for #name #ty_generics #where_clause {
                        fn from(x: #inner_name #inner_ty_generics) -> Self {
                            let #inner_name(#(#bindings),*) = x;
                            #name :: #variant_name (#(#bindings),*)
                        }
                    }
                }
            }
        });
    quote!(#(#inners)*)
}

#[cfg(test)]
mod test {
    use super::*;

    use syn::DeriveInput;

    #[test]
    fn unittest_enum_access() {
        let s: DeriveInput = parse_quote! {
            #[enum_access(get(name, address), get_some(index), iter(input))]
            enum A {
            }
        };
        assert_eq!(
            get_accessor_list(&s.attrs),
            vec![
                (ident!("get"), ident!("name")),
                (ident!("get"), ident!("address")),
                (ident!("get_some"), ident!("index")),
                (ident!("iter"), ident!("input")),
            ]
        );

        test_derive! {
            impl_enum_display {
                enum A<T> {
                    #[enum_display("B: {}, {}", 0, 1)]
                    B(i32, T),
                    C(i32),
                    #[enum_display("D: {}, {}", value, key)]
                    D{key: i32, value: i32}
                }
            }
            expands to {
                #[allow(non_upper_case_globals)]
                const _DERIVE_Display_FOR_A: () = {
                    use std::fmt::{Display, Error, Formatter};
                    use std::result::Result;

                    impl<T> Display for A<T> where T : Display {
                        #[allow(unused_variables)]
                        fn fmt(&self, f: & mut Formatter) -> Result<(), Error> {
                            match *self {
                                A::B(ref binding0, ref binding1,) => {
                                    write!(f, "B: {}, {}", binding0, binding1)
                                }
                                A::C(ref binding0,) => {
                                    write!(f, "")
                                }
                                A::D{key: ref key, value: ref value,} => {
                                    write!(f, "D: {}, {}", value, key)
                                }
                            }
                        }
                    }
                };
            }
            no_build
        }

        test_derive! {
            impl_enum_accessor {
                enum Enum<'a, T: Clone> {
                    #[enum_inner_struct]
                    Variant1(i32, T),
                    #[enum_inner_struct]
                    Variant2{key: &'a i32, value: i32},
                    #[enum_inner_struct]
                    Variant3(&'a T),
                }
            }
            expands to {
                pub struct EnumVariant1Inner<T: Clone> ( pub i32, pub T );
                impl<'a, T: Clone> From<Enum<'a, T> > for EnumVariant1Inner<T> {
                    fn from(x: Enum<'a, T>) -> Self {
                        match x {
                            Enum::Variant1(binding0, binding1) => EnumVariant1Inner(binding0, binding1),
                            _ => panic!("cannot converted to {}.", stringify!(EnumVariant1Inner)),
                        }
                    }
                }
                impl<'a, T: Clone> From<EnumVariant1Inner<T> > for Enum<'a, T> {
                    fn from(x: EnumVariant1Inner<T>) -> Self {
                        let EnumVariant1Inner(binding0, binding1) = x;
                        Enum::Variant1(binding0, binding1)
                    }
                }

                pub struct EnumVariant2Inner<'a> { pub key: &'a i32, pub value: i32 }
                impl<'a, T: Clone> From<Enum<'a, T> > for EnumVariant2Inner<'a> {
                    fn from(x: Enum<'a, T>) -> Self {
                        match x {
                            Enum::Variant2 { key, value } => EnumVariant2Inner { key, value },
                            _ => panic!("cannot converted to {}.", stringify!(EnumVariant2Inner)),
                        }
                    }
                }
                impl<'a, T: Clone> From<EnumVariant2Inner<'a> > for Enum<'a, T> {
                    fn from(x: EnumVariant2Inner<'a>) -> Self {
                        let EnumVariant2Inner { key, value } = x;
                        Enum::Variant2 { key, value }
                    }
                }

                pub struct EnumVariant3Inner<'a, T: Clone> ( pub &'a T );
                impl<'a, T: Clone> From<Enum<'a, T> > for EnumVariant3Inner<'a, T> {
                    fn from(x: Enum<'a, T>) -> Self {
                        match x {
                            Enum::Variant3(binding0) => EnumVariant3Inner(binding0),
                            _ => panic!("cannot converted to {}.", stringify!(EnumVariant3Inner)),
                        }
                    }
                }
                impl<'a, T: Clone> From<EnumVariant3Inner<'a, T> > for Enum<'a, T> {
                    fn from(x: EnumVariant3Inner<'a, T>) -> Self {
                        let EnumVariant3Inner(binding0) = x;
                        Enum::Variant3(binding0)
                    }
                }
            }
            no_build
        }
    }
}
