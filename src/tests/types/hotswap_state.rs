use std::sync::Arc;

use super::HotswapState;

#[test]
fn new_parent_reports_is_parent() {
	let state = HotswapState::new(42);
	assert!(state.is_parent());
	assert_eq!(*state.clone_as_child().as_inner_ref(), 42);
}

#[test]
fn replace_parent_state_returns_old_value_and_child_sees_new_value() {
	let parent = HotswapState::new(1);
	let child_before = parent.clone_as_child();
	assert_eq!(*child_before.as_inner_ref(), 1);

	let old = parent.replace_parent_state(2).expect("parent should return the old state");
	assert_eq!(*old, 1);

	// A pre-existing child is unaffected by the swap (it holds its own `Arc` snapshot).
	assert_eq!(*child_before.as_inner_ref(), 1);
	// But a fresh child taken after the swap observes the new value.
	assert_eq!(*parent.clone_as_child().as_inner_ref(), 2);
}

#[test]
fn replace_parent_state_on_a_child_is_a_noop_returning_none() {
	let parent = HotswapState::new(1);
	let child = parent.clone_as_child();
	assert!(child.replace_parent_state(99).is_none());
	// The parent (and therefore future children) is untouched.
	assert_eq!(*parent.clone_as_child().as_inner_ref(), 1);
}

#[test]
fn clone_as_parent_from_a_parent_shares_the_same_lock() {
	let parent = HotswapState::new(1);
	let shared = parent.clone_as_parent();
	assert!(shared.is_parent());

	parent.replace_parent_state(2);
	// `shared` is the *same* parent (shared `Arc<RwLock<..>>`), so it sees the update too.
	assert_eq!(*shared.clone_as_child().as_inner_ref(), 2);
}

#[test]
fn clone_as_parent_from_a_child_creates_an_independent_parent() {
	let original_parent = HotswapState::new(1);
	let child = original_parent.clone_as_child();
	let new_parent = child.clone_as_parent();
	assert!(new_parent.is_parent());

	// Updating the new parent must not affect the original parent's state.
	new_parent.replace_parent_state(2);
	assert_eq!(*original_parent.clone_as_child().as_inner_ref(), 1);
	assert_eq!(*new_parent.clone_as_child().as_inner_ref(), 2);
}

#[test]
fn clone_is_equivalent_to_clone_as_parent_or_child_depending_on_variant() {
	let parent = HotswapState::new(1);
	let parent_clone = parent.clone();
	assert!(parent_clone.is_parent());
	parent.replace_parent_state(2);
	assert_eq!(*parent_clone.clone_as_child().as_inner_ref(), 2);

	let child = parent.clone_as_child();
	let child_clone = child.clone();
	assert!(!child_clone.is_parent());
	assert_eq!(*child_clone.as_inner_ref(), 2);
}

#[test]
fn into_inner_unwraps_a_child_directly_and_snapshots_a_parent() {
	let parent = HotswapState::new(1);
	let child = parent.clone_as_child();
	assert_eq!(*child.into_inner(), 1);

	// Calling it on a parent takes a snapshot of the current value rather than panicking.
	assert_eq!(*parent.into_inner(), 1);
}

#[test]
#[should_panic(expected = "was called on a parent")]
fn as_inner_ref_panics_on_a_parent() {
	let parent = HotswapState::new(1);
	let _ = parent.as_inner_ref();
}

#[test]
fn try_as_inner_ref_is_the_non_panicking_counterpart() {
	let parent = HotswapState::new(1);
	assert!(parent.try_as_inner_ref().is_none());
	assert_eq!(*parent.clone_as_child().try_as_inner_ref().unwrap(), 1);
}

#[test]
fn from_arc_makes_a_parent_and_into_arc_round_trips() {
	let arc = Arc::new(7);
	let state: HotswapState<i32> = arc.into();
	assert!(state.is_parent());
	let back: Arc<i32> = state.into();
	assert_eq!(*back, 7);
}

#[test]
fn arc_from_ref_never_disturbs_the_original() {
	let parent = HotswapState::new(7);
	let arc: Arc<i32> = Arc::from(&parent);
	assert_eq!(*arc, 7);
	// `&parent` conversion must go through `clone_as_child`, i.e. never mutate/consume `parent`.
	assert!(parent.is_parent());
}
