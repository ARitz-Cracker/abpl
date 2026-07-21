pub mod encoding;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "std")]
mod maybe_borrowed;
#[cfg(feature = "std")]
pub use maybe_borrowed::MaybeBorrowed;
#[cfg(feature = "std")]
pub mod hotswap_state;
