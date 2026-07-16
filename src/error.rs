extern crate alloc; // so nostd works
use core::{
	fmt::{Display, Formatter, Result as FmtResult},
	panic::Location,
};

use crate::maybe_std::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(
	feature = "serde",
	derive(serde::Serialize, serde::Deserialize),
	serde(rename_all = "camelCase")
)]
/// An error which has no canonical JSON representation
pub struct UnserializableError {
	/// Human-readable (hopefully) description of the error
	pub error_message: String,
	/// Inner-representation of the error - this is usually Rust's "debug" formatting
	pub error_kind: String,
}
impl Display for UnserializableError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		self.error_message.fmt(f)
	}
}
impl core::error::Error for UnserializableError {}
impl UnserializableError {
	pub fn from_error<E: core::error::Error>(error: &E) -> Self {
		Self {
			error_message: format!("{error}"),
			error_kind: format!("{error:?}"),
		}
	}
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde_with::SerializeDisplay))]

pub enum ErrorTrace {
	#[default]
	None,
	Location(&'static Location<'static>),
	#[cfg(feature = "std")]
	Backtrace(Rc<std::backtrace::Backtrace>),
	Erased(Rc<str>),
}
impl ErrorTrace {
	/// If this is an std environment, this calls `std::backtrace::Backtrace::capture()`
	///
	/// If this is a nostd environment, this calls the same as `ErrorTrace::new_location`
	#[track_caller]
	pub fn new_backtrace() -> Self {
		#[cfg(feature = "std")]
		{
			Self::Backtrace(Rc::new(std::backtrace::Backtrace::capture()))
		}
		#[cfg(not(feature = "std"))]
		{
			Self::Location(core::panic::Location::caller())
		}
	}

	/// Uses `core::panic::Location::caller()` see its documentation for more info
	#[track_caller]
	pub fn new_location() -> Self {
		Self::Location(core::panic::Location::caller())
	}
}
impl Display for ErrorTrace {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			ErrorTrace::None => f.write_str("\n"),
			ErrorTrace::Location(location) => {
				f.write_str(" @")?;
				location.fmt(f)?;
				f.write_str("\n")?;
				Ok(())
			},
			ErrorTrace::Backtrace(backtrace) => {
				// backtrace ends with a newline
				backtrace.fmt(f)
			},
			ErrorTrace::Erased(str) => {
				str.fmt(f)?;
				if str.ends_with('\n') { Ok(()) } else { f.write_str("\n") }
			},
		}
	}
}
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ErrorTrace {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(ErrorTrace::Erased(Rc::<str>::deserialize(deserializer)?))
	}
}

#[cfg(feature = "utoipa")]
impl utoipa::PartialSchema for ErrorTrace {
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
		String::schema()
	}
}

#[cfg(feature = "utoipa")]
impl utoipa::ToSchema for ErrorTrace {
	fn name() -> std::borrow::Cow<'static, str> {
		String::name()
	}
}

#[cfg(test)]
mod tests {
	use crate::{error::ErrorTrace, providers::ProvidesExitCode};

	// Whether or not my thesis of a duel parsable/generic error is even sound
	#[test]
	#[allow(unused_variables)]
	fn sanity_check() {
		#[derive(Debug, Clone, abpl_macros::Error, ::serde::Serialize, ::serde::Deserialize, utoipa::ToSchema)]
		#[abpl_error(location, serialize)]
		#[abpl_provider(ProvidesExitCode(1.into(), exit_code, std::process::ExitCode))]

		enum TestKind {
			#[cause(std::io::Error, unserializable)]
			One,
			#[cause(std::num::TryFromIntError, unserializable)]
			#[cause(std::num::ParseIntError, unserializable)]
			Two(u32, u32),
			#[cause(std::num::TryFromIntError, unserializable)]
			#[abpl_provider(exit_code(10.into()))]
			Three { a: u64, b: u64 },
		}

		#[derive(Debug, serde::Serialize, serde::Deserialize)]
		#[serde(untagged)]
		enum TestTwo {
			Something {
				a: u64,
			},
			#[serde(skip_serializing)]
			Nothing(serde::de::IgnoredAny),
		}
		dbg!(serde_json::to_string(&TestTwo::Something { a: 42 }));
		dbg!(serde_json::to_string(&TestTwo::Nothing(serde::de::IgnoredAny)));
		dbg!(serde_json::from_value::<TestTwo>(serde_json::json!({"b": 42})));

		let _a: Test = std::io::Error::from_raw_os_error(22).into();

		_a.kind();

		//a.exit_code();
		fn _test() -> Result<(), Test> {
			let _ = std::fs::read("asdf")?;
			let _ = u32::try_from(69u64).map_err_two(69, 420)?;
			let _ = u32::from_str_radix("fhf", 10).map_err_two(69, 420)?;
			// let _ = u32::try_from(69u64).map_err_three(69, 420)?;

			Ok(())
		}
	}
}
