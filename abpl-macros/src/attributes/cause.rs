use syn::Token;

pub struct CauseAttribute {
	pub cause: syn::Path,
	pub unserializable: bool,
}
impl CauseAttribute {
	fn apply_arg(&mut self, maybe_ident: &syn::Path) -> syn::Result<()> {
		let ident = maybe_ident.require_ident()?;
		if ident == "unserializable" {
			if self.unserializable {
				return Err(syn::Error::new_spanned(ident, "duplicate modifier"));
			}
			self.unserializable = true;
			return Ok(());
		}
		Err(syn::Error::new_spanned(ident, "unknown modifier"))
	}
	pub fn parse_from_single(maybe_cause_attribute: &syn::Attribute) -> syn::Result<Option<Self>> {
		if !maybe_cause_attribute.meta.path().is_ident("cause") {
			return Ok(None);
		}
		let meta_list = maybe_cause_attribute.meta.require_list()?;

		let mut args = meta_list
			.parse_args_with(syn::punctuated::Punctuated::<syn::Path, Token![,]>::parse_separated_nonempty)?
			.into_iter();

		let mut parsed_attribute = Self {
			cause: args
				.next()
				.expect("parse_separated_nonempty should have enforced nonempty"),
			unserializable: false,
		};
		for arg in args {
			parsed_attribute.apply_arg(&arg)?;
		}
		Ok(Some(parsed_attribute))
	}
	pub fn parse_from_slice(maybe_cause_attributes: &[syn::Attribute]) -> syn::Result<Vec<Self>> {
		let mut selves = Vec::new();
		for maybe_cause_attribute in maybe_cause_attributes {
			selves.extend(Self::parse_from_single(maybe_cause_attribute)?);
		}
		Ok(selves)
	}
}
