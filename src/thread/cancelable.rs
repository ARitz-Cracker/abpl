use std::{
	sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
	thread::{sleep, yield_now},
	time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Sleeper {
	rx: Receiver<()>,
}
impl Sleeper {
	/// Sleeps the current thread until the deadline, or until `SleeperHandle.cancel` was called on another thread.
	pub fn sleep_until(&self, deadline: Instant) {
		let mut now = Instant::now();
		if deadline <= now {
			yield_now();
			return;
		}
		// fixme: use recv_deadline and sleep_until after those have been stabilized
		if self.rx.recv_timeout(deadline.duration_since(now)) == Err(RecvTimeoutError::Disconnected) {
			// If no handlers exist, we might have some more sleeping to do
			now = Instant::now();
			if deadline > now {
				sleep(deadline.duration_since(now));
			}
		}
	}

	/// Sleeps the current thread for the specified duration, or until `SleeperHandle.cancel` was called on another thread.
	pub fn sleep(&self, dur: Duration) {
		let now = Instant::now();
		let Some(deadline) = now.checked_add(dur) else {
			sleep(dur);
			return;
		};
		self.sleep_until(deadline);
	}
}

#[derive(Debug, Clone)]
pub struct SleeperHandle {
	tx: SyncSender<()>,
}
impl SleeperHandle {
	/// Cancels the tread is sleeping via `Sleeper`. If no thread currently is using the `Sleeper`, then this is a
	/// noop. Returns `false` if the corresponding `Sleeper` is gone.
	pub fn cancel(&self) -> bool {
		self.tx.try_send(()) != Err(TrySendError::Disconnected(()))
	}
}

pub fn cancelable_sleep() -> (SleeperHandle, Sleeper) {
	let (tx, rx) = sync_channel(0);
	(SleeperHandle { tx }, Sleeper { rx })
}

#[cfg(test)]
#[path = "../tests/thread/cancelable.rs"]
mod tests;
