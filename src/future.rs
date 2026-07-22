use std::{
	cell::{LazyCell, OnceCell},
	panic::catch_unwind,
};

thread_local! {
	static CAN_BLOCK: OnceCell<bool> = const { OnceCell::new() };
	static REUSABLE_RT: LazyCell<tokio::runtime::Runtime> = LazyCell::new(|| {
		tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.expect("tokio runtime")
	})
}

/// Returns true if it's safe to perform a blocking operation in the current context.
pub fn can_block_current_tokio() -> bool {
	CAN_BLOCK.with(|cell| {
		*cell.get_or_init(|| {
			// Try to call block_on. This panics we're already running within a tokio runtime.
			let result = catch_unwind(|| {
				tokio::runtime::Handle::current().block_on(async {
					tokio::task::yield_now().await;
				});
			});

			result.is_ok() // true = can block, false = cannot
		})
	})
}

fn block_on_with_thread<F>(fut: F) -> F::Output
where
	F: Future + Send,
	F::Output: Send,
{
	// Note, it _might_ be safe for the future to not implement `Send`, as this thread is always waiting for the scoped
	// thread to complete and doesn't touch the future or its output in the meantime, but it's not something I can be
	// 100% sure of rn.
	//
	// After some additional thinking, i don't think this can be done safely, what if a future relies on something
	// thread-local?
	std::thread::scope(|scope| {
		scope
			.spawn(move || block_on(fut))
			.join()
			.expect("inner-thread panicked")
	})
}

/// Like `futures::executor::block_on` but ensures that there is a tokio runtime on the current thread. The purpose of
/// this is to convert async calls to sync calls where appropriate.
pub fn block_on<F>(f: F) -> F::Output
where
	F: IntoFuture,
{
	let Ok(rt) = tokio::runtime::Handle::try_current() else {
		return REUSABLE_RT.with(|cell| cell.block_on(f.into_future()));
	};
	if can_block_current_tokio() {
		rt.block_on(f.into_future())
	} else {
		panic!("you've gone too deep with the nested block_on calls, use \"block_on_mt\" instead.")
	}
}

/// Returns the tokio runtime handle which is used for the `block_on` function exported by this library
pub fn runtime_handle() -> tokio::runtime::Handle {
	tokio::runtime::Handle::try_current().unwrap_or_else(|_| REUSABLE_RT.with(|v| v.handle().clone()))
}

/// Like `futures::executor::block_on` but ensures that there is a tokio runtime on the current thread. The purpose of
/// this is to flatten async calls to sync calls where appropriate. This also works with nested `block_on` calls, but
/// requires the future to implement Send, as it will pass to future to a new thread if the current executor is
/// currently blocked. Also, be mindful of https://github.com/tokio-rs/tokio/issues/7337 when using this function.
pub fn block_on_mt<F>(f: F) -> F::Output
where
	F: Future + Send,
	F::Output: Send,
{
	// We must use the tokio executor when possible, otherwise things like reqwest, or anything else that uses tokio's
	// async IO will hang forever. But there are situations where tokio won't let us use block_on
	// Requrements:
	// - If there is no runtime, start a current-thread one
	// - If there is a runtime, and...
	//   - We're in a spawn_blocking thread, use the handle's block_on method
	//   - We're in a "main" thread, use `block_on_with_thread`
	let Ok(rt) = tokio::runtime::Handle::try_current() else {
		return REUSABLE_RT.with(|cell| cell.block_on(f));
	};
	if can_block_current_tokio() {
		rt.block_on(f)
	} else {
		block_on_with_thread(f)
	}
}

#[cfg(test)]
#[path = "tests/future.rs"]
mod tests;
