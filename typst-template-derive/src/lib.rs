//! Derive macro for the `ToDict` trait from the [`typst-template`] crate.
//!
//! This crate is an implementation detail of `typst-template`; depend on that
//! crate (with its default `derive` feature) instead of using this one
//! directly.
//!
//! [`typst-template`]: https://docs.rs/typst-template

#![forbid(unsafe_code)]

use darling::ast::Data;
use darling::util::Ignored;
use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Generics, Ident, Path, Type, parse_macro_input, parse_quote};

/// Derives [`ToDict`] (and, through it, [`ToValue`]) for a named struct.
///
/// Each field is converted to a Typst value via [`ToValue`] and inserted under
/// its name. The generated `into_value` wraps the resulting dict in
/// `Value::Dict`, so derived types nest inside one another.
///
/// # Attributes
///
/// Container level (`#[typst(...)]` on the struct):
/// - `rename_all = "..."` — rename every field with one of the rules `lowercase`, `UPPERCASE`,
///   `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case`,
///   `SCREAMING-KEBAB-CASE`.
///
/// Field level (`#[typst(...)]` on a field):
/// - `rename = "name"` — use a fixed key, overriding `rename_all`.
/// - `skip` — leave the field out of the dict.
/// - `with = "path::to::fn"` — call that function (named as a string, like serde) as `fn(field) ->
///   Value` to produce the value instead of using the field's [`ToValue`] impl. Useful for types
///   that do not implement the trait or that need a custom representation.
/// - `flatten` — merge the field's own dict into this one instead of nesting it under a key. The
///   field type must implement [`ToDict`]; `rename`/`with` are ignored for it.
///
/// [`ToDict`]: https://docs.rs/typst-template/latest/typst_template/trait.ToDict.html
/// [`ToValue`]: https://docs.rs/typst-template/latest/typst_template/trait.ToValue.html
#[proc_macro_derive(ToDict, attributes(typst))]
pub fn derive_into_dict(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    match ToDictInput::from_derive_input(&parsed) {
        Ok(receiver) => receiver.expand().into(),
        Err(err) => err.write_errors().into(),
    }
}

#[derive(FromDeriveInput)]
#[darling(attributes(typst), supports(struct_named))]
struct ToDictInput {
    ident: Ident,
    generics: Generics,
    data: Data<Ignored, ToDictField>,
    #[darling(default)]
    rename_all: Option<RenameRule>,
}

#[derive(FromField)]
#[darling(attributes(typst))]
struct ToDictField {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    skip: bool,
    #[darling(default)]
    with: Option<Path>,
    #[darling(default)]
    flatten: bool,
}

impl ToDictInput {
    fn expand(&self) -> proc_macro2::TokenStream {
        let ident = &self.ident;
        let data = self
            .data
            .as_ref()
            .take_struct()
            .expect("supports(struct_named)");
        let fields: Vec<&ToDictField> = data.fields.into_iter().filter(|f| !f.skip).collect();

        // One statement per kept field: a flattened field merges its dict,
        // others insert under their key.
        let inserts = fields.iter().map(|f| {
            let field = f.ident.as_ref().expect("named struct field");
            if f.flatten {
                return quote! {
                    ::typst_template::__private::merge_dict(
                        &mut __dict,
                        ::typst_template::ToDict::into_dict(self.#field),
                    );
                };
            }
            let key = self.key_for(f, field);
            let value = match &f.with {
                Some(path) => quote!(#path(self.#field)),
                None => quote!(::typst_template::ToValue::into_value(self.#field)),
            };
            quote!(__dict.insert(::typst_template::Str::from(#key), #value);)
        });

        // Bound each field type: `ToDict` when flattened, `ToValue` when
        // converted through the trait (a `with` field needs neither).
        let mut generics = self.generics.clone();
        let where_clause = generics.make_where_clause();
        for f in &fields {
            let ty = &f.ty;
            if f.flatten {
                where_clause
                    .predicates
                    .push(parse_quote!(#ty: ::typst_template::ToDict));
            } else if f.with.is_none() {
                where_clause
                    .predicates
                    .push(parse_quote!(#ty: ::typst_template::ToValue));
            }
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        quote! {
            impl #impl_generics ::typst_template::ToDict for #ident #ty_generics #where_clause {
                fn into_dict(self) -> ::typst_template::Dict {
                    let mut __dict = ::typst_template::Dict::new();
                    #(#inserts)*
                    __dict
                }
            }

            impl #impl_generics ::typst_template::ToValue for #ident #ty_generics #where_clause {
                fn into_value(self) -> ::typst_template::Value {
                    ::typst_template::Value::Dict(::typst_template::ToDict::into_dict(self))
                }
            }
        }
    }

    /// The dict key for a field: explicit `rename`, else `rename_all`, else the
    /// raw field name.
    fn key_for(&self, field: &ToDictField, ident: &Ident) -> String {
        if let Some(name) = &field.rename {
            return name.clone();
        }
        let raw = ident.to_string();
        let name = raw.strip_prefix("r#").unwrap_or(&raw);
        match self.rename_all {
            Some(rule) => rule.apply(name),
            None => name.to_owned(),
        }
    }
}

/// Field-renaming rules, mirroring serde's `rename_all`.
#[derive(Clone, Copy)]
enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl FromMeta for RenameRule {
    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(match value {
            "lowercase" => Self::Lower,
            "UPPERCASE" => Self::Upper,
            "PascalCase" => Self::Pascal,
            "camelCase" => Self::Camel,
            "snake_case" => Self::Snake,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            "kebab-case" => Self::Kebab,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebab,
            other => return Err(darling::Error::unknown_value(other)),
        })
    }
}

impl RenameRule {
    /// Rewrites a `snake_case` field name into the target style.
    fn apply(self, field: &str) -> String {
        let words: Vec<&str> = field.split('_').filter(|w| !w.is_empty()).collect();
        match self {
            Self::Snake => field.to_owned(),
            Self::Lower => words.concat(),
            Self::Upper => words.concat().to_uppercase(),
            Self::ScreamingSnake => join(&words, "_", str::to_uppercase),
            Self::Kebab => words.join("-"),
            Self::ScreamingKebab => join(&words, "-", str::to_uppercase),
            Self::Pascal => words.iter().map(|w| capitalize(w)).collect(),
            Self::Camel => {
                let mut out = String::new();
                for (i, word) in words.iter().enumerate() {
                    if i == 0 {
                        out.push_str(word);
                    } else {
                        out.push_str(&capitalize(word));
                    }
                }
                out
            }
        }
    }
}

fn join(words: &[&str], sep: &str, map: fn(&str) -> String) -> String {
    words.iter().map(|w| map(w)).collect::<Vec<_>>().join(sep)
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
