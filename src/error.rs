extern crate alloc; // so nostd works
use core::{
	fmt::{Display, Formatter, Result as FmtResult, Write as _},
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
	/// The cause of this error, if available
	pub error_cause: Option<Box<UnserializableError>>,
}
impl Display for UnserializableError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		self.error_message.fmt(f)
	}
}
impl core::error::Error for UnserializableError {}

impl UnserializableError {
	pub fn from_error<E: core::error::Error + ?Sized>(error: &E) -> Self {
		Self {
			error_message: format!("{error}"),
			error_kind: format!("{error:?}"),
			error_cause: error
				.source()
				.map(|inner_error| Box::new(Self::from_error(inner_error))),
		}
	}
}

/// Renders the `Display` output of a macro-generated error struct, honoring format modifiers:
///
/// - `{}`: just the kind's own message.
/// - `{:-}` / `{:+}`: causal chain, forward (`∵`, root first) or reverse (`∴`, deepest first).
/// - `{:-.N}` / `{:+.N}`: same, limited to `N` entries deep via the precision modifier.
/// - `{:#}`: this error's own verbose block (message + trace), no chain expansion.
/// - `{:-#}` / `{:+#}`: verbose chain -- each hop renders itself via `{:#}` (falling back to a
///   plain one-liner if that hop's own `Display` ignores the alternate flag), indented under a
///   `∵`/`∴` marker. This never requires knowing the concrete type of a cause: it's just
///   `core::error::Error::source()` and re-entrant `Display`, so it composes across crate
///   boundaries for free.
#[cfg(feature = "derive_error")]
pub fn fmt_generated_error(
	f: &mut Formatter<'_>,
	message: &impl Display,
	trace: &ErrorTrace,
	cause: Option<&(dyn core::error::Error + 'static)>,
) -> FmtResult {
	let max_depth = f.precision().unwrap_or(usize::MAX);
	match (f.alternate(), f.sign_minus(), f.sign_plus()) {
		(true, true, _) => fmt_verbose_chain(f, message, trace, cause, max_depth, false),
		(true, false, true) => fmt_verbose_chain(f, message, trace, cause, max_depth, true),
		(true, false, false) => write_verbose_node(f, message, trace),
		(false, true, _) => fmt_text_chain(f, message, cause, max_depth, false),
		(false, false, true) => fmt_text_chain(f, message, cause, max_depth, true),
		(false, false, false) => write!(f, "{message}"),
	}
}

#[cfg(feature = "derive_error")]
fn write_verbose_node<W: core::fmt::Write>(w: &mut W, message: &dyn Display, trace: &ErrorTrace) -> core::fmt::Result {
	write!(w, "error: {message}\n{trace}")
}

#[cfg(feature = "derive_error")]
fn fmt_text_chain(
	f: &mut Formatter<'_>,
	message: &dyn Display,
	cause: Option<&(dyn core::error::Error + 'static)>,
	max_depth: usize,
	reverse: bool,
) -> FmtResult {
	let mut parts = alloc::vec![format!("{message}")];
	let mut current = cause;
	while parts.len() < max_depth {
		let Some(err) = current else { break };
		parts.push(format!("{err}"));
		current = err.source();
	}
	let separator = if reverse { " ∴ " } else { " ∵ " };
	let write_part = |f: &mut Formatter<'_>, i: usize, part: &str| -> FmtResult {
		if i > 0 {
			f.write_str(separator)?;
		}
		f.write_str(part)
	};
	if reverse {
		for (i, part) in parts.iter().rev().enumerate() {
			write_part(f, i, part)?;
		}
	} else {
		for (i, part) in parts.iter().enumerate() {
			write_part(f, i, part)?;
		}
	}
	Ok(())
}

#[cfg(feature = "derive_error")]
fn fmt_verbose_chain(
	f: &mut Formatter<'_>,
	message: &dyn Display,
	trace: &ErrorTrace,
	cause: Option<&(dyn core::error::Error + 'static)>,
	max_depth: usize,
	reverse: bool,
) -> FmtResult {
	// Trimmed because a hop's own `Display` (notably `ErrorTrace`'s) may end with its own
	// trailing newline; we want full control over the blank line before each `∵`/`∴` marker
	// rather than double up with whatever the hop already terminated itself with.
	let mut root = String::new();
	write_verbose_node(&mut root, message, trace)?;
	let mut blocks = alloc::vec![root.trim_end_matches('\n').to_string()];
	let mut current = cause;
	while blocks.len() < max_depth {
		let Some(err) = current else { break };
		blocks.push(format!("{err:#}").trim_end_matches('\n').to_string());
		current = err.source();
	}
	let marker = if reverse { "∴ " } else { "∵ " };
	let write_block = |f: &mut Formatter<'_>, i: usize, block: &str| -> FmtResult {
		if i == 0 {
			return f.write_str(block);
		}
		write!(f, "\n\n{marker}")?;
		// The marker already visually sets off the first line; only indent lines after it, so
		// the block doesn't get a stray indent right after the marker.
		match block.split_once('\n') {
			Some((first_line, rest)) => {
				f.write_str(first_line)?;
				f.write_char('\n')?;
				write!(indenter::indented(f), "{rest}")
			},
			None => f.write_str(block),
		}
	};
	if reverse {
		for (i, block) in blocks.iter().rev().enumerate() {
			write_block(f, i, block)?;
		}
	} else {
		for (i, block) in blocks.iter().enumerate() {
			write_block(f, i, block)?;
		}
	}
	Ok(())
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
	use crate::providers::ProvidesExitCode;

	// Whether or not my thesis of a duel parsable/generic error is even sound
	#[test]
	#[allow(unused_variables, dead_code)]
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
			Three {
				a: u64,
				b: u64,
			},
			Four {
				a: u64,
				b: u64,
			},
		}
		impl core::fmt::Display for TestKind {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					TestKind::One => f.write_str("the one"),
					TestKind::Two(a, b) => {
						f.write_str("the two numbers are")?;
						a.fmt(f)?;
						f.write_str(" + ")?;
						b.fmt(f)?;
						Ok(())
					},
					TestKind::Three { a, b } => {
						f.write_str("the three (two) numbers are")?;
						a.fmt(f)?;
						f.write_str(" + ")?;
						b.fmt(f)?;
						Ok(())
					},
					TestKind::Four { a, b } => {
						f.write_str("the four (two) numbers are")?;
						a.fmt(f)?;
						f.write_str(" + ")?;
						b.fmt(f)?;
						Ok(())
					},
				}
			}
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
		// dbg!(serde_json::to_string(&TestTwo::Something { a: 42 }));
		// dbg!(serde_json::to_string(&TestTwo::Nothing(serde::de::IgnoredAny)));
		// dbg!(serde_json::from_value::<TestTwo>(serde_json::json!({"b": 42})));

		let _a: Test = std::io::Error::from_raw_os_error(22).into();

		_a.kind();

		//a.exit_code();
		fn _test() -> Result<(), Test> {
			let _ = std::fs::read("asdf")?;
			let _ = u32::try_from(69u64).map_err_two(69, 420)?;
			let _ = "fhf".parse::<u32>().map_err_two(69, 420)?;

			// let _ = u32::try_from(69u64).map_err_three(69, 420)?;

			Err(Test::four(69, 420))
		}
	}

	#[test]
	fn serde_round_trip() {
		#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
		struct DeclaredCause {
			message: String,
		}
		impl std::fmt::Display for DeclaredCause {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.write_str(&self.message)
			}
		}
		impl std::error::Error for DeclaredCause {}

		#[derive(Debug, Clone, abpl_macros::Error, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
		#[abpl_error(serialize, deserialize, utoipa)]
		enum SerdeProbeKind {
			#[cause(std::io::Error, unserializable)]
			NoArgs,
			#[cause(DeclaredCause)]
			WithArgs { a: u64, b: u64 },
		}
		impl core::fmt::Display for SerdeProbeKind {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					Self::NoArgs => f.write_str("SerdeProbe with no args"),
					Self::WithArgs { a, b } => {
						f.write_str("SerdeProbe with args ")?;
						a.fmt(f)?;
						f.write_str(", ")?;
						b.fmt(f)?;
						Ok(())
					},
				}
			}
		}

		// utoipa schema generation shouldn't panic.
		let _ = <SerdeProbe as utoipa::PartialSchema>::schema();
		assert_eq!(<SerdeProbe as utoipa::ToSchema>::name(), "SerdeProbe");

		// An unserializable cause always round-trips through the erased representation.
		let with_erased_cause: SerdeProbe = std::io::Error::from_raw_os_error(22).into();
		let json = serde_json::to_string(&with_erased_cause).expect("serialize should succeed");
		// eprintln!("erased: {json}");
		let round_tripped: SerdeProbe = serde_json::from_str(&json).expect("deserialize should succeed");
		assert!(matches!(round_tripped.kind(), SerdeProbeKind::NoArgs));

		// A declared, serializable cause is preserved with full fidelity on the wire -- via a
		// borrowed downcast (`MaybeBorrowed::Borrowed`), not a clone -- rather than being erased.
		let with_declared_cause = SerdeProbe::new_with_cause(
			SerdeProbeKind::WithArgs { a: 1, b: 2 },
			Some(DeclaredCause {
				message: "boom".to_string(),
			}),
		);
		let json = serde_json::to_string(&with_declared_cause).expect("serialize should succeed");
		// eprintln!("declared cause (preserved, zero clone): {json}");
		// Preserved, not erased: the wire form is the declared type's own shape, not
		// UnserializableError's `errorMessage`/`errorKind` blob.
		assert!(json.contains(r#""errorCause":{"message":"boom"}"#));
		let round_tripped: SerdeProbe = serde_json::from_str(&json).expect("deserialize should succeed");
		assert!(matches!(round_tripped.kind(), SerdeProbeKind::WithArgs { a: 1, b: 2 }));

		// A payload that encodes the declared cause directly (e.g. written by hand, or
		// produced by another service) also deserializes into the concrete type, via the
		// untagged ordered-fallback matching on the cause enum.
		let handwritten = r#"{"errorMessage":"throwaway","errorTrace":"\n","errorKind":"withArgs","errorCause":{"message":"boom"},"errorDetail":{"a":1,"b":2}}"#;
		let round_tripped: SerdeProbe = serde_json::from_str(handwritten).expect("deserialize should succeed");
		assert!(matches!(round_tripped.kind(), SerdeProbeKind::WithArgs { a: 1, b: 2 }));

		// `SerdeProbe::new(kind)` (no cause at all) round-trips too.
		let no_cause = SerdeProbe::new(SerdeProbeKind::NoArgs);
		let json = serde_json::to_string(&no_cause).expect("serialize should succeed");
		let round_tripped: SerdeProbe = serde_json::from_str(&json).expect("deserialize should succeed");
		assert!(matches!(round_tripped.kind(), SerdeProbeKind::NoArgs));
	}

	#[test]
	fn display_format_modifiers() {
		#[derive(Debug, Clone, abpl_macros::Error)]
		#[abpl_error(backtrace)]
		enum PublishKind {
			Failed,
		}
		impl core::fmt::Display for PublishKind {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				f.write_str("publish failed")
			}
		}

		#[derive(Debug, Clone, abpl_macros::Error)]
		#[abpl_error(location)]
		enum AuthKind {
			#[cause(Token)]
			Failed,
		}
		impl core::fmt::Display for AuthKind {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				f.write_str("auth error")
			}
		}

		#[derive(Debug, Clone, abpl_macros::Error)]
		enum TokenKind {
			Expired,
		}
		impl core::fmt::Display for TokenKind {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				f.write_str("token expired")
			}
		}

		let token = Token::new(TokenKind::Expired);
		let auth = Auth::new_with_cause(AuthKind::Failed, Some(token));
		let publish = Publish::new_with_cause(PublishKind::Failed, Some(auth));

		// eprintln!("--- {{}} ---\n{publish}");
		// eprintln!("--- {{:-}} ---\n{publish:-}");
		// eprintln!("--- {{:+}} ---\n{publish:+}");
		// eprintln!("--- {{:-.2}} ---\n{publish:-.2}");
		// eprintln!("--- {{:+.2}} ---\n{publish:+.2}");
		// eprintln!("--- {{:#}} ---\n{publish:#}");
		// eprintln!("--- {{:-#}} ---\n{publish:-#}");
		// eprintln!("--- {{:+#}} ---\n{publish:+#}");

		assert_eq!(format!("{publish}"), "publish failed");
		assert_eq!(format!("{publish:-}"), "publish failed ∵ auth error ∵ token expired");
		assert_eq!(format!("{publish:+}"), "token expired ∴ auth error ∴ publish failed");
		assert_eq!(format!("{publish:-.2}"), "publish failed ∵ auth error");
		assert_eq!(format!("{publish:+.2}"), "auth error ∴ publish failed");
	}

	#[test]
	fn provider_cause_delegation() {
		#[derive(Debug, Clone, abpl_macros::Error)]
		#[abpl_provider(ProvidesExitCode(1.into(), exit_code, std::process::ExitCode))]
		enum InnerKind {
			Boom,
		}
		impl core::fmt::Display for InnerKind {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				f.write_str("inner boom")
			}
		}

		#[derive(Debug, Clone, abpl_macros::Error)]
		#[abpl_provider(ProvidesExitCode(2.into(), exit_code, std::process::ExitCode))]
		enum OuterKind {
			#[cause(Inner)]
			#[abpl_provider(exit_code(cause))]
			Wrapping,
			Standalone,
		}
		impl core::fmt::Display for OuterKind {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				f.write_str("outer wrapping")
			}
		}

		// Delegates to the cause's own `exit_code()` (1), not Outer's default (2).
		let wrapping_with_cause = Outer::new_with_cause(OuterKind::Wrapping, Some(Inner::new(InnerKind::Boom)));
		assert_eq!(wrapping_with_cause.exit_code(), 1.into());

		// No cause set at all -> falls back to Outer's own default (2).
		let wrapping_without_cause = Outer::new(OuterKind::Wrapping);
		assert_eq!(wrapping_without_cause.exit_code(), 2.into());

		// A variant that never opted into `cause` delegation just uses the default (2).
		let standalone = Outer::new(OuterKind::Standalone);
		assert_eq!(standalone.exit_code(), 2.into());
	}

	#[test]
	fn map_err_with_is_lazy_and_receives_the_cause() {
		#[derive(Debug, Clone, abpl_macros::Error)]
		enum WithKind {
			#[cause(std::num::TryFromIntError)]
			#[cause(std::num::ParseIntError)]
			Two(u32, u32),
			#[cause(std::num::TryFromIntError)]
			One(u32),
		}
		impl core::fmt::Display for WithKind {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				write!(f, "{self:?}")
			}
		}

		// The closure must never run on the `Ok` path.
		let mut invoked = false;
		let ok: Result<u32, With> = Ok::<u32, std::num::TryFromIntError>(1u32).map_err_two_with(|_cause| {
			invoked = true;
			(0, 0)
		});
		assert!(ok.is_ok());
		assert!(!invoked, "closure ran even though the Result was Ok");

		// Multi-field variant: closure returns a tuple, and receives the actual cause.
		let try_from_cause = u32::try_from(-1i64 as u64).unwrap_err();
		let expected_len = try_from_cause.to_string().len() as u32;
		let err = Err::<u32, _>(try_from_cause)
			.map_err_two_with(|cause| (cause.to_string().len() as u32, 42))
			.unwrap_err();
		assert!(matches!(err.kind(), WithKind::Two(a, 42) if *a == expected_len));

		// Same variant, different declared cause type -- same trait, different `Self::Cause`.
		let parse_cause = "not a number".parse::<u32>().unwrap_err();
		let expected_len = parse_cause.to_string().len() as u32;
		let err = Err::<u32, _>(parse_cause)
			.map_err_two_with(|cause: &std::num::ParseIntError| (cause.to_string().len() as u32, 7))
			.unwrap_err();
		assert!(matches!(err.kind(), WithKind::Two(a, 7) if *a == expected_len));

		// Single-field variant: closure returns the bare value, not a 1-tuple.
		let err = u32::try_from(-1i64 as u64).map_err_one_with(|_cause| 99).unwrap_err();
		assert!(matches!(err.kind(), WithKind::One(99)));
	}
}
