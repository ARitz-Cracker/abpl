#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes through either an owned value or a borrowed reference, but always deserializes
/// into an owned value. Lets a value only reachable by reference (e.g. after downcasting a
/// type-erased trait object) be serialized without requiring `Clone`/`ToOwned` on it.
#[derive(Debug)]
pub enum MaybeBorrowed<'a, T> {
	Owned(T),
	Borrowed(&'a T),
}

#[cfg(feature = "serde")]
impl<'a, T: Serialize> Serialize for MaybeBorrowed<'a, T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			Self::Owned(inner) => inner.serialize(serializer),
			Self::Borrowed(inner) => inner.serialize(serializer),
		}
	}
}

// Generic over 'a rather than fixed to 'static: `Owned` never touches 'a at runtime, so nothing
// forces the deserialized instantiation to be 'static specifically.
#[cfg(feature = "serde")]
impl<'de, 'a, T: Deserialize<'de>> Deserialize<'de> for MaybeBorrowed<'a, T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(Self::Owned(T::deserialize(deserializer)?))
	}
}

// `#[derive(ToSchema)]` on a struct/enum composes its field schemas via `ComposeSchema`
// specifically (not just `PartialSchema`), so we implement that directly; `PartialSchema` then
// comes for free via utoipa's blanket `impl<T: ComposeSchema> PartialSchema for T`.
#[cfg(feature = "utoipa")]
impl<'a, T: utoipa::__dev::ComposeSchema> utoipa::__dev::ComposeSchema for MaybeBorrowed<'a, T> {
	fn compose(
		generics: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
	) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
		T::compose(generics)
	}
}

#[cfg(feature = "utoipa")]
impl<'a, T: utoipa::ToSchema + utoipa::__dev::ComposeSchema> utoipa::ToSchema for MaybeBorrowed<'a, T> {
	fn name() -> std::borrow::Cow<'static, str> {
		T::name()
	}
}

#[cfg(test)]
#[path = "../tests/types/maybe_borrowed.rs"]
mod tests;
