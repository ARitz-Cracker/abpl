use proc_macro::TokenStream;

pub(crate) mod attributes;
mod error;

/// Derives `abpl::Error` for an enum.
///
/// The annotated enum is treated as the "kind" enum. The macro generates:
/// - A wrapper struct `{Name}` (without "Kind" suffix)
///   containing `error_message`, `error_detail`, and `error_cause` fields.
/// - A `{Name}Cause` enum covering all types listed in `#[cause(...)]` attributes.
/// - `Serialize`/`Deserialize` impls for both generated types.
/// - `From<T>` impls for each `#[cause(T)]` on each variant.
/// - `std::error::Error` and `Display` delegation to the inner kind enum.
///
/// # Usage
/// ```ignore
/// #[derive(abpl::Error)]
/// enum MyErrorKind {
///     #[cause(unserializable(IoError))]
///     #[cause(ParseError)]
///     ReadFailed { path: String },
///     NotFound,
/// }
/// ```
#[proc_macro_derive(Error, attributes(cause, abpl_error, abpl_provider))]
pub fn derive_error(input: TokenStream) -> TokenStream {
	error::derive(syn::parse_macro_input!(input as syn::DeriveInput))
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}
