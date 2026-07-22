use tracing_subscriber::{
	EnvFilter, Layer as _, Registry, filter::Filtered, fmt as tracing_fmt, layer::SubscriberExt,
	reload as tracing_reload, util::SubscriberInitExt,
};

pub trait ProvidesEnvFilter {
	/// Log filter for stdout/journald. [::serde_with::DisplayFromStr] would be the most convenient way to include this
	/// in your config file
	fn log_filter(&self) -> EnvFilter;
}

pub type FilteredFmtLayer = Filtered<tracing_fmt::Layer<Registry>, EnvFilter, Registry>;
pub type ReloadableFilteredFmtLayer = tracing_reload::Layer<FilteredFmtLayer, Registry>;
pub type ReloadableFilteredFmtLayerHandle = tracing_reload::Handle<FilteredFmtLayer, Registry>;

pub fn setup_reloadable_fmt_layer(
	initial_filter: EnvFilter,
) -> (ReloadableFilteredFmtLayer, ReloadableFilteredFmtLayerHandle) {
	let fmt_layer: FilteredFmtLayer = tracing_fmt::Layer::default().with_filter(initial_filter);
	ReloadableFilteredFmtLayer::new(fmt_layer)
}

pub type FilteredJournalDLayer = Filtered<tracing_journald::Layer, EnvFilter, Registry>;
pub type ReloadableFilteredJournalDLayer = tracing_reload::Layer<FilteredJournalDLayer, Registry>;
pub type ReloadableFilteredJournalDLayerHandle = tracing_reload::Handle<FilteredJournalDLayer, Registry>;

pub fn setup_reloadable_journald_layer(
	initial_filter: EnvFilter,
) -> Result<(ReloadableFilteredJournalDLayer, ReloadableFilteredJournalDLayerHandle), std::io::Error> {
	let journald_layer: FilteredJournalDLayer = tracing_journald::Layer::new()?.with_filter(initial_filter);
	Ok(ReloadableFilteredJournalDLayer::new(journald_layer))
}

pub enum ReloadableLocalLogHandle {
	Fmt(ReloadableFilteredFmtLayerHandle),
	JournalD(ReloadableFilteredJournalDLayerHandle),
}
impl ReloadableLocalLogHandle {
	pub fn replace_filter(&self, new_filter: EnvFilter) {
		match self {
			Self::Fmt(handle) => {
				// handle.modify only returns Err in 2 cases
				// 1. The subscriber is gone. This won't happen if we're using this for globally.
				// 2. Poison. Nothing we do here can result in a poisoned lock
				// Therefore, it should be safe to treat this as infallible.
				let _ = handle.modify(|layer| *layer.filter_mut() = new_filter);
			},
			Self::JournalD(handle) => {
				// same reasons to ignore the error as above
				let _ = handle.modify(|layer| *layer.filter_mut() = new_filter);
			},
		}
	}
}

/// Sets up tracing for either stdout or journald logging
pub fn setup_local_logging(initial_filter: EnvFilter, try_journald: bool) -> ReloadableLocalLogHandle {
	let mut journald_error = None;
	if try_journald {
		match setup_reloadable_journald_layer(initial_filter.clone()) {
			Ok((layer, handle)) => {
				tracing_subscriber::registry().with(layer).init();
				return ReloadableLocalLogHandle::JournalD(handle);
			},
			Err(err) => journald_error = Some(err),
		}
	}
	let (layer, handle) = setup_reloadable_fmt_layer(initial_filter);
	tracing_subscriber::registry().with(layer).init();
	if let Some(journald_error) = journald_error {
		tracing::error!("failed to connect to journald: {journald_error}");
	}
	ReloadableLocalLogHandle::Fmt(handle)
}

#[cfg(test)]
#[path = "../tests/app/log.rs"]
mod tests;
