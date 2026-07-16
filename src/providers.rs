#[cfg(feature = "std")]
use std::process::ExitCode;

#[cfg(feature = "http")]
pub trait ProvidesHttpStatus {
	fn http_status(&self) -> http::StatusCode;
}

#[cfg(all(feature = "app", feature = "std"))]
pub trait ProvidesExitCode {
	fn exit_code(&self) -> ExitCode;
}
