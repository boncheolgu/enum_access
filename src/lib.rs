extern crate quote;
#[allow(unused_imports)]
#[macro_use]
extern crate syn;
extern crate syn_util;
#[macro_use]
extern crate synstructure;
extern crate proc_macro2;

use proc_macro2::{Span, TokenStream};
use syn::{AttrStyle, Attribute, Ident, Lit, Meta, NestedMeta, Type};
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

decl_derive!([EnumAccess, attributes(enum_alias, enum_ignore, enum_access)] => impl_enum_accessor);
decl_derive!([EnumDisplay, attributes(enum_display)] => impl_enum_display);

fn impl_enum_accessor(mut s: Structure) -> TokenStream {
    let name = &s.ast().ident;
    let (impl_generics, ty_generics, where_clause) = s.ast().generics.split_for_impl();

    s.binding_name(|bi, i| bi.ident.clone().unwrap_or_else(|| ident!("binding{}", i)));

    let mut s_mut = s.clone();
    s_mut.bind_with(|_| BindStyle::RefMut);

    let accessors = get_accessor_list(&s.ast().attrs);

    let body = accessors.iter().flat_map(|(kind, ident)| {
        let ty = ident_type(&s, ident);

        if kind == "get" {
            let body = impl_enum_get(&s, ident);
            let get = ident!("get_{}", ident);

            let body_mut = impl_enum_get(&s_mut, ident);
            let get_mut = ident!("get_mut_{}", ident);

            Some(quote!{
                #[allow(unused_variables, dead_code)]
                impl #impl_generics #name #ty_generics #where_clause {
                    fn #get (&self) -> &#ty {
                        match *self { #body }
                    }

                    fn #get_mut (&mut self) -> &mut #ty {
                        match *self { #body_mut }
                    }
                }
            })
        } else if kind == "get_some" {
            let body = impl_enum_get_some(&s, ident);
            let get = ident!("get_{}", ident);

            let body_mut = impl_enum_get_some(&s_mut, ident);
            let get_mut = ident!("get_mut_{}", ident);

            Some(quote!{
                #[allow(unused_variables, dead_code)]
                impl #impl_generics #name #ty_generics #where_clause {
                    fn #get (&self) -> Option<&#ty> {
                        match *self { #body }
                    }

                    fn #get_mut (&mut self) -> Option<&mut #ty> {
                        match *self { #body_mut }
                    }
                }
            })
        } else if kind == "iter" {
            let body = impl_enum_iter(&s, ident);
            let iter = ident!("iter_{}s", ident);

            let body_mut = impl_enum_iter(&s_mut, ident);
            let iter_mut = ident!("iter_mut_{}s", ident);

            Some(quote!{
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

    quote!( #(#body)* )
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
                                }).collect();
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

    &bi.binding == ident || get_attribute_list(&bi.ast().attrs)
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
                }).collect()
        }).collect();
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
        quote!{ #bi }
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
            0 => quote!{ None },
            1 => {
                let bi = &bindings[0];
                quote!{ Some(#bi) }
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

        quote!{ vec![#(#bindings,)*] }
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

#[cfg(test)]
mod test {
    use super::*;

    use syn::DeriveInput;

    #[test]
    fn it_works() {
        let s: DeriveInput = parse_quote!{
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
    }
}
