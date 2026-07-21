use syn::Token;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TraceKind {
	#[default]
	None,
	Location,
	Backtrace,
}

#[derive(Debug, Default)]
pub struct AbplErrorAttribute {
	pub trace_kind: TraceKind,
	pub deserialize: bool,
	pub serialize: bool,
	pub utoipa: bool,
}
impl AbplErrorAttribute {
	pub fn generate_serde_struct(&self) -> bool {
		self.deserialize || self.serialize || self.utoipa
	}
	fn apply_arg(&mut self, maybe_ident: &syn::Path) -> syn::Result<()> {
		let ident = maybe_ident.require_ident()?;
		if ident == "location" {
			if self.trace_kind != TraceKind::None {
				return Err(syn::Error::new_spanned(
					ident,
					"backtrace and location are mutually exclusive and must only be specified once",
				));
			}
			self.trace_kind = TraceKind::Location;
			return Ok(());
		} else if ident == "backtrace" {
			if self.trace_kind != TraceKind::None {
				return Err(syn::Error::new_spanned(
					ident,
					"backtrace and location are mutually exclusive and must only be specified once",
				));
			}
			self.trace_kind = TraceKind::Backtrace;
			return Ok(());
		} else if ident == "deserialize" {
			if self.deserialize {
				return Err(syn::Error::new_spanned(ident, "duplicate deserialize"));
			}
			self.deserialize = true;
			return Ok(());
		} else if ident == "serialize" {
			if self.serialize {
				return Err(syn::Error::new_spanned(ident, "duplicate serialize"));
			}
			self.serialize = true;
			return Ok(());
		} else if ident == "utoipa" {
			if self.utoipa {
				return Err(syn::Error::new_spanned(ident, "duplicate utoipa"));
			}
			self.utoipa = true;
			return Ok(());
		}
		Err(syn::Error::new_spanned(ident, "unknown modifier"))
	}
	pub fn parse_from_single(maybe_err_attribute: &syn::Attribute) -> syn::Result<Option<Self>> {
		if !maybe_err_attribute.meta.path().is_ident("abpl_error") {
			return Ok(None);
		}
		let meta_list = maybe_err_attribute.meta.require_list()?;

		let args = meta_list
			.parse_args_with(syn::punctuated::Punctuated::<syn::Path, Token![,]>::parse_separated_nonempty)?
			.into_iter();

		let mut parsed_attribute = Self::default();
		for arg in args {
			parsed_attribute.apply_arg(&arg)?;
		}
		Ok(Some(parsed_attribute))
	}
	pub fn parse_from_slice(maybe_err_attributes: &[syn::Attribute]) -> syn::Result<Option<Self>> {
		let mut maybe_self = None;
		for maybe_err_attribute in maybe_err_attributes {
			let maybe_maybe_self = Self::parse_from_single(maybe_err_attribute)?;
			if maybe_maybe_self.is_some() {
				if maybe_self.is_some() {
					return Err(syn::Error::new_spanned(
						maybe_err_attribute.path(),
						"duplicate abpl_error",
					));
				}
				maybe_self = maybe_maybe_self;
			}
		}
		Ok(maybe_self)
	}
}
