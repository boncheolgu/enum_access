extern crate quote;
extern crate syn;
#[macro_use]
extern crate synstructure;
extern crate proc_macro2;

use proc_macro2::TokenStream;
use syn::{AttrStyle, Attribute, Ident, Meta, NestedMeta, Type};
use synstructure::{BindStyle, BindingInfo, Structure};

decl_derive!([EnumAccess, attributes(enum_get, enum_get_some, enum_iter, enum_alias, enum_ignore)]
             => impl_enum_accessor);
fn impl_enum_accessor(mut s: Structure) -> TokenStream {
    s.binding_name(|bi, i| {
        bi.ident
            .clone()
            .unwrap_or_else(|| Ident::new(&format!("binding{}", i), proc_macro2::Span::call_site()))
    });

    let mut s_mut = s.clone();
    s_mut.bind_with(|_| BindStyle::RefMut);

    let accessors = get_attribute_list(&s.ast().attrs);

    let body = accessors.iter().flat_map(|(kind, ident)| {
        let sident = &s.ast().ident;
        let ty = ident_type(&s, ident);

        if kind == "enum_get" {
            let body = impl_enum_get(&s, ident);
            let get = Ident::new(&format!("get_{}", ident), proc_macro2::Span::call_site());

            let body_mut = impl_enum_get(&s_mut, ident);
            let get_mut = Ident::new(
                &format!("get_mut_{}", ident),
                proc_macro2::Span::call_site(),
            );

            Some(quote!{
                #[allow(unused_variables, dead_code)]
                impl #sident {
                    pub fn #get (&self) -> &#ty {
                        match *self { #body }
                    }

                    pub fn #get_mut (&mut self) -> &mut #ty {
                        match *self { #body_mut }
                    }
                }
            })
        } else if kind == "enum_get_some" {
            let body = impl_enum_get_some(&s, ident);
            let get = Ident::new(&format!("get_{}", ident), proc_macro2::Span::call_site());

            let body_mut = impl_enum_get_some(&s_mut, ident);
            let get_mut = Ident::new(
                &format!("get_mut_{}", ident),
                proc_macro2::Span::call_site(),
            );

            Some(quote!{
                #[allow(unused_variables, dead_code)]
                impl #sident {
                    pub fn #get (&self) -> Option<&#ty> {
                        match *self { #body }
                    }

                    pub fn #get_mut (&mut self) -> Option<&mut #ty> {
                        match *self { #body_mut }
                    }
                }
            })
        } else if kind == "enum_iter" {
            let body = impl_enum_iter(&s, ident);
            let iter = Ident::new(&format!("iter_{}s", ident), proc_macro2::Span::call_site());

            let body_mut = impl_enum_iter(&s_mut, ident);
            let iter_mut = Ident::new(
                &format!("iter_mut_{}s", ident),
                proc_macro2::Span::call_site(),
            );

            Some(quote!{
                #[allow(unused_variables, dead_code)]
                impl #sident {
                    pub fn #iter (&self) -> Vec<&#ty> {
                        match *self { #body }
                    }

                    pub fn #iter_mut (&mut self) -> Vec<&mut #ty> {
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

fn ident_of(bi: &BindingInfo, ident: &Ident) -> bool {
    if contains_word(&bi.ast().attrs, "enum_ignore") {
        return false;
    }

    let attrs = get_attribute_list(&bi.ast().attrs);
    &bi.binding == ident || { attrs.iter().any(|(k, v)| k == "enum_alias" && v == ident) }
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
            bindings.len() > 0,
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

fn contains_word(attrs: &[Attribute], word: &str) -> bool {
    for attr in attrs {
        if attr.style == AttrStyle::Outer {
            if let Some(meta) = attr.interpret_meta() {
                match meta {
                    Meta::Word(ref id) if id == word => return true,
                    _ => continue,
                }
            }
        }
    }
    return false;
}

fn get_attribute_list(attrs: &[Attribute]) -> Vec<(Ident, Ident)> {
    let mut result = Vec::new();

    for attr in attrs {
        if attr.style != AttrStyle::Outer {
            continue;
        }

        if let Some(meta) = attr.interpret_meta() {
            match meta {
                Meta::List(meta_list) => {
                    for meta in &meta_list.nested {
                        match *meta {
                            NestedMeta::Meta(Meta::Word(ref ident)) => {
                                result.push((meta_list.ident.clone(), ident.clone()));
                            }
                            _ => continue,
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}
