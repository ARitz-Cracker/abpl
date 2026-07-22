use super::HotReloadAxumError;
use crate::providers::ProvidesExitCode;

// `HotReloadingAxumService::bind_sockets` and friends actually spawn OS threads, dedicated tokio
// runtimes, and bind real sockets to serve real axum requests -- that's the OS-interaction shell
// left untested here. `HotReloadAxumError` is the one piece of pure logic in this file, and is
// covered below.

#[test]
fn display_and_exit_code() {
	let err = HotReloadAxumError {};
	assert_eq!(err.to_string(), "no sockets could be listened to");
	assert_eq!(err.exit_code(), crate::app::consts::EX_CONFIG.into());
}
