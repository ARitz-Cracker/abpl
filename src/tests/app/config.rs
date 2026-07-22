use std::path::PathBuf;

use super::{ParseTomlFileErrorKind, parse_external_toml_file, parse_toml_file};

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Sample {
	name: String,
	count: u32,
}

#[test]
fn path_not_specified_when_none_given() {
	let err = parse_toml_file::<Sample, PathBuf>(None).unwrap_err();
	assert!(matches!(err.kind(), ParseTomlFileErrorKind::PathNotSpecified));
}

#[test]
fn io_error_when_the_path_does_not_exist() {
	let dir = tempfile::tempdir().unwrap();
	let missing = dir.path().join("does-not-exist.toml");
	let err = parse_toml_file::<Sample, _>(Some(missing.clone())).unwrap_err();
	assert!(matches!(err.kind(), ParseTomlFileErrorKind::Io { path } if *path == missing));
}

#[test]
fn invalid_utf8_when_file_contents_are_not_utf8() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("bad-utf8.toml");
	std::fs::write(&path, [0xFF, 0xFE, 0xFD]).unwrap();
	let err = parse_toml_file::<Sample, _>(Some(path.clone())).unwrap_err();
	assert!(matches!(err.kind(), ParseTomlFileErrorKind::InvalidUtf8 { path: p } if *p == path));
}

#[test]
fn invalid_schema_when_toml_does_not_match_the_target_type() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("bad-schema.toml");
	// Missing the required `name` field.
	std::fs::write(&path, "count = 3\n").unwrap();
	let err = parse_toml_file::<Sample, _>(Some(path.clone())).unwrap_err();
	assert!(matches!(err.kind(), ParseTomlFileErrorKind::InvalidSchema { path: p } if *p == path));
}

#[test]
fn parses_successfully_when_everything_lines_up() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("good.toml");
	std::fs::write(&path, "name = \"abc\"\ncount = 3\n").unwrap();
	let parsed: Sample = parse_toml_file(Some(path)).unwrap();
	assert_eq!(
		parsed,
		Sample {
			name: "abc".to_string(),
			count: 3
		}
	);
}

#[derive(Debug, serde::Deserialize)]
struct Wrapper {
	#[serde(deserialize_with = "parse_external_toml_file")]
	secrets: Sample,
}

#[test]
fn external_toml_file_accepts_an_inline_value() {
	let wrapper: Wrapper = toml::from_str("[secrets]\nname = \"inline\"\ncount = 1\n").unwrap();
	assert_eq!(wrapper.secrets.name, "inline");
	assert_eq!(wrapper.secrets.count, 1);
}

#[test]
fn external_toml_file_follows_a_path_to_another_file() {
	let dir = tempfile::tempdir().unwrap();
	let secrets_path = dir.path().join("secrets.toml");
	std::fs::write(&secrets_path, "name = \"external\"\ncount = 2\n").unwrap();

	let document = format!("secrets = {:?}\n", secrets_path.display().to_string());
	let wrapper: Wrapper = toml::from_str(&document).unwrap();
	assert_eq!(wrapper.secrets.name, "external");
	assert_eq!(wrapper.secrets.count, 2);
}

#[test]
fn external_toml_file_reports_a_missing_referenced_file() {
	let dir = tempfile::tempdir().unwrap();
	let missing = dir.path().join("does-not-exist.toml");
	let document = format!("secrets = {:?}\n", missing.display().to_string());
	let err = toml::from_str::<Wrapper>(&document).unwrap_err();
	// The io error's own message (path + reason) surfaces directly in the custom serde error.
	assert!(err.to_string().contains(&missing.display().to_string()) || err.to_string().to_lowercase().contains("os error"));
}

#[test]
fn external_toml_file_reports_invalid_utf8_in_the_referenced_file() {
	let dir = tempfile::tempdir().unwrap();
	let secrets_path = dir.path().join("bad-utf8.toml");
	std::fs::write(&secrets_path, [0xFF, 0xFE, 0xFD]).unwrap();
	let document = format!("secrets = {:?}\n", secrets_path.display().to_string());
	let err = toml::from_str::<Wrapper>(&document).unwrap_err();
	assert!(err.to_string().contains("file referenced has"));
}

#[test]
fn external_toml_file_reports_an_invalid_schema_in_the_referenced_file() {
	let dir = tempfile::tempdir().unwrap();
	let secrets_path = dir.path().join("bad-schema.toml");
	// Missing the required `name` field.
	std::fs::write(&secrets_path, "count = 3\n").unwrap();
	let document = format!("secrets = {:?}\n", secrets_path.display().to_string());
	let err = toml::from_str::<Wrapper>(&document).unwrap_err();
	assert!(err.to_string().to_lowercase().contains("missing field"));
}
