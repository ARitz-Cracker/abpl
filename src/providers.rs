#[cfg(feature = "std")]
use std::process::ExitCode;

#[cfg(feature = "http")]
pub trait ProvidesHttpStatus {
	fn http_status(&self) -> http::StatusCode;
}

#[cfg(feature = "std")]
pub trait ProvidesExitCode {
	fn exit_code(&self) -> ExitCode;
}

// `EX_*` constants live in `crate::app::consts`, which requires the `app` feature too.
#[cfg(all(feature = "app", feature = "std"))]
impl ProvidesExitCode for std::io::Error {
	fn exit_code(&self) -> ExitCode {
		use crate::app::consts::*;
		use std::io::ErrorKind;
		match self.kind() {
			// Filesystem: read side
			ErrorKind::NotFound => EX_NOINPUT,
			// Not fs-specific per sysexits.h's own guidance, but io::Error can't tell us
			// whether this was a read, a write, or something else -- NOPERM is the least
			// wrong generic choice.
			ErrorKind::PermissionDenied => EX_NOPERM,
			ErrorKind::NotADirectory | ErrorKind::IsADirectory => EX_USAGE,
			ErrorKind::InvalidFilename => EX_DATAERR,
			ErrorKind::StaleNetworkFileHandle => EX_IOERR,

			// Filesystem: write side -- all "can't produce the output you asked for"
			ErrorKind::AlreadyExists
			| ErrorKind::DirectoryNotEmpty
			| ErrorKind::ReadOnlyFilesystem
			| ErrorKind::StorageFull
			| ErrorKind::QuotaExceeded
			| ErrorKind::FileTooLarge
			| ErrorKind::ExecutableFileBusy => EX_CANTCREAT,

			// OS-level resource limits, not really "our" fault or the user's data
			ErrorKind::AddrInUse | ErrorKind::TooManyLinks | ErrorKind::ArgumentListTooLong => EX_OSERR,
			// Likely transient -- worth a retry
			ErrorKind::ResourceBusy
			| ErrorKind::ConnectionReset
			| ErrorKind::ConnectionAborted
			| ErrorKind::TimedOut
			| ErrorKind::WouldBlock
			| ErrorKind::Interrupted => EX_TEMPFAIL,
			ErrorKind::WriteZero | ErrorKind::BrokenPipe => EX_IOERR,

			// Network: host reachable in principle, but not currently answering/routable
			ErrorKind::ConnectionRefused | ErrorKind::NetworkDown => EX_UNAVAILABLE,
			ErrorKind::HostUnreachable | ErrorKind::NetworkUnreachable | ErrorKind::AddrNotAvailable => EX_NOHOST,

			// Bad/malformed data, as opposed to a usage mistake
			ErrorKind::InvalidInput | ErrorKind::InvalidData | ErrorKind::UnexpectedEof => EX_DATAERR,

			// Programming/logic errors -- misuse of the API rather than an environmental problem
			ErrorKind::NotConnected | ErrorKind::NotSeekable | ErrorKind::Deadlock | ErrorKind::CrossesDevices => {
				EX_SOFTWARE
			},

			// Not supported here/now, but not necessarily wrong to have tried
			ErrorKind::Unsupported => EX_UNAVAILABLE,
			// OOM is an environment/OS-resource problem, not a bug in this program
			ErrorKind::OutOfMemory => EX_OSERR,

			_ => EX_IOERR,
		}
		.into()
	}
}

// `http::StatusCode` requires the `http` feature (and this impl needs `std::io::Error` to exist).
#[cfg(all(feature = "http", feature = "std"))]
impl ProvidesHttpStatus for std::io::Error {
	fn http_status(&self) -> http::StatusCode {
		use http::StatusCode;
		use std::io::ErrorKind;
		match self.kind() {
			// The client's request/input is the problem
			ErrorKind::NotFound => StatusCode::NOT_FOUND,
			ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
			ErrorKind::AlreadyExists | ErrorKind::DirectoryNotEmpty => StatusCode::CONFLICT,
			ErrorKind::NotADirectory
			| ErrorKind::IsADirectory
			| ErrorKind::InvalidFilename
			| ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
			ErrorKind::InvalidData | ErrorKind::UnexpectedEof => StatusCode::UNPROCESSABLE_ENTITY,
			ErrorKind::FileTooLarge => StatusCode::PAYLOAD_TOO_LARGE,

			// Our own storage/resources are the problem, not any upstream dependency
			ErrorKind::ReadOnlyFilesystem | ErrorKind::StorageFull | ErrorKind::QuotaExceeded => {
				StatusCode::INSUFFICIENT_STORAGE
			},
			ErrorKind::Unsupported => StatusCode::NOT_IMPLEMENTED,

			// Likely transient, or some dependency (a socket, a remote host, a DB connection --
			// this impl has no way to know which) isn't currently reachable/responsive. Not
			// BAD_GATEWAY/GATEWAY_TIMEOUT: those specifically mean "acting as a gateway/proxy,
			// got a bad/no response from the inbound server" per RFC 9110, which asserts a role
			// in HTTP request routing that a generic io::Error has no business claiming.
			ErrorKind::ResourceBusy
			| ErrorKind::ExecutableFileBusy
			| ErrorKind::Interrupted
			| ErrorKind::ConnectionRefused
			| ErrorKind::ConnectionReset
			| ErrorKind::ConnectionAborted
			| ErrorKind::HostUnreachable
			| ErrorKind::NetworkUnreachable
			| ErrorKind::NetworkDown
			| ErrorKind::AddrNotAvailable
			| ErrorKind::TimedOut => StatusCode::SERVICE_UNAVAILABLE,

			// Everything else (logic errors, OS resource limits, OOM, ...) is some internal
			// fault rather than something the client or an upstream dependency caused. HTTP's
			// status codes are much coarser than sysexits' here, so this catch-all covers a lot
			// more ground than `ProvidesExitCode`'s does.
			_ => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}

#[cfg(test)]
#[path = "tests/providers.rs"]
mod tests;
