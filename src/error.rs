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
			#[cfg(feature = "std")]
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
#[path = "tests/error.rs"]
mod tests;
