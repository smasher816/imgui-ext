extern crate proc_macro;

use std::string::ToString;

use proc_macro2::{Literal, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DeriveInput, Field, Fields, Ident, Lit, Meta, MetaNameValue, NestedMeta, parse_macro_input, Token};
use syn::parse::{Error, Parse};
use syn::punctuated::Pair;
use syn::spanned::Spanned;

const INVALID_ATTR_FORMAT: &str = "Invalid attribute format";
const INVALID_IDENT: &str = "Invalid identifier token";
const UNSUPPORTED_META: &str = "Unsupported metadata";

enum ImGuiAttr {
    // - `#[imgui]`
    // - `#[imgui(label = "...")]`
    Simple {
        label: Option<String>,
    },

    // `#[imgui(input)]`
    // `#[imgui(input( ... )]`
    Input {
        label: Option<String>,
        precission: Option<i32>,
        step: Option<f32>,
        step_fast: Option<f32>,
    },

    // `#[imgui(slider( ... ))]`
    Slider {
        label: Option<String>,
        display: Option<String>,
        min: f32,
        max: f32,
    },

    // `#[imgui(drag( ... ))]`
    Drag {
        label: Option<String>,
        display: Option<String>,
        min: Option<f32>,
        max: Option<f32>,
        speed: Option<f32>,
        power: Option<f32>,
    }
}

impl ImGuiAttr {
    fn from_meta(meta: &Meta) -> Result<Self, Error> {
        unimplemented!()
    }

    fn into_token_stream(self, ident: &Ident) -> Result<TokenStream, Error> {
        match self {
            ImGuiAttr::Simple { label } => {
                let label = Literal::string(&label.unwrap_or(ident.to_string()));

                Ok(quote! {{
                    imgui_ext_traits::Simple::build(ui, &mut ext.#ident, imgui_ext_traits::SimpleParams {
                        label: imgui::im_str!( #label ),
                    })
                }})
            },
            ImGuiAttr::Input { label, precission, step, step_fast } => {
                let label = Literal::string(&label.unwrap_or(ident.to_string()));
                let precission = precission.map(Literal::i32_suffixed);
                let step = step.map(Literal::f32_suffixed);
                let step_fast = step_fast.map(Literal::f32_suffixed);
                let mut fields = TokenStream::new();

                fields.extend(quote! { label: im_str!( #label ), });

                if let Some(val) = precission { fields.extend(quote! { precission: Some( #val ), }) }
                else { fields.extend(quote! { precission: None, }) }

                if let Some(val) = step { fields.extend(quote! { step: Some( #val ), }) }
                else { fields.extend(quote! { step: None, }) }

                if let Some(val) = step_fast { fields.extend(quote! { step_fast: Some( #val ), }) }
                else { fields.extend(quote! { step_fast: None, }) }

                Ok(quote! {
                    imgui_ext_traits::Input::build(ui, &mut ext.#ident, imgui_ext_traits::InputParams {
                        #fields
                    })
                })
            }
            ImGuiAttr::Slider { label, display, min, max } => {
                let label = Literal::string(&label.unwrap_or(ident.to_string()));
                let minlit = Literal::f32_suffixed(min);
                let maxlit = Literal::f32_suffixed(max);

                let mut fields = quote! {
                    min: #minlit,
                    max: #maxlit,
                    label: im_str!( #label ),
                };

                if let Some(disp) = display.map(|s| Literal::string(s.as_str())) {
                    fields.extend(quote! { display: Some(im_str!(#disp)), });
                } else {
                    fields.extend(quote! { display: None, });
                }

                Ok(quote! {
                    imgui_ext_traits::Slider::build(ui, &mut ext.#ident, imgui_ext_traits::SliderParams {
                        #fields
                    })
                })
            },
            ImGuiAttr::Drag { label, display, min, max, power, speed } => {
                let label = Literal::string(&label.unwrap_or(ident.to_string()));
                let mut fields = quote! { label: im_str!(#label), };

                if let Some(val) = display { fields.extend(quote! { display: Some(im_str!(#val)), }); }
                else { fields.extend(quote! { display: None, }) }

                if let Some(val) = min { fields.extend(quote! { min: Some(#val), }); }
                else { fields.extend(quote! { min: None, }) }

                if let Some(val) = max { fields.extend(quote! { max: Some(#val), }); }
                else { fields.extend(quote! { max: None, }) }

                if let Some(val) = power { fields.extend(quote! { power: Some(#val), }); }
                else { fields.extend(quote! { power: None, }) }

                if let Some(val) = speed { fields.extend(quote! { speed: Some(#val), }); }
                else { fields.extend(quote! { speed: None, }) }

                Ok(quote! {
                    imgui_ext_traits::Drag::build(ui, &mut ext.#ident, imgui_ext_traits::DragParams {
                        #fields
                    })
                })
            }
            _ => unimplemented!(),
        }
    }
}

#[proc_macro_derive(ImGuiExt, attributes(imgui))]
pub fn imgui_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_derive(input: &DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match input.data {
        Data::Struct(ref body) => imgui_body_fields(body.fields.clone()),
        _ => Err(Error::new(input.span(), "Only structs"))
    }?;

    Ok(quote! {
        impl #impl_generics imgui_ext_traits::ImGuiExt for #name #ty_generics #where_clause {
            fn imgui_ext(ui: &imgui::Ui, ext: &mut Self) {
                #body
            }
        }
    })
}

fn parse_input(meta_list: &syn::MetaList) -> Result<ImGuiAttr, Error> {
    let mut step: Option<f32> = None;
    let mut step_fast: Option<f32> = None;
    let mut precission: Option<i32> = None;
    let mut label = None;

    for item in meta_list.nested.iter() {
        match item {
            NestedMeta::Literal(l) => return Err(Error::new(meta_list.span(), "Unrecognized attribute literal")),
            NestedMeta::Meta(meta) => match meta {
                Meta::NameValue(MetaNameValue { ident, lit: Lit::Int(lit), .. }) => match ident.to_string().as_str() {
                    "precission" => {
                        if precission.is_some() { return Err(Error::new(ident.span(), "`precission` attribute already set.")) }
                        else { precission = Some(lit.value() as i32) }
                    },
                    _ => return Err(Error::new(ident.span(), INVALID_IDENT)),
                },
                Meta::NameValue(MetaNameValue { ident, lit: Lit::Float(lit), .. }) => match ident.to_string().as_str() {
                    "step" => {
                        if step.is_some() { return Err(Error::new(ident.span(), "`step` attribute already set.")) }
                        else { step = Some(lit.value() as f32) }
                    },
                    "step_fast" => {
                        if step_fast.is_some() { return Err(Error::new(ident.span(), "`step_fast` attribute already set.")) }
                        else { step_fast = Some(lit.value() as f32) }
                    },
                    _ => return Err(Error::new(ident.span(), INVALID_IDENT)),
                },
                Meta::NameValue(MetaNameValue { ident, lit: Lit::Str(lit), .. }) => match ident.to_string().as_str() {
                    "label" => {
                        if label.is_some() { return Err(Error::new(ident.span(), "`label` attribute already set.")) }
                        else { label = Some(lit.value()) }
                    },
                    _ => return Err(Error::new(ident.span(), INVALID_IDENT)),
                }
                _ => return Err(Error::new(meta_list.span(), "Unrecognized attribute 2"))
            }
        }
    }

    Ok(ImGuiAttr::Input {
        step,
        step_fast,
        label,
        precission,
    })
}

fn parse_slider(meta_list: &syn::MetaList) -> Result<ImGuiAttr, Error> {
    let mut min: Option<f32> = None;
    let mut max: Option<f32> = None;
    let mut label = None;
    let mut display = None;

    for item in meta_list.nested.iter() {
        match item {
            NestedMeta::Literal(l) => return Err(Error::new(meta_list.span(), "Unrecognized attribute literal")),
            NestedMeta::Meta(meta) => match meta {
                Meta::NameValue(MetaNameValue { ident, lit: Lit::Float(lit), .. }) => match ident.to_string().as_str() {
                    "min" => {
                        if min.is_some() { return Err(Error::new(ident.span(), "`min` attribute already set.")) }
                        else { min = Some(lit.value() as f32) }
                    },
                    "max" => {
                        if max.is_some() { return Err(Error::new(ident.span(), "`max` attribute already set.")) }
                        else { max = Some(lit.value() as f32) }
                    },
                    _ => return Err(Error::new(ident.span(), INVALID_IDENT)),
                },
                Meta::NameValue(MetaNameValue { ident, lit: Lit::Str(lit), .. }) => match ident.to_string().as_str() {
                    "label" => {
                        if label.is_some() { return Err(Error::new(ident.span(), "`label` attribute already set.")) }
                        else { label = Some(lit.value()) }
                    },
                    "display" => {
                        if display.is_some() { return Err(Error::new(ident.span(), "`display` attribute already set.")) }
                        else { display = Some(lit.value()) }
                    },
                    _ => return Err(Error::new(ident.span(), INVALID_IDENT)),
                }
                _ => return Err(Error::new(meta_list.span(), "Unrecognized attribute 2"))
            }
        }
    }

    Ok(ImGuiAttr::Slider {
        min: min.ok_or(Error::new(meta_list.span(), "Attribute `min` missing."))?,
        max: max.ok_or(Error::new(meta_list.span(), "Attribute `max` missing."))?,
        label,
        display,
    })
}

// Parse the tokens between the parenthesis of a MetaList, that is, what
// is inside the parenthesis of this annotation:
//
//  - #[imgui( ... )]
//            ^^^^^
fn parse_meta_list(name: &Ident, meta: &syn::MetaList) -> Result<ImGuiAttr, Error> {
    // Allow only one level of nested depth
    let nested = &meta.nested;
    if nested.len() != 1 {
        return Err(Error::new(meta.span(), INVALID_ATTR_FORMAT));
    }

    match nested.first() {
        // TODO
        // Do we want to support both:
        // - `#[imgui( foo )]` and
        // - `#[imgui( foo, )]` (with trailing comma)
        // or just the first one?
        Some(Pair::End(attr)) | Some(Pair::Punctuated(attr, _)) => {
            match attr {
                // This is not allowed (literal inside of the annotation)
                //  - `#[imgui("...")]`
                NestedMeta::Literal(lit) => {
                    Err(Error::new(meta.span(), INVALID_ATTR_FORMAT))
                    /*
                    Ok(ImGuiAttr::Input {
                        label: None,
                        precission: None,
                        step: None,
                        step_fast: None
                    })
                    */
                },

                NestedMeta::Meta(meta) => {
                    match meta {
                        // We should have
                        //  - `#[imgui(label = "...")]`
                        Meta::NameValue(MetaNameValue { ident, lit: Lit::Str(label), .. }) => {
                            if ident.to_string() == "label" {
                                Ok(ImGuiAttr::Simple {
                                    label: Some(label.value()),
                                })
                            } else {
                                Err(Error::new(ident.span(), INVALID_IDENT))
                            }
                        },

                        // Check things like:
                        //  - `#[imgui(input( ... ))]`
                        //  - `#[imgui(progress( ... ))]`
                        //  - `#[imgui(slider( ... ))]`
                        Meta::List(meta_list) => match meta_list.ident.to_string().as_str() {
                            "input" => parse_input(meta_list),
                            "slider" => parse_slider(meta_list),
                            "drag" => unimplemented!("drag"),
                            _ => Err(Error::new(meta_list.span(), UNSUPPORTED_META)),
                        },

                        // Special cases like:
                        //  - `#[input(text)]`
                        //  - `#[input(drag)]`
                        Meta::Word(ident) => match ident.to_string().as_str() {
                            "input" => Ok(ImGuiAttr::Input {
                                label: None,
                                precission: None,
                                step: None,
                                step_fast: None
                            }),
                            "drag" => Ok(ImGuiAttr::Drag {
                                label: None,
                                display: None,
                                min: None,
                                max: None,
                                speed: None,
                                power: None
                            }),
                            _ => Err(Error::new(name.span(), INVALID_ATTR_FORMAT)),
                        }

                        _ => Err(Error::new(name.span(), INVALID_ATTR_FORMAT)),
                    }
                }
            }
        },
        _ => {
            // FIXME
            Err(Error::new(meta.span(), INVALID_ATTR_FORMAT))
        }
    }
}

// #[imgui( ... )]
//   ^^^^^^^^^^^^
fn parse_meta(name: &Ident, meta: &Meta) -> Result<ImGuiAttr, Error> {
    use syn::MetaList;

    match meta {
        // At this point we know we have this:
        // #[imgui]
        &Meta::Word(_) => {
            Ok(ImGuiAttr::Simple { label: None })
        },

        // #[imgui( meta_list )]
        //
        // We might have (but not be limited to):
        //  - #[imgui(display = "...")]
        //  - #[imgui(input( ... ))]
        //  - #[imgui(progress( ... ))]
        //  - #[imgui(slider( ... ))]
        &Meta::List(ref meta_list) => parse_meta_list(name, meta_list),

        // This type of attribute is not allowed
        //  - #[imgui = "..."]
        &Meta::NameValue(_) => {
            Err(Error::new(meta.span(), INVALID_ATTR_FORMAT))
        },
    }
}

fn imgui_body_fields(fields: Fields) -> Result<TokenStream, Error> {
    let field_assign = fields.iter().map(|field| {

        // collect all #[imgui] attributes
        let mut attributes = field.attrs.iter()
            .filter(is_imgui_attr)
            .map(Attribute::parse_meta)
            .collect::<Result<Vec<_>, Error>>()?;

        // Only one `#[imgui]` attribute per field is allowed.
        // If we encounter more than one, raise a compilation error
        if attributes.is_empty() {
            return Ok(TokenStream::new());
        } else if attributes.len() > 1 {
            return Err(Error::new(field.span(), "Only one `#[imgui]` tag per attribute is allowed"));
        }

        // At this point, we are parsing the following attribute:
        //
        // #[imgui( ... )]
        //   ^^^^^^^^^^^^
        // Therefore it is safe to unwrap
        let attr_meta = attributes.get(0).unwrap();
        let ident = field.ident.as_ref().unwrap();

        parse_meta(&ident, attr_meta)?.into_token_stream(&ident)
    }).collect::<Result<Vec<_>, Error>>()?;
    Ok(quote! {
        #( #field_assign );*
    })
}

fn is_imgui_attr(attr: &&Attribute) -> bool {
    attr.path.is_ident(Ident::new("imgui", Span::call_site()))
}
