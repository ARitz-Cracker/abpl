use std::{
	env::args_os,
	error::Error,
	io::{IsTerminal as _, stdin},
	process::{ExitCode, Termination},
};

use libc::{SIGALRM, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGUSR1, SIGUSR2};
use serde::de::DeserializeOwned;

#[cfg(target_os = "linux")]
use sd_notify::NotifyState;
use signal_hook::iterator::Signals;

use crate::{
	app::{
		config::{ParseTomlFileError, parse_toml_file},
		log::ProvidesEnvFilter,
	},
	providers::ProvidesExitCode,
};

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
		self.error
			.map(|error| {
				eprintln!("{error:-}");
				error.exit_code()
			})
			.unwrap_or(ExitCode::SUCCESS)
	}
}

#[cfg(feature = "http")]
pub mod axum;
pub mod config;
pub mod consts;
pub mod log;

pub trait ReloadableService: Sized {
	const INTERVAL_SECONDS: libc::c_uint = 5;

	type Error: core::error::Error + ProvidesExitCode + From<ParseTomlFileError>;
	type Config: DeserializeOwned + ProvidesEnvFilter;

	/// Called after the config and secret files have been parsed. This is only called once.
	///
	/// If this returns an Err, then [service_main] will immediately return.
	fn start(config: Self::Config) -> Result<Self, Self::Error>;

	/// Called every time a reload is requested, and the re-parsing of the config succeeds.
	/// The reload will be considered finished once this function returns. Returning an Err will log it, not return
	/// [service_main], so you must ensure that your service continues as if a reload didn't happen should this
	/// function fail. That said, the new [ProvidesEnvFilter::log_filter] will still be applied.
	///
	/// This usually invoked a SIGHUP happens while not running in a TTY, which is usually happens when
	/// `systemctl reload your-service.service`
	fn reload(&mut self, config: Self::Config) -> Result<(), Self::Error>;

	/// Called when the process received SIGUSR1.
	fn sigusr1(&mut self) {}

	/// Called when the process received SIGUSR2.
	fn sigusr2(&mut self) {}

	/// Called on the regular interval set by [Self::INTERVAL_SECONDS]. Returning an error here will cause the service to be
	/// stopped without calling [Self::stop].
	fn interval(&mut self) -> Result<(), Self::Error> {
		Ok(())
	}

	/// Called when the process receives `SIGTERM`, `SIGQUIT`, `SIGINT`, or `SIGHUP` (non-tty)
	///
	/// A result is in the signature for convenient non-zero-exit-code reasons
	fn stop(self) -> Result<(), Self::Error>;
}

/*
	Some personal notes if/when I ship software for windows.
	- The signal-hook crate I'm using has limited support for windows, not 100% sure what that is entails
	- Windows does not have signals, but it does have events that you can listen to, those include...
		- https://learn.microsoft.com/en-us/windows/win32/winmsg/window-notifications (and others)
			- iirc this works because services run on their own virtual desktopm and therefore have... windows...
		- https://learn.microsoft.com/en-us/windows/console/setconsolectrlhandler?redirectedfrom=MSDN
	- Any service I'd ship would likely be ran under NSSM
		- NSSM has a multi-stage approach of send ctrl-c, then send window close event, then force-kill
		- it might be unmaintained? the alternative would be to learn this
		  https://learn.microsoft.com/en-us/windows/win32/system-services
		- Actually, Microsoft provides https://crates.io/crates/windows-services , no NSSM needed.
	- Windows doesn't seem to have the capability to "reload" a service
	- I'd like the option to output tracing messages to the windows event log, similar how I can send structured logs
	  to systemd now
		- it has 5 log levels, but they're different. Critical < Error < Warning < Info < Verbose. The entire thing
		  has its "seriousness" level translated 1 tick upwards.
*/

/// Allows you to write the following
/// ```ignore
/// pub fn main() -> MainResult<YourError> {
///     abpl::app::service_main::<YourService>().into()
/// }
/// ```
///
/// This piece of magic:
/// - Assumes the first cli argument is a path to [toml] file containing your [ReloadableService::Config].
/// - Sets up [tracing] for you, using [tracing_journald] if available, [mod@tracing_subscriber::fmt] if not.
/// - If this service started via systemd, then it will assume a
///   [service type](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#Type=) of
///   `notify-reload`; Informing systemd when the service has finished starting and reloading.
/// - Calls [alarm(2)](https://www.man7.org/linux/man-pages/man2/alarm.2.html) with the duration specified in
///   [ReloadableService::INTERVAL_SECONDS]
/// - Listens to the following signals. `SIGALRM`, `SIGHUP`, `SIGINT`, `SIGTERM`, `SIGQUIT`, `SIGUSR1`, and `SIGUSR2`.
///   - on `SIGHUP` and stdin is not a tty:
///     - Your config file will be re-read, the log filter will be updated, and [ReloadableService::reload] will be
///       called.
///   - on `SIGINT`, `SIGTERM`, `SIGHUP` (if std is a tty), or `SIGQUIT`:
///     - [ReloadableService::stop] will be called, afterwhich this function returns.
///   - on `SIGALRM`:
///     - [ReloadableService::interval] is called, [alarm(2)](https://www.man7.org/linux/man-pages/man2/alarm.2.html)
///       is called again with the duration specified in [ReloadableService::INTERVAL_SECONDS]
///   - on `SIGUSR1`:
///     - [ReloadableService::sigusr1] is called.
///   - on `SIGUSR2`:
///     - [ReloadableService::sigusr2] is called.
///
pub fn service_main<T: ReloadableService>() -> Result<(), T::Error> {
	// The error won't look as nice, but a panic here is very unlikely.
	// Setting up early also allows us to go through the startup
	let mut signals = Signals::new([SIGALRM, SIGHUP, SIGINT, SIGTERM, SIGQUIT]).expect("set up signal hooks");
	let config_file_path = args_os().nth(1);
	let first_config = parse_toml_file(config_file_path.clone())?;

	#[cfg(target_os = "linux")]
	let running_systemd = sd_notify::notify(
		false,
		&[
			NotifyState::MainPid(std::process::id()),
			NotifyState::Status("starting..."),
		],
	)
	.is_ok();
	#[cfg(not(target_os = "linux"))]
	let running_systemd = false;

	let log_reload_handle = log::setup_local_logging(ProvidesEnvFilter::log_filter(&first_config), running_systemd);

	let mut service = T::start(first_config).inspect_err(|err| {
		#[cfg(target_os = "linux")]
		let _ = sd_notify::notify(false, &[NotifyState::Status(&format!("failed to start: {err:-}"))]);
	})?;
	#[cfg(target_os = "linux")]
	let _ = sd_notify::notify(false, &[NotifyState::Status("started!"), NotifyState::Ready]);

	// SAFTY: we expect that this function is not used in any other thread.
	unsafe { libc::alarm(T::INTERVAL_SECONDS) };
	for signal in signals.forever() {
		let is_terminal = stdin().is_terminal();
		match signal {
			SIGHUP if !is_terminal => {
				#[cfg(target_os = "linux")]
				let _ = NotifyState::monotonic_usec_now().and_then(|notify_curtime| {
					sd_notify::notify(
						false,
						&[
							NotifyState::Status("reloading..."),
							NotifyState::Reloading,
							notify_curtime,
						],
					)
				});

				if let Err(reload_err) = {
					|| -> Result<(), T::Error> {
						let new_config = parse_toml_file(config_file_path.clone())?;
						log_reload_handle.replace_filter(ProvidesEnvFilter::log_filter(&new_config));
						service.reload(new_config)?;
						Ok(())
					}
				}() {
					tracing::error!(
						error_details = format!("{reload_err:-#}"),
						"service failed to reload; but we're continuing anyway: {reload_err:-.3}"
					);
					#[cfg(target_os = "linux")]
					let _ = sd_notify::notify(
						false,
						&[
							NotifyState::Status(&format!("reload error: {reload_err:-}")),
							NotifyState::Ready,
						],
					);
				} else {
					#[cfg(target_os = "linux")]
					let _ = sd_notify::notify(false, &[NotifyState::Status("reloaded!"), NotifyState::Ready]);
				}
			},
			SIGINT | SIGTERM | SIGHUP | SIGQUIT => {
				#[cfg(target_os = "linux")]
				let _ = sd_notify::notify(false, &[NotifyState::Status("stopping..."), NotifyState::Stopping]);
				service.stop().inspect_err(|stop_err| {
					tracing::error!(
						error_details = format!("{stop_err:-#}"),
						"service returned on stop request: {stop_err:-.3}"
					);
					#[cfg(target_os = "linux")]
					let _ = sd_notify::notify(
						false,
						&[NotifyState::Status(&format!("error while stopping: {stop_err:-}"))],
					);
				})?;
				#[cfg(target_os = "linux")]
				let _ = sd_notify::notify(false, &[NotifyState::Status("stopped")]);
				break;
			},
			SIGALRM => {
				tracing::trace!("received SIGALRM");
				service.interval().inspect_err(|interval_err| {
					tracing::error!(
						error_details = format!("{interval_err:-#}"),
						"service returned error on interval: {interval_err:-.3}"
					);
					#[cfg(target_os = "linux")]
					let _ = sd_notify::notify(
						false,
						&[NotifyState::Status(&format!("error during interval: {interval_err:-}"))],
					);
				})?;
				// SAFTY: we expect that this function is not used in any other thread.
				unsafe { libc::alarm(T::INTERVAL_SECONDS) };
			},
			SIGUSR1 => {
				tracing::trace!("received SIGUSR1");
				service.sigusr1();
			},
			SIGUSR2 => {
				tracing::trace!("received SIGUSR2");
				service.sigusr2();
			},
			_ => unreachable!("signal {signal} should have been handled"),
		}
	}
	tracing::info!("service stopped");
	Ok(())
}

#[cfg(test)]
#[path = "tests/app.rs"]
mod tests;
