//pub trait mithridate

use std::sync::{LockResult, TryLockError, TryLockResult};

/// A trait auto-implemted on [LockResult] that implements a method that is slightly more convenient than
/// `.unwrap_or_else(|err| err.into_inner())`
pub trait MithridatistLockResult<T> {
	/// Assert that you are immune to poison.
	fn mithridate(self) -> T;
}
impl<T> MithridatistLockResult<T> for LockResult<T> {
	fn mithridate(self) -> T {
		match self {
			Ok(inner) => inner,
			Err(poison_error) => poison_error.into_inner(),
		}
	}
}

/// A trait auto-implemted on [TryLockResult] that implements a method that is slightly more convenient than
/// matching against it
pub trait MithridatistTryLockResult<T> {
	/// Assert that you are immune to poison.
	fn mithridate(self) -> Option<T>;
}
impl<T> MithridatistTryLockResult<T> for TryLockResult<T> {
	fn mithridate(self) -> Option<T> {
		match self {
			Ok(inner) => Some(inner),
			Err(TryLockError::Poisoned(poison_error)) => Some(poison_error.into_inner()),
			Err(TryLockError::WouldBlock) => None,
		}
	}
}

#[cfg(test)]
#[path = "tests/sync.rs"]
mod tests;
