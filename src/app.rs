use std::{
	error::Error,
	process::{ExitCode, Termination},
};

use crate::providers::ProvidesExitCode;

pub struct MainResult<E: ProvidesExitCode + Error> {
	error: Option<E>,
}

impl<E: ProvidesExitCode + Error> From<Result<(), E>> for MainResult<E> {
	fn from(value: Result<(), E>) -> Self {
		Self { error: value.err() }
	}
}

impl<E: ProvidesExitCode + Error> Termination for MainResult<E> {
	fn report(self) -> ExitCode {
		// todo, write the stuff
		self.error.map(|error| error.exit_code()).unwrap_or(ExitCode::SUCCESS)
	}
}
