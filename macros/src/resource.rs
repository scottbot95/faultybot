use proc_macro::TokenStream;
use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro_error::{emit_call_site_error, emit_error};
use quote::{format_ident, quote, ToTokens};
use syn::DeriveInput;

#[derive(FromDeriveInput)]
struct Opts {
    data: darling::ast::Data<darling::util::Ignored, SpecifierField>,
}

#[derive(FromField, Clone)]
#[darling(forward_attrs(specifier))]
struct SpecifierField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    attrs: Vec<syn::Attribute>,
}

pub fn derive_resource(input: DeriveInput) -> TokenStream {
    let opts = Opts::from_derive_input(&input).expect("Must specify the actions via #[actions()]");

    let ident = input.ident;
    let vis = input.vis;
    let action_ty = format_ident!("{}Actions", ident);

    let actions = input.attrs.into_iter()
        .filter_map(|attr| if attr.path() == &syn::Path::from_string("actions").unwrap() {
            let parser = syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated;
            let args = attr.meta.require_list()
                .and_then(|list| list.parse_args_with(parser));

            if let Err(err) = &args {
                emit_call_site_error!("Failed to parse arguments: {}", err)
            }

            args.ok()
        } else {
            None
        });

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let specifier_field = if let Some(fields) = opts.data.take_struct() {
        let fields = fields.into_iter()
            .filter(|f|
                f.attrs.iter().any(|attr|
                    match attr.meta.path().segments.first() {
                        Some(segment) => segment.ident == "specifier",
                        None => false
                    }
                )
            )
            .collect::<Vec<_>>();

        if fields.len() > 1 {
            for f in &fields {
                emit_error!(f.ident, "Can only have at most one specifier");
            }
        }

        fields.first()
            .cloned()
            .and_then(|field| field.ident.map(|ident| (field.ty, ident)))
    } else {
        emit_error!(ident, "Resources must be a struct");
        None
    };

    let specifier_ty;
    let specifier_fn;
    if let Some((ty, field)) = specifier_field {
        specifier_ty = extract_inner_type(&ty)
            .unwrap_or(&ty)
            .to_token_stream();
        specifier_fn = quote!(&self.#field);
    } else {
        specifier_ty = quote!(());
        specifier_fn = quote!(&None);
    }

    let expanded = quote! {
        #vis enum #action_ty {
            #(#actions),*
        }

        impl #impl_generics crate::permissions::v2::Resource for #ident #ty_generics #where_clause {
            type Action = #action_ty;
            type Specifier = #specifier_ty;

            fn specifier(&self) -> &::core::option::Option<Self::Specifier> {
                #specifier_fn
            }
        }
    };

    TokenStream::from(expanded)
}


fn extract_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}