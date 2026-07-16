use std::collections::{HashMap, HashSet};

use convert_case::Casing;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{DeriveInput, Token};

use crate::attributes::{
	abpl_error::{AbplErrorAttribute, TraceKind},
	abpl_provider::{AbplProviderAttribute, AbplProviderAttributeItem},
	cause::CauseAttribute,
};

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
	let top_level_error_attr = AbplErrorAttribute::parse_from_slice(&input.attrs)?.unwrap_or_default();
	let top_level_provider_attr = AbplProviderAttribute::parse_from_slice(&input.attrs)?;

	let mut variant_provider_fn_name_to_trait = HashMap::<syn::Ident, Vec<syn::Path>>::new();
	let mut provider_trait_match_body = HashMap::<syn::Path, TokenStream>::new();
	// So the per-variant loop below can resolve `cause` (delegate-to-cause) return values back
	// to the trait's own method name and default, without waiting for the final assembly loop.
	let mut provider_trait_fn_name = HashMap::<syn::Path, syn::Ident>::new();
	let mut provider_trait_default_return_value = HashMap::<syn::Path, syn::Expr>::new();

	for item in top_level_provider_attr.items.iter() {
		let Some(fn_name) = item.fn_name.clone() else {
			return Err(syn::Error::new_spanned(&item.trait_or_fn_name, "expected 3 arguments"));
		};
		variant_provider_fn_name_to_trait
			.entry(fn_name.clone())
			.or_default()
			.push(item.trait_or_fn_name.clone());
		provider_trait_match_body.insert(item.trait_or_fn_name.clone(), TokenStream::new());
		provider_trait_fn_name.insert(item.trait_or_fn_name.clone(), fn_name);
		provider_trait_default_return_value.insert(item.trait_or_fn_name.clone(), item.return_value.clone());
	}

	let new_error_trace = match top_level_error_attr.trace_kind {
		TraceKind::None => quote! { ::abpl::error::ErrorTrace::default() },
		TraceKind::Location => quote! { ::abpl::error::ErrorTrace::new_location() },
		TraceKind::Backtrace => quote! { ::abpl::error::ErrorTrace::new_backtrace() },
	};

	let mut exported_body = TokenStream::new();
	let mut sealed_body = TokenStream::new();

	let syn::Data::Enum(input_enum) = input.data else {
		return Err(syn::Error::new_spanned(input, "abpl::Error: must be an enum"));
	};

	// We can't use `input.ident.span().source_text()` to create the substring because `rust-analyzer` doesn't seem to
	// provide source text
	let input_ident_string = input.ident.to_string();
	let Some(struct_name_str) = Some(input_ident_string.as_str())
		.filter(|str| str.ends_with("Kind"))
		.map(|str| &str[0..(str.len() - "Kind".len())])
	else {
		return Err(syn::Error::new_spanned(
			input.ident,
			"abpl::Error: enum name must end with \"Kind\"",
		));
	};

	struct EnumVariantDetail {
		ident: syn::Ident,
		fields: syn::Fields,
		causes: Vec<CauseAttribute>,
		providers: Vec<AbplProviderAttributeItem>,
	}
	let mut enum_variant_details = Vec::<EnumVariantDetail>::new();
	let mut cause_to_field_count = HashMap::<syn::Path, u32>::new();

	for variant in input_enum.variants.iter() {
		let causes = CauseAttribute::parse_from_slice(&variant.attrs)?;
		for cause_attr in causes.iter() {
			*cause_to_field_count.entry(cause_attr.cause.clone()).or_default() += 1;
		}
		let providers = AbplProviderAttribute::parse_from_slice(&variant.attrs)?.items;
		for provider in providers.iter() {
			if let Some(fn_name) = &provider.fn_name {
				return Err(syn::Error::new_spanned(fn_name, "unexpected argument"));
			}
		}
		enum_variant_details.push(EnumVariantDetail {
			ident: variant.ident.clone(),
			fields: variant.fields.clone(),
			causes,
			providers,
		});
	}

	let generated_struct_ident = syn::Ident::new(struct_name_str, Span::call_site());
	let input_ident = &input.ident;
	let input_visibility = &input.vis;
	exported_body.append_all(quote! {
		#[derive(Debug, Clone)]
		#[automatically_derived]
		#input_visibility struct #generated_struct_ident {
			kind: #input_ident,
			cause: ::core::option::Option<::abpl::maybe_std::Rc<dyn ::core::error::Error>>,
			trace: ::abpl::error::ErrorTrace,
		}
		impl #generated_struct_ident {
			#[track_caller]
			pub fn new(kind: #input_ident) -> Self {
				Self {
					cause: ::core::option::Option::None,
					kind,
					trace: #new_error_trace,
				}
			}
			#[track_caller]
			pub fn new_with_cause<T: core::error::Error + 'static>(kind: #input_ident, maybe_cause: Option<T>) -> Self {
				Self {
					cause: maybe_cause.map(|cause| {
						let cause_box: ::abpl::maybe_std::Rc<dyn ::core::error::Error> =
							::abpl::maybe_std::Rc::new(cause);
						cause_box
					}),
					kind,
					trace: #new_error_trace,
				}
			}
			pub fn kind(&self) -> &#input_ident {
				&self.kind
			}
		}
		impl ::core::fmt::Display for #generated_struct_ident {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				::abpl::error::fmt_generated_error(f, &self.kind, &self.trace, self.cause.as_deref())
			}
		}
		impl ::core::error::Error for #generated_struct_ident {
			fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
				self.cause.as_deref()
			}
		}
	});

	for variant_detail in enum_variant_details.iter() {
		// Quick means for us to lazily define the trait and fn signature
		let mut result_trait_ident_and_fn: Option<(syn::Ident, syn::Signature)> = None;

		// Generating `From` blocks and traits for the `Result` types for more conveneint map_err_*
		for cause_attr in variant_detail.causes.iter() {
			if matches!(variant_detail.fields, syn::Fields::Unit)
				&& cause_to_field_count
					.get(&cause_attr.cause)
					.is_none_or(|count| *count <= 1)
			{
				// Enum has no fields, and has a single cause -> kind relationship, so we can just make a From impl.
				let cause_ident_path = &cause_attr.cause;
				let variant_ident = &variant_detail.ident;
				exported_body.append_all(quote! {
					impl From<#cause_ident_path> for #generated_struct_ident {
						fn from(input: #cause_ident_path) -> Self {
							Self {
								kind: #input_ident::#variant_ident,
								cause: ::core::option::Option::Some(::abpl::maybe_std::Rc::new(input)),
								trace: #new_error_trace
							}
						}
					}
				});
			} else {
				let (result_trait_ident, result_trait_fn) = &*result_trait_ident_and_fn.get_or_insert_with(|| {
					let result_trait_name = format!("ResultInto{struct_name_str}{}", variant_detail.ident);
					let result_trait_ident = syn::Ident::new(&result_trait_name, Span::call_site());

					let function_name = format!(
						"map_err_{}",
						variant_detail.ident.to_string().to_case(convert_case::Case::Snake)
					);

					let mut result_trait_func_params: syn::punctuated::Punctuated<syn::FnArg, Token![,]> =
						Default::default();
					result_trait_func_params.push(syn::parse_quote!(self)); // Consumes self

					for (i, enum_field) in variant_detail.fields.iter().enumerate() {
						let func_param_type = &enum_field.ty;
						let func_param_ident = if let Some(ident) = &enum_field.ident {
							ident
						} else {
							// enum payload is tuple-like, so we'll have to generate argument names
							&syn::Ident::new(&format!("arg_{i}"), Span::call_site())
						};
						result_trait_func_params.push(syn::parse_quote! { #func_param_ident: #func_param_type });
					}
					let result_trait_func_sig = syn::Signature {
						constness: None,
						asyncness: None,
						unsafety: None,
						abi: None,
						fn_token: Default::default(),
						ident: syn::Ident::new(&function_name, variant_detail.ident.span()),
						generics: Default::default(),
						paren_token: Default::default(),
						inputs: result_trait_func_params,
						variadic: Default::default(),
						output: syn::parse_quote! { -> Result<T, #generated_struct_ident> },
					};
					exported_body.append_all(quote! {
						pub trait #result_trait_ident<T> {
							#result_trait_func_sig;
						}
					});
					(result_trait_ident, result_trait_func_sig)
				});
				let cause_ident_path = &cause_attr.cause;
				let variant_ident = &variant_detail.ident;

				let mut new_enum_fields: syn::punctuated::Punctuated<syn::Ident, Token![,]> = Default::default();
				for fn_arg in result_trait_fn.inputs.iter() {
					let syn::FnArg::Typed(fn_arg) = fn_arg else {
						// skip the `self`
						continue;
					};
					let fn_arg = match &*fn_arg.pat {
						syn::Pat::Ident(fn_arg) => fn_arg,
						_ => unreachable!(
							"If the fn args were all defined using idents, then destructuring them again into an ident must also be possible"
						),
					};
					new_enum_fields.push(fn_arg.ident.clone());
				}
				let enum_fields_with_paren = match variant_detail.fields {
					syn::Fields::Unit => TokenStream::new(),
					syn::Fields::Named(_) => {
						// the generated argument names in the `result_trait_fn` are the same as the named fields, so this should work.
						quote! {{#new_enum_fields}}
					},
					syn::Fields::Unnamed(_) => quote! {(#new_enum_fields)},
				};
				exported_body.append_all(quote! {
					impl<T> #result_trait_ident<T> for Result<T, #cause_ident_path> {
						#[track_caller]
						#result_trait_fn {
							// Can't use map_err because of #[track_caller]
							match self {
								Ok(inner) => Ok(inner),
								Err(input) => Err(#generated_struct_ident::new_with_cause(#input_ident::#variant_ident #enum_fields_with_paren, Some(input))),
							}
						}
					}
				});
			}
		}

		// Now we generate any needed provider traits
		// because I wanted the "convenience" of using the fn name, we gotta keep track of the traits used here
		let mut seen_paths = HashSet::<syn::Path>::new();
		for provider_attr in variant_detail.providers.iter() {
			let trait_name = match provider_attr
				.trait_or_fn_name
				.get_ident()
				.and_then(|fn_name| variant_provider_fn_name_to_trait.get(fn_name))
			{
				Some(trait_names) => {
					if trait_names.len() > 1 {
						let mut trait_name_list: syn::punctuated::Punctuated<syn::Path, Token![,]> = Default::default();
						trait_name_list.extend(trait_names.iter().cloned());
						let trait_name_list = trait_name_list.into_token_stream();
						return Err(syn::Error::new_spanned(
							&provider_attr.trait_or_fn_name,
							format!(
								"function name is ambiguous as it's used by multiple provider traits; replace with one of: {trait_name_list}"
							),
						));
					}
					&trait_names[0] // at least 1 item must exist
				},
				None => &provider_attr.trait_or_fn_name,
			};

			let Some(provider_trait_match_body) = provider_trait_match_body.get_mut(trait_name) else {
				return Err(syn::Error::new_spanned(
					&trait_name,
					format!("trait must match one of the ones defiend on the top-level abpl_provider attribute"),
				));
			};
			if !seen_paths.insert(trait_name.clone()) {
				let trait_name_to_string = trait_name.to_token_stream();

				return Err(syn::Error::new_spanned(
					&provider_attr.trait_or_fn_name,
					format!("duplicate use of {trait_name_to_string}"),
				));
			}

			//let cause_ident_path = &cause_attr.cause;
			let variant_ident = &variant_detail.ident;

			let match_arm_ts = match &variant_detail.fields {
				syn::Fields::Named(fields_named) => {
					let mut destructure_params: syn::punctuated::Punctuated<syn::Ident, Token![,]> = Default::default();
					for field in fields_named.named.iter() {
						destructure_params.push(field.ident.clone().expect("named fields should have names"));
					}
					quote! { #input_ident::#variant_ident { #destructure_params } }
				},
				syn::Fields::Unnamed(fields_unnamed) => {
					let mut destructure_params: syn::punctuated::Punctuated<syn::Ident, Token![,]> = Default::default();
					for (i, _) in fields_unnamed.unnamed.iter().enumerate() {
						destructure_params.push(syn::Ident::new(&format!("_{i}"), Span::call_site()));
					}
					quote! { #input_ident::#variant_ident (#destructure_params) }
				},
				syn::Fields::Unit => quote! { #input_ident::#variant_ident },
			};
			let return_value = &provider_attr.return_value;
			if let syn::Expr::Path(return_value_path) = return_value
				&& return_value_path.path.is_ident("cause")
			{
				if variant_detail.causes.is_empty() {
					return Err(syn::Error::new_spanned(
						return_value_path,
						"cause delegation requires at least one #[cause(..)] on this variant",
					));
				}
				let provider_fn_name = &provider_trait_fn_name[trait_name];
				let default_return_value = &provider_trait_default_return_value[trait_name];
				// Try each declared cause in turn (via downcast, same as the serde path); if
				// none match -- no cause set, or some other type entirely -- fall back to
				// whatever the trait's own default would have produced for this variant.
				let mut downcast_chain = quote! { #default_return_value };
				for cause in variant_detail.causes.iter().rev() {
					let cause_ty = &cause.cause;
					downcast_chain = quote! {
						if let ::core::option::Option::Some(__abpl_cause) = self.cause.as_deref().and_then(|__abpl_c| __abpl_c.downcast_ref::<#cause_ty>()) {
							__abpl_cause.#provider_fn_name()
						} else {
							#downcast_chain
						}
					};
				}
				provider_trait_match_body.append_all(quote! {#match_arm_ts => {#downcast_chain}, });
			} else {
				provider_trait_match_body.append_all(quote! {#match_arm_ts => {#return_value}, });
			}
		}
	}

	// Now all the match arms have been defined for the provider traits,

	for provider_attr in top_level_provider_attr.items.iter() {
		let trait_name = &provider_attr.trait_or_fn_name;
		let fn_name = provider_attr
			.fn_name
			.as_ref()
			.expect("top-level definition must have fn name");

		let Some(fn_return_type) = &provider_attr.fn_return_type else {
			return Err(syn::Error::new_spanned(trait_name, "expected 3 arguments"));
		};

		let default_return_value = &provider_attr.return_value;

		let provider_trait_match_body = provider_trait_match_body
			.remove(&trait_name)
			.unwrap_or_else(|| TokenStream::new());

		sealed_body.append_all(quote! {
			impl #trait_name for #generated_struct_ident {
				fn #fn_name(&self) -> #fn_return_type {
					match self.kind() {
						#provider_trait_match_body
						_ => #default_return_value,
					}
				}
			}
		});
	}

	if top_level_error_attr.generate_serde_struct() {
		let mut derives = TokenStream::new();
		if top_level_error_attr.deserialize {
			derives.append_all(quote! {
				#[derive(::serde::Deserialize)]
			});
		}
		if top_level_error_attr.serialize {
			derives.append_all(quote! {
				#[derive(::serde::Serialize)]
			});
		}
		if top_level_error_attr.utoipa {
			derives.append_all(quote! {
				#[derive(::utoipa::ToSchema)]
			});
		}
		let parseable_struct_ident = syn::Ident::new(&format!("Parsable{struct_name_str}"), Span::call_site());
		let kind_and_cause_ident = syn::Ident::new(&format!("{struct_name_str}KindAndCause"), Span::call_site());

		// Named fields are self-terminating (`{ .. }`) when used as a pattern or a construction
		// expression, so the same tokens work for both directions; Unit/Unnamed just need
		// consistent placeholder names (`arg_N`) to bind/rebind through.
		fn fields_bind_pattern(fields: &syn::Fields) -> TokenStream {
			match fields {
				syn::Fields::Unit => TokenStream::new(),
				syn::Fields::Named(fields_named) => {
					let idents: Vec<syn::Ident> = fields_named
						.named
						.iter()
						.map(|field| field.ident.clone().expect("named fields should have names"))
						.collect();
					quote! { { #(#idents),* } }
				},
				syn::Fields::Unnamed(fields_unnamed) => {
					let idents: Vec<syn::Ident> = (0..fields_unnamed.unnamed.len())
						.map(|i| syn::Ident::new(&format!("arg_{i}"), Span::call_site()))
						.collect();
					quote! { ( #(#idents),* ) }
				},
			}
		}

		let mut kind_and_cause_variants = TokenStream::new();
		let mut detail_structs: TokenStream = TokenStream::new();
		let mut cause_enums = TokenStream::new();
		let mut serialize_match_arms = TokenStream::new();
		let mut deserialize_match_arms = TokenStream::new();

		// `#kind_and_cause_ident`/`#parseable_struct_ident` only need to be generic over `'a`
		// if some variant somewhere actually has a `MaybeBorrowed<'a, _>` to hold; otherwise
		// declaring `<'a>` on them would itself be an unused lifetime parameter.
		let any_variant_has_borrowed_causes = enum_variant_details
			.iter()
			.any(|variant_detail| variant_detail.causes.iter().any(|cause| !cause.unserializable));
		let kind_and_cause_lifetime = if any_variant_has_borrowed_causes {
			quote! { <'a> }
		} else {
			TokenStream::new()
		};

		for variant_detail in enum_variant_details.iter() {
			let variant_ident = &variant_detail.ident;

			let cause_enum_ident = syn::Ident::new(&format!("_{input_ident}_{variant_ident}Cause"), Span::call_site());
			let detail_ident = syn::Ident::new(&format!("_{input_ident}_{variant_ident}"), Span::call_site());
			let detail_fields = &variant_detail.fields;
			let detail_struct = syn::ItemStruct {
				ident: detail_ident.clone(),
				fields: detail_fields.clone(),
				attrs: Default::default(),
				vis: syn::Visibility::Inherited,
				struct_token: Default::default(),
				generics: Default::default(),
				semi_token: Default::default(),
			};
			derives.to_tokens(&mut detail_structs);
			detail_struct.to_tokens(&mut detail_structs);

			let fields_pattern = fields_bind_pattern(detail_fields);

			let mut cause_enum_variants = TokenStream::new();
			let mut cause_reconstruction_arms = TokenStream::new();
			// Ordered "try each declared, non-unserializable cause type via downcast, else
			// erase" chain used on the serialize path; `Option::or_else` short-circuits on the
			// first match, same precedence the deserialize side's `#[serde(untagged)]` gives.
			let mut cause_downcast_chain = quote! { ::core::option::Option::None };
			for (i, cause) in variant_detail.causes.iter().enumerate() {
				if cause.unserializable {
					// error types marked as unserializable should use ::abpl::error::UnserializableError::from_error
					// when converting #generated_struct_ident to #parseable_struct_ident
					continue;
				}
				let cause_enum_variant_ident = syn::Ident::new(&format!("E{i}"), Span::call_site());
				let cause_type = &cause.cause;
				cause_enum_variants.append_all(quote! {
					#cause_enum_variant_ident(::abpl::types::MaybeBorrowed<'a, #cause_type>),
				});
				cause_reconstruction_arms.append_all(quote! {
					::core::option::Option::Some(#cause_enum_ident::#cause_enum_variant_ident(::abpl::types::MaybeBorrowed::Owned(__abpl_v))) => {
						let __abpl_boxed: ::abpl::maybe_std::Rc<dyn ::core::error::Error> = ::abpl::maybe_std::Rc::new(__abpl_v);
						::core::option::Option::Some(__abpl_boxed)
					},
					::core::option::Option::Some(#cause_enum_ident::#cause_enum_variant_ident(::abpl::types::MaybeBorrowed::Borrowed(_))) =>
						::core::unreachable!("MaybeBorrowed::Borrowed is never produced by deserialize"),
				});
				cause_downcast_chain = quote! {
					#cause_downcast_chain.or_else(|| {
						self.cause.as_deref()
							.and_then(|__abpl_c| __abpl_c.downcast_ref::<#cause_type>())
							.map(|__abpl_v| #cause_enum_ident::#cause_enum_variant_ident(::abpl::types::MaybeBorrowed::Borrowed(__abpl_v)))
					})
				};
			}
			// Only generic over `'a` if this variant actually has a declared, non-`unserializable`
			// cause to hold as `MaybeBorrowed<'a, _>`; otherwise `'a` would go unused.
			let cause_enum_lifetime = if cause_enum_variants.is_empty() {
				TokenStream::new()
			} else {
				quote! { <'a> }
			};
			cause_enums.append_all(quote! {
				#derives
				// Lets the deserializer try each declared cause type (and then the erased /
				// unknown fallbacks) in turn, since there's no explicit tag to dispatch on.
				#[serde(untagged)]
				enum #cause_enum_ident #cause_enum_lifetime {
					#cause_enum_variants
					Erased(::abpl::error::UnserializableError),
					#[serde(skip_serializing)]
					Unknown(::serde::de::IgnoredAny),
				}
			});

			// `cause` is `Option`-wrapped because `Test::new(kind)` can leave the wrapper's
			// `cause` as `None` for any variant, even ones that declare `#[cause(..)]`. It's
			// generic over `'a` because each declared, non-`unserializable` cause type is held
			// as `MaybeBorrowed<'a, Type>`, which lets the serialize path hand over a downcast
			// `&Type` directly (no `Clone`/`ToOwned` bound needed) while deserialize always
			// produces an owned value regardless of `'a`.
			kind_and_cause_variants.append_all(quote! {
				#variant_ident {
					#[serde(rename = "errorCause", default, skip_serializing_if = "Option::is_none")]
					cause: ::core::option::Option<#cause_enum_ident #cause_enum_lifetime>,
					#[serde(rename = "errorDetail")]
					detail: #detail_ident
				},
			});

			// Try each declared cause type in turn (via downcast, borrowing rather than
			// cloning); if none match -- either a foreign cause type, or one explicitly marked
			// `unserializable` -- fall back to erasing it via `UnserializableError::from_error`.
			serialize_match_arms.append_all(quote! {
				#input_ident::#variant_ident #fields_pattern => #kind_and_cause_ident::#variant_ident {
					cause: #cause_downcast_chain.or_else(|| {
						self.cause.as_deref().map(|__abpl_c| {
							#cause_enum_ident::Erased(::abpl::error::UnserializableError::from_error(__abpl_c))
						})
					}),
					detail: #detail_ident #fields_pattern,
				},
			});

			deserialize_match_arms.append_all(quote! {
				#kind_and_cause_ident::#variant_ident { cause, detail: #detail_ident #fields_pattern } => (
					#input_ident::#variant_ident #fields_pattern,
					match cause {
						#cause_reconstruction_arms
						::core::option::Option::Some(#cause_enum_ident::Erased(__abpl_erased)) => {
							let __abpl_boxed: ::abpl::maybe_std::Rc<dyn ::core::error::Error> = ::abpl::maybe_std::Rc::new(__abpl_erased);
							::core::option::Option::Some(__abpl_boxed)
						},
						::core::option::Option::Some(#cause_enum_ident::Unknown(_))
						| ::core::option::Option::None => ::core::option::Option::None,
					},
				),
			});
		}

		sealed_body.append_all(quote! {
			#detail_structs
			#cause_enums

			#derives
			#[serde(tag = "errorKind", rename_all = "camelCase")]
			enum #kind_and_cause_ident #kind_and_cause_lifetime {
				#kind_and_cause_variants
			}
			#derives
			struct #parseable_struct_ident #kind_and_cause_lifetime {
				#[serde(rename = "errorMessage")]
				msg: String,
				#[serde(rename = "errorTrace")]
				trace: ::abpl::error::ErrorTrace,
				#[serde(flatten)]
				detail: #kind_and_cause_ident #kind_and_cause_lifetime,
			}
		});

		if top_level_error_attr.serialize {
			sealed_body.append_all(quote! {
				impl ::serde::Serialize for #generated_struct_ident {
					fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
					where
						S: ::serde::Serializer,
					{
						let __abpl_detail = match self.kind.clone() {
							#serialize_match_arms
						};
						::serde::Serialize::serialize(
							&#parseable_struct_ident {
								msg: self.kind.to_string(),
								trace: self.trace.clone(),
								detail: __abpl_detail,
							},
							serializer,
						)
					}
				}
			});
		}

		if top_level_error_attr.deserialize {
			sealed_body.append_all(quote! {
				impl<'de> ::serde::Deserialize<'de> for #generated_struct_ident {
					fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
					where
						D: ::serde::Deserializer<'de>,
					{
						let __abpl_parseable = <#parseable_struct_ident as ::serde::Deserialize>::deserialize(deserializer)?;
						let (kind, cause) = match __abpl_parseable.detail {
							#deserialize_match_arms
						};
						::core::result::Result::Ok(Self {
							kind,
							cause,
							trace: __abpl_parseable.trace,
						})
					}
				}
			});
		}

		if top_level_error_attr.utoipa {
			sealed_body.append_all(quote! {
				impl ::utoipa::PartialSchema for #generated_struct_ident {
					fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema> {
						<#parseable_struct_ident as ::utoipa::PartialSchema>::schema()
					}
				}
				impl ::utoipa::ToSchema for #generated_struct_ident {
					fn name() -> ::std::borrow::Cow<'static, str> {
						::std::borrow::Cow::Borrowed(#struct_name_str)
					}
				}
			});
		}
	}
	Ok(quote! {
		#exported_body
		const _: () = {
			#sealed_body
		};
	})
}
