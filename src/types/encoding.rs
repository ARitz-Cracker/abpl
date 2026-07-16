#[cfg(feature = "newtype_base64")]
mod base64;
#[cfg(feature = "newtype_base64")]
pub use base64::{BASE64_STANDARD_WHATEVER_PAD, Base64Bytes, Base64BytesMut, Base64UrlBytes, Base64UrlBytesMut};
