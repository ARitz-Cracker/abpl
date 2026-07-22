use std::sync::{Mutex, RwLock};

use super::{MithridatistLockResult as _, MithridatistTryLockResult as _};

#[test]
fn mithridate_passes_through_a_healthy_lock() {
	let mutex = Mutex::new(42);
	assert_eq!(*mutex.lock().mithridate(), 42);
}

#[test]
fn mithridate_recovers_a_poisoned_mutex() {
	let mutex = Mutex::new(0);
	let _ = std::thread::scope(|scope| {
		scope
			.spawn(|| {
				let _guard = mutex.lock().unwrap();
				panic!("poisoning on purpose");
			})
			.join()
	});
	assert!(mutex.is_poisoned());
	// `mithridate` shrugs off the poison and hands back the guard anyway.
	assert_eq!(*mutex.lock().mithridate(), 0);
}

#[test]
fn mithridate_recovers_a_poisoned_rwlock() {
	let lock = RwLock::new(0);
	let _ = std::thread::scope(|scope| {
		scope
			.spawn(|| {
				let _guard = lock.write().unwrap();
				panic!("poisoning on purpose");
			})
			.join()
	});
	assert!(lock.is_poisoned());
	assert_eq!(*lock.read().mithridate(), 0);
}

#[test]
fn try_mithridate_passes_through_a_healthy_lock() {
	let mutex = Mutex::new(42);
	assert_eq!(mutex.try_lock().mithridate().as_deref(), Some(&42));
}

#[test]
fn try_mithridate_recovers_a_poisoned_lock() {
	let mutex = Mutex::new(0);
	let _ = std::thread::scope(|scope| {
		scope
			.spawn(|| {
				let _guard = mutex.lock().unwrap();
				panic!("poisoning on purpose");
			})
			.join()
	});
	assert!(mutex.is_poisoned());
	assert_eq!(mutex.try_lock().mithridate().as_deref(), Some(&0));
}

#[test]
fn try_mithridate_returns_none_when_contended() {
	let mutex = Mutex::new(0);
	let _guard = mutex.lock().unwrap();
	// The lock is held by `_guard` on this same thread, so `try_lock` observes `WouldBlock`.
	assert!(mutex.try_lock().mithridate().is_none());
}
