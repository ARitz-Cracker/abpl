// `ProvidesExitCode`/`ProvidesHttpStatus` for `std::io::Error` are big lookup tables mapping
// `ErrorKind` -> a constant. Exhaustively asserting every arm here would just be restating the
// same table a second time (identical code, no real coverage value) -- these spot-check a
// representative sample from each category in the match instead.

#[cfg(all(feature = "app", feature = "std"))]
#[test]
fn io_error_exit_code() {
	use super::ProvidesExitCode;

	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::NotFound).exit_code(),
		crate::app::consts::EX_NOINPUT.into()
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::PermissionDenied).exit_code(),
		crate::app::consts::EX_NOPERM.into()
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::ConnectionRefused).exit_code(),
		crate::app::consts::EX_UNAVAILABLE.into()
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::InvalidData).exit_code(),
		crate::app::consts::EX_DATAERR.into()
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::OutOfMemory).exit_code(),
		crate::app::consts::EX_OSERR.into()
	);
	// Anything not explicitly matched falls back to `EX_IOERR`.
	assert_eq!(
		std::io::Error::other("something unmapped").exit_code(),
		crate::app::consts::EX_IOERR.into()
	);
}

#[cfg(all(feature = "http", feature = "std"))]
#[test]
fn io_error_http_status() {
	use super::ProvidesHttpStatus;

	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::NotFound).http_status(),
		http::StatusCode::NOT_FOUND
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::TimedOut).http_status(),
		http::StatusCode::SERVICE_UNAVAILABLE
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::StorageFull).http_status(),
		http::StatusCode::INSUFFICIENT_STORAGE
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::Unsupported).http_status(),
		http::StatusCode::NOT_IMPLEMENTED
	);
	assert_eq!(
		std::io::Error::from(std::io::ErrorKind::OutOfMemory).http_status(),
		http::StatusCode::INTERNAL_SERVER_ERROR
	);
}
