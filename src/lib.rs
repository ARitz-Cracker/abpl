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
//!
//! # Cargo features
//! Almost everything here is opt-in -- enable only what you need.
//!
//! - `std` (**default**): the standard library. `#![no_std]` support is aspirational, not
//!   functional -- disabling this today does not currently build (a few spots reach for
//!   `alloc`-provided items like `String`/`Box`/`format!` without actually importing them from
//!   `alloc`). Treat `std` as required until this note goes away.
//! - `serde`: `Serialize`/`Deserialize` for the error macro's generated types and other newtypes.
//! - `utoipa`: [utoipa::ToSchema] for the same types, for OpenAPI schema generation.
//! - `derive_error`: the [abpl::Error] derive macro itself, plus its `Display`/serde support.
//! - `app`: [abpl::app] -- the reloadable-systemd-service lifecycle helper (config parsing,
//!   logging setup, signal handling).
//! - `http`: axum/tokio integration -- [abpl::app::axum::HotReloadingAxumService] and the
//!   [abpl::types::http] socket abstractions. Implies `future_util`.
//! - `future_util`: [abpl::future::block_on]/[abpl::future::block_on_mt], the tokio-runtime
//!   bridging helpers.
//! - `thread`: [abpl::thread] -- currently just a cancelable sleep.
//! - `newtype_base64`: base64-encoded byte-array newtypes. Implies `serde`.
//!
//! Run the test suite with `--all-features` rather than enabling individual features --
//! that's what the test suite is written against.

#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
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
