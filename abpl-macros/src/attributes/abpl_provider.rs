use std::collections::HashSet;

use quote::ToTokens;
use syn::Token;
#[derive(Debug, Clone)]
pub struct AbplProviderAttributeItem {
	pub trait_or_fn_name: syn::Path,
	pub return_value: syn::Expr,
	pub fn_name: Option<syn::Ident>,
	pub fn_return_type: Option<syn::Path>,
}

#[derive(Debug, Default, Clone)]
pub struct AbplProviderAttribute {
	pub items: Vec<AbplProviderAttributeItem>,
}

impl AbplProviderAttribute {
	pub fn parse_from_single(
		maybe_provider_attribute: &syn::Attribute,
		seen_values: &mut HashSet<syn::Path>,
	) -> syn::Result<Self> {
		let mut parsed_attribute = Self::default();
		if !maybe_provider_attribute.meta.path().is_ident("abpl_provider") {
			return Ok(parsed_attribute);
		}
		let meta_list = maybe_provider_attribute.meta.require_list()?;

		for arg in meta_list
			.parse_args_with(syn::punctuated::Punctuated::<syn::ExprCall, Token![,]>::parse_separated_nonempty)?
		{
			let trait_or_fn_name = match *arg.func {
				syn::Expr::Path(syn::ExprPath { path, attrs, qself }) => {
					if let Some(qself) = qself {
						return Err(syn::Error::new_spanned(
							qself.lt_token,
							"unexpected Self type qualifier",
						));
					}
					if let Some(attr) = attrs.get(0) {
						return Err(syn::Error::new_spanned(attr, "unexpected inner-attributes"));
					}
					path
				},
				_ => {
					return Err(syn::Error::new_spanned(
						arg.func,
						"expected a path to a named item or identifier",
					));
				},
			};
			if !seen_values.insert(trait_or_fn_name.clone()) {
				let thing_to_string = trait_or_fn_name.to_token_stream();
				return Err(syn::Error::new_spanned(
					trait_or_fn_name,
					format!("duplicate {thing_to_string}"),
				));
			}
			let mut fn_arg_iter = arg.args.into_iter();
			let Some(return_value) = fn_arg_iter.next() else {
				return Err(syn::Error::new_spanned(
					trait_or_fn_name,
					"expected at least 1 argument",
				));
			};
			let fn_name = fn_arg_iter
				.next()
				.map(|maybe_fn_name| match maybe_fn_name {
					syn::Expr::Path(syn::ExprPath { path, attrs, qself }) => {
						if let Some(qself) = qself {
							return Err(syn::Error::new_spanned(
								qself.lt_token,
								"unexpected Self type qualifier",
							));
						}
						if let Some(attr) = attrs.get(0) {
							return Err(syn::Error::new_spanned(attr, "unexpected inner-attributes"));
						}
						path.require_ident().cloned()
					},
					_ => Err(syn::Error::new_spanned(maybe_fn_name, "expected a named item")),
				})
				.transpose()?;

			let fn_return_type = fn_arg_iter
				.next()
				.map(|maybe_fn_name| match maybe_fn_name {
					syn::Expr::Path(syn::ExprPath { path, attrs, qself }) => {
						if let Some(qself) = qself {
							return Err(syn::Error::new_spanned(
								qself.lt_token,
								"unexpected Self type qualifier",
							));
						}
						if let Some(attr) = attrs.get(0) {
							return Err(syn::Error::new_spanned(attr, "unexpected inner-attributes"));
						}
						Ok(path)
					},
					_ => Err(syn::Error::new_spanned(maybe_fn_name, "expected a named item")),
				})
				.transpose()?;
			if let Some(extra_arg) = fn_arg_iter.next() {
				return Err(syn::Error::new_spanned(extra_arg, "unexpected argument"));
			}
			parsed_attribute.items.push(AbplProviderAttributeItem {
				trait_or_fn_name,
				return_value,
				fn_name,
				fn_return_type,
			});
		}
		Ok(parsed_attribute)
	}
	pub fn parse_from_slice(maybe_provider_attributes: &[syn::Attribute]) -> syn::Result<Self> {
		let mut seen_values = HashSet::new();
		let mut aggregated = Self::default();
		for maybe_provider_attribute in maybe_provider_attributes {
			aggregated
				.items
				.extend(Self::parse_from_single(maybe_provider_attribute, &mut seen_values)?.items);
		}
		Ok(aggregated)
	}
}
