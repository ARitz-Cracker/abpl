use core::{
	borrow::{Borrow, BorrowMut},
	fmt::Display,
	ops::{Deref, DerefMut},
	str::FromStr,
};

use base64::{Engine as _, display::Base64Display, engine::GeneralPurpose as Base64Engine};
use bytes::{Bytes, BytesMut};
use serde_with::{DeserializeFromStr, SerializeDisplay};
pub const BASE64_STANDARD_WHATEVER_PAD: Base64Engine = Base64Engine::new(
	&base64::alphabet::STANDARD,
	base64::engine::general_purpose::GeneralPurposeConfig::new()
		.with_encode_padding(false)
		.with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

pub const BASE64_URL_WHATEVER_PAD: Base64Engine = Base64Engine::new(
	&base64::alphabet::URL_SAFE,
	base64::engine::general_purpose::GeneralPurposeConfig::new()
		.with_encode_padding(false)
		.with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

/// Immutable byte buffer that serializes as a base64 string.
///
/// Padding is omitted on encode; any padding (or none) is accepted on decode.
#[repr(transparent)]
#[derive(Debug, Clone, Default, SerializeDisplay, DeserializeFromStr, PartialEq, Eq, Hash)]
pub struct Base64Bytes(pub Bytes);

impl Display for Base64Bytes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Base64Display::new(&self.0, &BASE64_STANDARD_WHATEVER_PAD).fmt(f)
	}
}
impl FromStr for Base64Bytes {
	type Err = base64::DecodeError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Base64Bytes(Bytes::from(BASE64_STANDARD_WHATEVER_PAD.decode(s)?)))
	}
}

impl AsRef<Bytes> for Base64Bytes {
	fn as_ref(&self) -> &Bytes {
		&self.0
	}
}
impl Borrow<Bytes> for Base64Bytes {
	fn borrow(&self) -> &Bytes {
		&self.0
	}
}
impl AsRef<[u8]> for Base64Bytes {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl Borrow<[u8]> for Base64Bytes {
	fn borrow(&self) -> &[u8] {
		&self.0
	}
}
impl Deref for Base64Bytes {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Mutable byte buffer that serializes as a base64 string.
///
/// Padding is omitted on encode; any padding (or none) is accepted on decode.
#[repr(transparent)]
#[derive(Debug, Clone, Default, SerializeDisplay, DeserializeFromStr, PartialEq, Eq, Hash)]
pub struct Base64BytesMut(pub BytesMut);

impl Display for Base64BytesMut {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Base64Display::new(&self.0, &BASE64_STANDARD_WHATEVER_PAD).fmt(f)
	}
}
impl FromStr for Base64BytesMut {
	type Err = base64::DecodeError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Base64BytesMut(BytesMut::from(Bytes::from(
			BASE64_STANDARD_WHATEVER_PAD.decode(s)?,
		))))
	}
}

impl AsRef<BytesMut> for Base64BytesMut {
	fn as_ref(&self) -> &BytesMut {
		&self.0
	}
}
impl Borrow<BytesMut> for Base64BytesMut {
	fn borrow(&self) -> &BytesMut {
		&self.0
	}
}
impl AsRef<[u8]> for Base64BytesMut {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl AsMut<[u8]> for Base64BytesMut {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}
impl Borrow<[u8]> for Base64BytesMut {
	fn borrow(&self) -> &[u8] {
		&self.0
	}
}
impl BorrowMut<[u8]> for Base64BytesMut {
	fn borrow_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}
impl Deref for Base64BytesMut {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for Base64BytesMut {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

/// Immutable byte buffer that serializes as a url-safe base64 string.
///
/// Padding is omitted on encode; any padding (or none) is accepted on decode.
#[repr(transparent)]
#[derive(Debug, Clone, Default, SerializeDisplay, DeserializeFromStr, PartialEq, Eq, Hash)]
pub struct Base64UrlBytes(pub Bytes);

impl Display for Base64UrlBytes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Base64Display::new(&self.0, &BASE64_URL_WHATEVER_PAD).fmt(f)
	}
}
impl FromStr for Base64UrlBytes {
	type Err = base64::DecodeError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Base64UrlBytes(Bytes::from(BASE64_URL_WHATEVER_PAD.decode(s)?)))
	}
}

impl AsRef<Bytes> for Base64UrlBytes {
	fn as_ref(&self) -> &Bytes {
		&self.0
	}
}
impl Borrow<Bytes> for Base64UrlBytes {
	fn borrow(&self) -> &Bytes {
		&self.0
	}
}
impl AsRef<[u8]> for Base64UrlBytes {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl Borrow<[u8]> for Base64UrlBytes {
	fn borrow(&self) -> &[u8] {
		&self.0
	}
}
impl Deref for Base64UrlBytes {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Mutable byte buffer that serializes as a url-safe base64 string.
///
/// Padding is omitted on encode; any padding (or none) is accepted on decode.
#[repr(transparent)]
#[derive(Debug, Clone, Default, SerializeDisplay, DeserializeFromStr, PartialEq, Eq, Hash)]
pub struct Base64UrlBytesMut(pub BytesMut);

impl Display for Base64UrlBytesMut {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Base64Display::new(&self.0, &BASE64_URL_WHATEVER_PAD).fmt(f)
	}
}
impl FromStr for Base64UrlBytesMut {
	type Err = base64::DecodeError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Base64UrlBytesMut(BytesMut::from(Bytes::from(
			BASE64_URL_WHATEVER_PAD.decode(s)?,
		))))
	}
}

impl AsRef<BytesMut> for Base64UrlBytesMut {
	fn as_ref(&self) -> &BytesMut {
		&self.0
	}
}
impl Borrow<BytesMut> for Base64UrlBytesMut {
	fn borrow(&self) -> &BytesMut {
		&self.0
	}
}
impl AsRef<[u8]> for Base64UrlBytesMut {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl AsMut<[u8]> for Base64UrlBytesMut {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}
impl Borrow<[u8]> for Base64UrlBytesMut {
	fn borrow(&self) -> &[u8] {
		&self.0
	}
}
impl BorrowMut<[u8]> for Base64UrlBytesMut {
	fn borrow_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}
impl Deref for Base64UrlBytesMut {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for Base64UrlBytesMut {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
