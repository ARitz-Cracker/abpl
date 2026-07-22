use std::{
	sync::mpsc::sync_channel,
	time::{Duration, Instant},
};

#[test]
pub fn primitive_sanity_check() {
	// This is just to confirm that `sync_channel(0)`  won't block if it's actively another thread is actively
	// waiting/listening.
	let (tx, rx) = sync_channel::<()>(0);
	std::thread::spawn(move || tx.send(()));
	std::thread::sleep(Duration::from_secs(1));
	assert_eq!(rx.try_recv(), Ok(()));

	let (tx, rx) = sync_channel::<()>(0);
	std::thread::spawn(move || {
		let _ = rx.recv();
	});
	std::thread::sleep(Duration::from_secs(1));
	assert_eq!(tx.try_send(()), Ok(()));
}

#[test]
fn cancel_wakes_sleep() {
	let (handle, sleeper) = super::cancelable_sleep();
	std::thread::spawn(move || {
		std::thread::sleep(Duration::from_millis(50));
		handle.cancel();
	});
	let start = Instant::now();
	sleeper.sleep(Duration::from_secs(10));
	assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn sleep_until_past_deadline_returns_immediately() {
	let (_handle, sleeper) = super::cancelable_sleep();
	let start = Instant::now();
	sleeper.sleep_until(start - Duration::from_secs(1));
	assert!(start.elapsed() < Duration::from_millis(100));
}

#[test]
fn cancel_before_sleep_is_still_observed() {
	// `cancel` only reports `false` on disconnect -- a rendezvous channel with no receiver
	// currently blocked in `recv` fails `try_send` with `Full`, not `Disconnected`, so this is
	// still a "true" (the `Sleeper` just hadn't started listening yet, that's not the same as
	// it being gone).
	let (handle, sleeper) = super::cancelable_sleep();
	assert!(handle.cancel());

	let start = Instant::now();
	sleeper.sleep(Duration::from_millis(50));
	assert!(start.elapsed() >= Duration::from_millis(50));
}

#[test]
fn cancel_returns_false_once_sleeper_is_dropped() {
	let (handle, sleeper) = super::cancelable_sleep();
	drop(sleeper);
	assert!(!handle.cancel());
}

#[test]
fn handle_is_clone() {
	let (handle, sleeper) = super::cancelable_sleep();
	let handle2 = handle.clone();
	std::thread::spawn(move || {
		std::thread::sleep(Duration::from_millis(50));
		handle2.cancel();
	});
	let start = Instant::now();
	sleeper.sleep(Duration::from_secs(10));
	assert!(start.elapsed() < Duration::from_secs(1));
	// The original handle is still usable after the clone fired.
	drop(handle);
}
