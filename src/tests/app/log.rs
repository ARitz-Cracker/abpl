use tracing_subscriber::EnvFilter;

use super::{ReloadableLocalLogHandle, setup_reloadable_fmt_layer};

// `setup_local_logging`/`setup_reloadable_journald_layer` aren't exercised here: the former calls
// `tracing_subscriber::registry().with(layer).init()`, which sets the process-wide global
// default subscriber -- something that can only happen once per process and would silently
// break every other test's `tracing::*!` calls for the rest of the `cargo test` run. The latter
// needs a live journald socket, which isn't guaranteed to exist in every environment this runs
// in. Both are thin wiring around `setup_reloadable_fmt_layer`/`ReloadableLocalLogHandle`, which
// *are* covered below without either external dependency.

#[test]
fn fmt_layer_is_built_with_the_initial_filter() {
	let (_layer, handle) = setup_reloadable_fmt_layer(EnvFilter::new("info"));
	let filter_str = handle.with_current(|layer| layer.filter().to_string()).unwrap();
	assert_eq!(filter_str, "info");
}

#[test]
fn replace_filter_swaps_the_fmt_layers_filter() {
	let (_layer, handle) = setup_reloadable_fmt_layer(EnvFilter::new("info"));
	let wrapped = ReloadableLocalLogHandle::Fmt(handle.clone());

	wrapped.replace_filter(EnvFilter::new("debug"));

	let filter_str = handle.with_current(|layer| layer.filter().to_string()).unwrap();
	assert_eq!(filter_str, "debug");
}
