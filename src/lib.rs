#![cfg_attr(not(feature = "std"), no_std)]

extern crate self as abpl; // Makes the abpl_macros work internally.
#[cfg(feature = "derive_error")]
pub use abpl_macros::Error;
pub mod error;

#[cfg(all(feature = "app", feature = "std"))]
pub mod app;
pub mod providers;
#[cfg(feature = "std")]
pub mod sync;
#[cfg(feature = "thread")]
pub mod thread;
pub mod types;

pub mod maybe_std;

#[cfg(feature = "derive_error")]
pub mod deps {
	pub use indenter;
}

#[cfg(feature = "future_util")]
pub mod future;
