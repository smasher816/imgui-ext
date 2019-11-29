#![recursion_limit = "128"]
#![deny(warnings)]
extern crate proc_macro;

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma, Attribute, Data, DeriveInput, Fields, Ident, Variant};

use error::Error;

use crate::error::ErrorKind;

mod error;
mod parser;

#[proc_macro_derive(Gui, attributes(imgui))]
pub fn ui_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_derive(input: &DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let (body, catch_fields, catch_methods) = match input.data {
        Data::Struct(ref body) => struct_body(body.fields.clone()),
        Data::Enum(ref body) => enum_body(body.variants.clone()),
        _ => Err(Error::non_struct(input.span())),
    }?;

    // crate a new type.
    // It should never generate a collision
    let event_type = Ident::new(&format!("__{}_Events", name.to_string()), input.span());

    Ok(quote! {
        #[allow(non_camel_case_types)]
        pub struct #event_type {
            #catch_fields
        }
        impl #event_type {
            #catch_methods
        }
        impl #impl_generics imgui_ext::Gui for #name #ty_generics #where_clause {
            type Events = #event_type;
            fn draw_gui(ui: &imgui::Ui, ext: &mut Self) -> Self::Events {
                // Because all fields are bool, it should be OK to zero the memory (right...?)
                let mut events: Self::Events = unsafe { std::mem::zeroed() };
                #body
                events
            }
        }
    })
}

// Adds support to allow multiple imgui tags in a single field:
// ```
// struct Demo {
//     #[imgui(drag(...))]
//     x: f32,
//
//     // multiple annotations
//     #[imgui(separator)]
//     #[imgui(slider(...))]
//     #[imgui(input(...))]
//     y: f32,
// }
fn struct_body(fields: Fields) -> Result<(TokenStream, TokenStream, TokenStream), Error> {
    let mut input_methods: TokenStream = TokenStream::new();

    let mut input_fields: TokenStream = TokenStream::new();
    let mut input_fields_set = HashSet::new();

    let field_body = fields
        .iter()
        .enumerate()
        .flat_map(|(_, field)| {
            // TODO support for unnamed attributes
            let ident = field
                .ident
                .clone()
                .expect("Unnamed fields not yet supported.");
            let ty = &field.ty;

            // collect all the imgui attributes
            // we need to check that there is only one.
            let attrs: Vec<Attribute> = field
                .attrs
                .iter()
                .filter(|attr| {
                    let ident = Ident::new("imgui", attr.span());
                    attr.path.is_ident(&ident)
                })
                .cloned()
                .collect();

            let mut attrs = attrs.into_iter();
            let first = attrs.next();
            let second = attrs.next();

            match (first, second) {
                // No annotations were found.
                // Emmit no sourcecode.
                (None, None) => vec![Ok(TokenStream::new())],

                // There is more than one imgui annotation.
                // Raise a descriptive error pointing to the extra annotation.
                (Some(_), Some(err)) => vec![Err(Error::multiple(err.span()))],

                // There is a single annotation, as it should.
                // Parse the annotation and emmit the source code for this field
                (Some(attr), None) => {
                    let tags = attr
                        .parse_meta() // -> Meta
                        .map_err(|_| Error::new(ErrorKind::ParseError, attr.span()))
                        .and_then(parser::parse_meta); // -> Result<Vec<Tag>>

                    match tags {
                        Err(error) => vec![Err(error)],
                        Ok(tags) => tags
                            .into_iter()
                            .map(|tag| {
                                parser::emmit_tag_tokens(
                                    &ident,
                                    &ty,
                                    &attr,
                                    &tag,
                                    &mut input_fields,
                                    &mut input_methods,
                                    &mut input_fields_set,
                                )
                            })
                            .collect(),
                    }
                }

                _ => unreachable!(),
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok((quote! { #( #field_body );*}, input_fields, input_methods))
}

fn enum_body(variants: Punctuated<Variant, Comma>) -> Result<(TokenStream, TokenStream, TokenStream), Error> {
    let mut input_fields: TokenStream = TokenStream::new();
    let mut input_methods: TokenStream = TokenStream::new();
    let mut input_fields_set = HashSet::new();


    let field_body = variants
        .iter()
        .enumerate()
        .flat_map(|(_, variant)| {
            let ident = &variant.ident;

            let field = variant.fields.iter().next().expect("No field");
            let ty = &field.ty;

            //let attr = variant.attrs.get(0).expect("No attr");
            //let tag = parser::Tag::None;

            // collect all the imgui attributes
            // we need to check that there is only one.
            let attrs: Vec<Attribute> = variant
                .attrs
                .iter()
                .filter(|attr| {
                    let ident = Ident::new("imgui", attr.span());
                    attr.path.is_ident(&ident)
                })
                .cloned()
                .collect();

            let mut attrs = attrs.into_iter();
            let first = attrs.next();
            let second = attrs.next();

            match (first, second) {
                // No annotations were found.
                // Emmit no sourcecode.
                (None, None) => vec![Ok(TokenStream::new())],

                // There is more than one imgui annotation.
                // Raise a descriptive error pointing to the extra annotation.
                (Some(_), Some(err)) => vec![Err(Error::multiple(err.span()))],

                // There is a single annotation, as it should.
                // Parse the annotation and emmit the source code for this field
                (Some(attr), None) => {
                    let tags = attr
                        .parse_meta() // -> Meta
                        .map_err(|_| Error::new(ErrorKind::ParseError, attr.span()))
                        .and_then(parser::parse_meta); // -> Result<Vec<Tag>>

                    match tags {
                        Err(error) => vec![Err(error)],
                        Ok(tags) => tags
                            .into_iter()
                            .map(|tag| {
                                parser::emmit_tag_tokens(
                                    ident,
                                    ty,
                                    &attr,
                                    &tag,
                                    &mut input_fields,
                                    &mut input_methods,
                                    &mut input_fields_set,
                                )
                            })
                            .collect(),
                    }
                }

                _ => unreachable!(),
            }
    })
    .collect::<Result<Vec<_>, Error>>()?;

    Ok((quote! { #( #field_body );*}, input_fields, input_methods))
}
