use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derives `as_str()`, `FromStr`, and `Display` implementations for enums with string mappings.
///
/// Each variant is mapped based on serde rename attributes.
/// Supports `#[serde(rename = "...")]` on variants and `#[serde(rename_all = "...")]` on enum.
#[proc_macro_derive(StringEnum)]
pub fn derive_string_enum(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let name = &input.ident;
  let variants = match &input.data {
    Data::Enum(data) => &data.variants,
    _ => {
      return syn::Error::new_spanned(&input, "StringEnum can only be derived for enums")
        .to_compile_error()
        .into();
    }
  };

  let rename_all_rule = extract_rename_all(&input.attrs);

  let mut string_cases = Vec::new();
  let mut from_str_cases = Vec::new();

  for variant in variants {
    if !matches!(variant.fields, Fields::Unit) {
      return syn::Error::new_spanned(variant, "StringEnum only supports unit variants")
        .to_compile_error()
        .into();
    }

    let variant_name = &variant.ident;
    let variant_name_str = variant_name.to_string();

    let variant_str = extract_rename(&variant.attrs)
      .or_else(|| apply_rename_all(&variant_name_str, &rename_all_rule))
      .unwrap_or(variant_name_str);

    string_cases.push(quote! {
        Self::#variant_name => #variant_str
    });

    from_str_cases.push(quote! {
        #variant_str => Ok(Self::#variant_name)
    });
  }

  let expanded = quote! {
      impl #name {
          pub fn as_str(&self) -> &'static str {
              match self {
                  #(#string_cases),*
              }
          }
      }

      impl core::fmt::Display for #name {
          fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
              f.write_str(self.as_str())
          }
      }

      impl core::str::FromStr for #name {
          type Err = ();

          fn from_str(s: &str) -> Result<Self, Self::Err> {
              match s {
                  #(#from_str_cases),*,
                  _ => Err(()),
              }
          }
      }

      impl From<#name> for alloc::string::String {
          fn from(e: #name) -> Self {
              e.to_string()
          }
      }
  };

  TokenStream::from(expanded)
}

fn extract_rename(attrs: &[syn::Attribute]) -> Option<String> {
  for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
    if let syn::Meta::NameValue(nv) = &attr.meta {
      if nv.path.is_ident("rename") {
        if let syn::Expr::Lit(syn::ExprLit {
          lit: syn::Lit::Str(lit_str),
          ..
        }) = &nv.value
        {
          return Some(lit_str.value());
        }
      }
    }
  }
  None
}

fn extract_rename_all(attrs: &[syn::Attribute]) -> Option<String> {
  for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
    if let syn::Meta::List(meta_list) = &attr.meta {
      if let Ok(nested_meta) = meta_list.parse_args::<syn::MetaNameValue>() {
        if nested_meta.path.is_ident("rename_all") {
          if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit_str),
            ..
          }) = &nested_meta.value
          {
            return Some(lit_str.value());
          }
        }
      }
    }
  }
  None
}

fn apply_rename_all(name: &str, rule: &Option<String>) -> Option<String> {
  rule.as_ref().map(|r| match r.as_str() {
    "lowercase" => name.to_lowercase(),
    "SCREAMING_SNAKE_CASE" => screaming_snake_case(name),
    "snake_case" => snake_case(name),
    _ => name.to_string(),
  })
}

fn screaming_snake_case(s: &str) -> String {
  let mut result = String::new();
  for (i, c) in s.chars().enumerate() {
    if i > 0 && c.is_uppercase() {
      result.push('_');
    }
    for upper in c.to_uppercase() {
      result.push(upper);
    }
  }
  result
}

fn snake_case(s: &str) -> String {
  let mut result = String::new();
  for (i, c) in s.chars().enumerate() {
    if i > 0 && c.is_uppercase() {
      result.push('_');
    }
    for lower in c.to_lowercase() {
      result.push(lower);
    }
  }
  result
}
