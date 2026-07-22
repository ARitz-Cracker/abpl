use super::MaybeBorrowed;

#[test]
fn owned_and_borrowed_serialize_identically() {
	let owned = MaybeBorrowed::Owned(42u32);
	let value = 42u32;
	let borrowed = MaybeBorrowed::Borrowed(&value);

	let owned_json = serde_json::to_string(&owned).unwrap();
	let borrowed_json = serde_json::to_string(&borrowed).unwrap();
	assert_eq!(owned_json, "42");
	assert_eq!(owned_json, borrowed_json);
}

#[test]
fn deserialize_always_produces_owned() {
	let deserialized: MaybeBorrowed<'static, u32> = serde_json::from_str("42").unwrap();
	assert!(matches!(deserialized, MaybeBorrowed::Owned(42)));
}

#[test]
fn borrowed_can_outlive_a_shorter_deserialize_lifetime() {
	// `Deserialize<'de>` is implemented generically over the struct's own `'a`, not fixed to
	// `'static` -- this compiling at all (for a `'a` shorter than `'de`) is the point of the test.
	fn round_trip<'a, T: serde::Serialize + serde::de::DeserializeOwned>(value: &MaybeBorrowed<'a, T>) -> T {
		let json = serde_json::to_string(value).unwrap();
		serde_json::from_str(&json).unwrap()
	}
	let value = String::from("hi");
	let borrowed: MaybeBorrowed<'_, String> = MaybeBorrowed::Borrowed(&value);
	assert_eq!(round_trip(&borrowed), "hi");
}
