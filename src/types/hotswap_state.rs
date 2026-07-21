use std::sync::{Arc, RwLock};

use crate::sync::MithridatistLockResult as _;

/// Think of this as a fancy `Arc<T>`, and can be converted to/from an `Arc<T>`.
///
/// The primary purpose of this struct is to allow for new axum requests to function with new state without needing to
/// wait for the old requests to finish, thus allowing for hot-reloads with minimal downtime
///
/// This has 2 states
/// - `Parent`: Internally an `Arc<RwLock<Arc<T>>>`. The inner value can be replaced using the `replace_parent_state`
///   method. This is the mechanism used to allow for hot-reloading of an Axum state.
/// - `Child`: Internally an `Arc<T>` cloned from the parent's `RwLock`.
#[derive(Debug)]
pub enum HotswapState<T: Send + Sync + 'static> {
	Parent(Arc<RwLock<Arc<T>>>),
	Child(Arc<T>),
}
impl<T: Send + Sync + 'static> HotswapState<T> {
	/// Creates a new parent state container
	pub fn new(state: T) -> Self {
		Self::from(Arc::new(state))
	}
	pub fn is_parent(&self) -> bool {
		matches!(self, Self::Parent(_))
	}
	/// Replaces the parent state, returning the old state. Returns `None` if this was a child
	pub fn replace_parent_state(&self, new_state: T) -> Option<Arc<T>> {
		match self {
			Self::Parent(inner_lock) => Some(std::mem::replace(
				&mut inner_lock.write().mithridate(),
				Arc::new(new_state),
			)),
			Self::Child(_) => None,
		}
	}
	/// If `self` is a parent, then this will cloned the shared reference to the parent. If `self` is a child, this
	/// create a parent using `self`'s inner value.
	pub fn clone_as_parent(&self) -> Self {
		match self {
			Self::Parent(inner_lock) => Self::Parent(inner_lock.clone()),
			Self::Child(inner) => Self::Parent(Arc::new(RwLock::new(inner.clone()))),
		}
	}
	/// Creates a clone of `self` which is always a child. That is, calling `replace_parent_state` will have no effect
	/// on the the clone.
	pub fn clone_as_child(&self) -> Self {
		match self {
			Self::Parent(inner_lock) => Self::Child(inner_lock.read().mithridate().clone()),
			Self::Child(inner) => Self::Child(inner.clone()),
		}
	}
	/// If `self` is a parent, then this will clone the inner-most `Arc<T>`, behaving similarly to `.clone_as_child()`. If
	/// `self` is a child, then it simply moves the inner `Arc<T>` out of `self`.
	pub fn into_inner(self) -> Arc<T> {
		match self {
			HotswapState::Parent(inner_lock) => inner_lock.read().mithridate().clone(),
			HotswapState::Child(inner) => inner,
		}
	}
	/// This will panic if `self` is a parent. To ensure this isn't the case, you can either call `.clone_as_child()` or `.into_inner()`
	pub fn as_inner_ref(&self) -> &T {
		match self {
			HotswapState::Parent(_) => panic!("HotswapState::is_inner_ref was called on a parent."),
			HotswapState::Child(inner) => inner.as_ref(),
		}
	}
	/// Returns a reference to the inner object if `self` is a child
	pub fn try_as_inner_ref(&self) -> Option<&T> {
		match self {
			HotswapState::Parent(_) => None,
			HotswapState::Child(inner) => Some(inner.as_ref()),
		}
	}
}
impl<T: Send + Sync + 'static> Clone for HotswapState<T> {
	fn clone(&self) -> Self {
		match self {
			Self::Parent(inner_lock) => Self::Parent(inner_lock.clone()),
			Self::Child(inner) => Self::Child(inner.clone()),
		}
	}
}
impl<T: Send + Sync + 'static> From<Arc<T>> for HotswapState<T> {
	fn from(value: Arc<T>) -> Self {
		Self::Parent(Arc::new(RwLock::new(value)))
	}
}
impl<T: Send + Sync + 'static> From<HotswapState<T>> for Arc<T> {
	fn from(value: HotswapState<T>) -> Self {
		value.into_inner()
	}
}
impl<T: Send + Sync + 'static> From<&HotswapState<T>> for Arc<T> {
	fn from(value: &HotswapState<T>) -> Self {
		value.clone_as_child().into_inner()
	}
}

#[cfg(feature = "http")]
impl<T: Send + Sync + 'static> axum::extract::FromRef<HotswapState<T>> for Arc<T> {
	// This is where the magic happens, the "clones" of parents are always the child, and axum always "clones" the
	// state, which means we can replace the state in the parent, allowing future requests to have the new state
	// without needing to re-start axum or wait until current requests are complete before replacing.

	// This is ultimately the point of all this. You can write axum listeners that extract a `State(Arc<T>)` while not
	// needing to worry about hot-swapping reloads
	fn from_ref(input: &HotswapState<T>) -> Self {
		input.clone_as_child().into_inner()
	}
}
