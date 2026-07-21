use serde::{Deserialize, Deserializer, de::DeserializeOwned};

use crate::{app::consts, providers::ProvidesExitCode};
use std::{fmt::Display, path::PathBuf, process::ExitCode, string::FromUtf8Error};

#[derive(Debug, Clone, PartialEq, Eq, abpl::Error)]
#[abpl_provider(ProvidesExitCode(1.into(), exit_code, ExitCode))]
pub enum ParseTomlFileErrorKind {
	#[abpl_provider(exit_code(consts::EX_USAGE.into()))]
	PathNotSpecified,
	#[cause(std::io::Error)]
	#[abpl_provider(exit_code(cause))]
	Io { path: PathBuf },
	#[cause(FromUtf8Error)]
	#[abpl_provider(exit_code(consts::EX_CONFIG.into()))]
	InvalidUtf8 { path: PathBuf },
	#[cause(toml::de::Error)]
	#[abpl_provider(exit_code(consts::EX_CONFIG.into()))]
	InvalidSchema { path: PathBuf },
}
impl Display for ParseTomlFileErrorKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::PathNotSpecified => f.write_str("no file path was specified"),
			Self::Io { path } => f.write_fmt(format_args!("{path:?} could not be opened")),
			Self::InvalidUtf8 { path } => f.write_fmt(format_args!("{path:?} did not contain valid utf8")),
			Self::InvalidSchema { path } => f.write_fmt(format_args!(
				"{path:?} did not conform to the expected format or schema"
			)),
		}
	}
}

pub fn parse_toml_file<T: DeserializeOwned, P: Into<PathBuf>>(path: Option<P>) -> Result<T, ParseTomlFileError> {
	let Some(path) = path else { return Err(ParseTomlFileError::path_not_specified()) };
	let path = path.into();
	let file_contents = std::fs::read(&path).map_err_io_with(|_| path.clone())?;
	let file_contents = String::from_utf8(file_contents).map_err_invalid_utf_8_with(|_| path.clone())?;
	toml::from_str(&file_contents).map_err_invalid_schema(path)
}

/// So you can do `#[serde(deserialize_with = "parse_external_toml_file")]`. This could be used to have a config file
/// which allows you to optionally store secrets file elsewhere. Such is the norm with NixOS, or other situations where
/// you don't want commit secrets with deployment tooling. Note that all string-like items will be assumed to be a
/// path, so you should only really use this with a map-like or list-like structure.
pub fn parse_external_toml_file<'de, D: Deserializer<'de>, T: DeserializeOwned>(
	deserializer: D,
) -> Result<T, D::Error> {
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum MaybeT<T> {
		Path(PathBuf),
		T(T),
	}
	match MaybeT::<T>::deserialize(deserializer)? {
		MaybeT::Path(path) => {
			let file_contents = std::fs::read(&path).map_err(serde::de::Error::custom)?;
			let file_contents = String::from_utf8(file_contents)
				.map_err(|err| serde::de::Error::custom(format!("file referenced has {err}")))?;
			// This should show the nested toml parsing errors somewhat gracefully, they'll be shown the line showing
			// that the path is bad, then showing why the toml file is bad immediately after. Mildly annoying that
			// custom serde errors only have strings, but w/e.
			toml::from_str(&file_contents).map_err(serde::de::Error::custom)
		},
		MaybeT::T(inner) => Ok(inner),
	}
}
