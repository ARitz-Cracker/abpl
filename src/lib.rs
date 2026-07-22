//! # ABPL: Aritz's BoilerPlate Library
//! _A collection of junk that I only want to write once_
//!
//! Featuring:
//! - [abpl::app::service_main]: all you need to write a reloadable systemd service
//! - [abpl::app::axum::HotReloadingAxumService]: an axum service that can listen to multiple sockets while having
//!   zero-downtime reloads
//! - [abpl::Error]: an error macro that empowers you to create convenient-to-use error types that are typed and serializable
//!   (I feel like I'm underselling this)
//! - [abpl::future::block_on]: because OS threads are easier to debug during runtime but some widely-used crates needs the
//!   tokio runtime anyway
//! - Various newtypes
//! - And more!

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
