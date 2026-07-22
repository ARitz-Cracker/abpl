use std::time::Duration;

// `can_block_current_tokio`'s answer is memoized per-OS-thread forever (a `thread_local`
// `OnceCell`), and libtest reuses worker threads across multiple `#[test]` functions. Every
// scenario below that cares about that memoization runs on a freshly spawned `std::thread` so a
// stale answer left behind by some other test on a reused thread can never leak in.

#[test]
fn block_on_outside_a_tokio_runtime_uses_the_reusable_runtime() {
	std::thread::spawn(|| {
		assert_eq!(super::block_on(async { 1 + 1 }), 2);
	})
	.join()
	.unwrap();
}

#[test]
fn runtime_handle_outside_a_tokio_runtime_returns_a_usable_handle() {
	std::thread::spawn(|| {
		let handle = super::runtime_handle();
		assert_eq!(handle.block_on(async { 40 + 2 }), 42);
	})
	.join()
	.unwrap();
}

#[test]
fn runtime_handle_inside_a_tokio_runtime_returns_the_ambient_handle() {
	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();
	rt.block_on(async {
		// If `runtime_handle()` mistakenly built/returned the separate `REUSABLE_RT` instead of
		// the ambient ("current") ones, this would hang: `REUSABLE_RT` is a current-thread
		// runtime that only drives spawned tasks while something is blocked inside its own
		// `block_on`, and nothing here ever calls that. The timeout turns "wrong runtime" into a
		// clean failure instead of an indefinite hang.
		let (tx, rx) = tokio::sync::oneshot::channel();
		super::runtime_handle().spawn(async move {
			let _ = tx.send(());
		});
		tokio::time::timeout(Duration::from_secs(2), rx)
			.await
			.expect("task spawned via runtime_handle() should run on the ambient runtime")
			.unwrap();
	});
}

#[test]
fn block_on_inside_a_spawn_blocking_thread_is_allowed() {
	// `spawn_blocking` always uses a dedicated blocking thread pool regardless of the async
	// flavor, so a current-thread runtime is enough here (the `rt-multi-thread` tokio feature
	// isn't enabled for this crate).
	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();
	rt.block_on(async {
		tokio::task::spawn_blocking(|| {
			// We're on a dedicated blocking-pool thread here, which tokio permits nested
			// `block_on` calls from -- this is exactly the case `can_block_current_tokio` is
			// meant to detect as "safe".
			assert_eq!(super::block_on(async { 1 + 1 }), 2);
		})
		.await
		.unwrap();
	});
}

#[test]
#[should_panic(expected = "gone too deep")]
fn block_on_inside_a_plain_async_task_panics() {
	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();
	rt.block_on(async {
		// Directly inside an async task on a runtime's own worker thread (not a spawn_blocking
		// thread), nested `block_on` is exactly the case tokio itself refuses -- our probe
		// should detect that and turn it into our own clearer panic message.
		super::block_on(async { 1 + 1 });
	});
}

#[test]
fn block_on_mt_outside_a_tokio_runtime_uses_the_reusable_runtime() {
	std::thread::spawn(|| {
		assert_eq!(super::block_on_mt(async { 1 + 1 }), 2);
	})
	.join()
	.unwrap();
}

#[test]
fn block_on_mt_inside_a_spawn_blocking_thread_uses_the_ambient_runtime() {
	// `spawn_blocking` always uses a dedicated blocking thread pool regardless of the async
	// flavor, so a current-thread runtime is enough here (the `rt-multi-thread` tokio feature
	// isn't enabled for this crate).
	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();
	rt.block_on(async {
		tokio::task::spawn_blocking(|| {
			assert_eq!(super::block_on_mt(async { 1 + 1 }), 2);
		})
		.await
		.unwrap();
	});
}

#[test]
fn block_on_mt_inside_a_plain_async_task_falls_back_to_a_dedicated_thread() {
	// Unlike plain `block_on`, this must *not* panic: `block_on_mt` is explicitly the "even
	// works when nested" variant, at the cost of spawning a whole new OS thread + runtime for
	// the duration of the call.
	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();
	rt.block_on(async {
		assert_eq!(super::block_on_mt(async { 1 + 1 }), 2);
	});
}
