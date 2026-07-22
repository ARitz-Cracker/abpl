use std::process::{ExitCode, Termination};

use super::MainResult;

// `service_main`'s signal-handling loop (real `libc` signals, `sd_notify`, spawning threads) is
// deliberately left untested here -- it's OS-interaction plumbing, not logic. `MainResult` is the
// pure part underneath it (`Termination::report`'s Ok/Err -> ExitCode mapping), and is covered
// below.

#[derive(Debug)]
struct TestError;
impl std::fmt::Display for TestError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("test error")
	}
}
impl std::error::Error for TestError {}
impl crate::providers::ProvidesExitCode for TestError {
	fn exit_code(&self) -> ExitCode {
		ExitCode::from(7)
	}
}

#[test]
fn ok_reports_success() {
	let result: Result<(), TestError> = Ok(());
	let main_result: MainResult<TestError> = result.into();
	assert_eq!(main_result.report(), ExitCode::SUCCESS);
}

#[test]
fn err_reports_the_errors_own_exit_code() {
	let result: Result<(), TestError> = Err(TestError);
	let main_result: MainResult<TestError> = result.into();
	assert_eq!(main_result.report(), ExitCode::from(7));
}
