use crate::providers::ProvidesExitCode;

// Whether or not my thesis of a duel parsable/generic error is even sound
#[test]
#[allow(unused_variables, dead_code)]
fn sanity_check() {
	#[derive(Debug, Clone, abpl_macros::Error, ::serde::Serialize, ::serde::Deserialize, utoipa::ToSchema)]
	#[abpl_error(location, serialize)]
	#[abpl_provider(ProvidesExitCode(1.into(), exit_code, std::process::ExitCode))]
	enum TestKind {
		#[cause(std::io::Error, unserializable)]
		One,
		#[cause(std::num::TryFromIntError, unserializable)]
		#[cause(std::num::ParseIntError, unserializable)]
		Two(u32, u32),
		#[cause(std::num::TryFromIntError, unserializable)]
		#[abpl_provider(exit_code(10.into()))]
		Three {
			a: u64,
			b: u64,
		},
		Four {
			a: u64,
			b: u64,
		},
	}
	impl core::fmt::Display for TestKind {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			match self {
				TestKind::One => f.write_str("the one"),
				TestKind::Two(a, b) => {
					f.write_str("the two numbers are")?;
					a.fmt(f)?;
					f.write_str(" + ")?;
					b.fmt(f)?;
					Ok(())
				},
				TestKind::Three { a, b } => {
					f.write_str("the three (two) numbers are")?;
					a.fmt(f)?;
					f.write_str(" + ")?;
					b.fmt(f)?;
					Ok(())
				},
				TestKind::Four { a, b } => {
					f.write_str("the four (two) numbers are")?;
					a.fmt(f)?;
					f.write_str(" + ")?;
					b.fmt(f)?;
					Ok(())
				},
			}
		}
	}

	#[derive(Debug, serde::Serialize, serde::Deserialize)]
	#[serde(untagged)]
	enum TestTwo {
		Something {
			a: u64,
		},
		#[serde(skip_serializing)]
		Nothing(serde::de::IgnoredAny),
	}
	// dbg!(serde_json::to_string(&TestTwo::Something { a: 42 }));
	// dbg!(serde_json::to_string(&TestTwo::Nothing(serde::de::IgnoredAny)));
	// dbg!(serde_json::from_value::<TestTwo>(serde_json::json!({"b": 42})));

	let _a: Test = std::io::Error::from_raw_os_error(22).into();

	_a.kind();

	//a.exit_code();
	fn _test() -> Result<(), Test> {
		let _ = std::fs::read("asdf")?;
		let _ = u32::try_from(69u64).map_err_two(69, 420)?;
		let _ = "fhf".parse::<u32>().map_err_two(69, 420)?;

		// let _ = u32::try_from(69u64).map_err_three(69, 420)?;

		Err(Test::four(69, 420))
	}
}

#[test]
fn serde_round_trip() {
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
	struct DeclaredCause {
		message: String,
	}
	impl std::fmt::Display for DeclaredCause {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			f.write_str(&self.message)
		}
	}
	impl std::error::Error for DeclaredCause {}

	#[derive(Debug, Clone, abpl_macros::Error, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
	#[abpl_error(serialize, deserialize, utoipa)]
	enum SerdeProbeKind {
		#[cause(std::io::Error, unserializable)]
		NoArgs,
		#[cause(DeclaredCause)]
		WithArgs { a: u64, b: u64 },
	}
	impl core::fmt::Display for SerdeProbeKind {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			match self {
				Self::NoArgs => f.write_str("SerdeProbe with no args"),
				Self::WithArgs { a, b } => {
					f.write_str("SerdeProbe with args ")?;
					a.fmt(f)?;
					f.write_str(", ")?;
					b.fmt(f)?;
					Ok(())
				},
			}
		}
	}

	// utoipa schema generation shouldn't panic.
	let _ = <SerdeProbe as utoipa::PartialSchema>::schema();
	assert_eq!(<SerdeProbe as utoipa::ToSchema>::name(), "SerdeProbe");

	// An unserializable cause always round-trips through the erased representation.
	let with_erased_cause: SerdeProbe = std::io::Error::from_raw_os_error(22).into();
	let json = serde_json::to_string(&with_erased_cause).expect("serialize should succeed");
	// eprintln!("erased: {json}");
	let round_tripped: SerdeProbe = serde_json::from_str(&json).expect("deserialize should succeed");
	assert!(matches!(round_tripped.kind(), SerdeProbeKind::NoArgs));

	// A declared, serializable cause is preserved with full fidelity on the wire -- via a
	// borrowed downcast (`MaybeBorrowed::Borrowed`), not a clone -- rather than being erased.
	let with_declared_cause = SerdeProbe::new_with_cause(
		SerdeProbeKind::WithArgs { a: 1, b: 2 },
		Some(DeclaredCause {
			message: "boom".to_string(),
		}),
	);
	let json = serde_json::to_string(&with_declared_cause).expect("serialize should succeed");
	// eprintln!("declared cause (preserved, zero clone): {json}");
	// Preserved, not erased: the wire form is the declared type's own shape, not
	// UnserializableError's `errorMessage`/`errorKind` blob.
	assert!(json.contains(r#""errorCause":{"message":"boom"}"#));
	let round_tripped: SerdeProbe = serde_json::from_str(&json).expect("deserialize should succeed");
	assert!(matches!(round_tripped.kind(), SerdeProbeKind::WithArgs { a: 1, b: 2 }));

	// A payload that encodes the declared cause directly (e.g. written by hand, or
	// produced by another service) also deserializes into the concrete type, via the
	// untagged ordered-fallback matching on the cause enum.
	let handwritten = r#"{"errorMessage":"throwaway","errorTrace":"\n","errorKind":"withArgs","errorCause":{"message":"boom"},"errorDetail":{"a":1,"b":2}}"#;
	let round_tripped: SerdeProbe = serde_json::from_str(handwritten).expect("deserialize should succeed");
	assert!(matches!(round_tripped.kind(), SerdeProbeKind::WithArgs { a: 1, b: 2 }));

	// `SerdeProbe::new(kind)` (no cause at all) round-trips too.
	let no_cause = SerdeProbe::new(SerdeProbeKind::NoArgs);
	let json = serde_json::to_string(&no_cause).expect("serialize should succeed");
	let round_tripped: SerdeProbe = serde_json::from_str(&json).expect("deserialize should succeed");
	assert!(matches!(round_tripped.kind(), SerdeProbeKind::NoArgs));
}

#[test]
fn display_format_modifiers() {
	#[derive(Debug, Clone, abpl_macros::Error)]
	#[abpl_error(backtrace)]
	enum PublishKind {
		Failed,
	}
	impl core::fmt::Display for PublishKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("publish failed")
		}
	}

	#[derive(Debug, Clone, abpl_macros::Error)]
	#[abpl_error(location)]
	enum AuthKind {
		#[cause(Token)]
		Failed,
	}
	impl core::fmt::Display for AuthKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("auth error")
		}
	}

	#[derive(Debug, Clone, abpl_macros::Error)]
	enum TokenKind {
		Expired,
	}
	impl core::fmt::Display for TokenKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("token expired")
		}
	}

	let token = Token::new(TokenKind::Expired);
	let auth = Auth::new_with_cause(AuthKind::Failed, Some(token));
	let publish = Publish::new_with_cause(PublishKind::Failed, Some(auth));

	// eprintln!("--- {{}} ---\n{publish}");
	// eprintln!("--- {{:-}} ---\n{publish:-}");
	// eprintln!("--- {{:+}} ---\n{publish:+}");
	// eprintln!("--- {{:-.2}} ---\n{publish:-.2}");
	// eprintln!("--- {{:+.2}} ---\n{publish:+.2}");
	// eprintln!("--- {{:#}} ---\n{publish:#}");
	// eprintln!("--- {{:-#}} ---\n{publish:-#}");
	// eprintln!("--- {{:+#}} ---\n{publish:+#}");

	assert_eq!(format!("{publish}"), "publish failed");
	assert_eq!(format!("{publish:-}"), "publish failed ∵ auth error ∵ token expired");
	assert_eq!(format!("{publish:+}"), "token expired ∴ auth error ∴ publish failed");
	assert_eq!(format!("{publish:-.2}"), "publish failed ∵ auth error");
	assert_eq!(format!("{publish:+.2}"), "auth error ∴ publish failed");

	// `{:#}`: this error's own verbose block only, no chain expansion -- message plus trace,
	// no causal walk even though a cause is present.
	let verbose_own = format!("{publish:#}");
	assert!(verbose_own.starts_with("error: publish failed"));
	assert!(!verbose_own.contains("auth error"));

	// `{:-#}` / `{:+#}`: verbose chain, forward and reverse -- each hop renders its own verbose
	// block, separated by the `∵`/`∴` marker, with continuation lines indented.
	let verbose_forward = format!("{publish:-#}");
	let auth_pos = verbose_forward.find("∵ error: auth error").expect("forward chain should reach auth");
	let token_pos = verbose_forward
		.find("∵ error: token expired")
		.expect("forward chain should reach token");
	assert!(verbose_forward.starts_with("error: publish failed"));
	assert!(auth_pos < token_pos, "forward verbose chain should be root-first");

	let verbose_reverse = format!("{publish:+#}");
	assert!(verbose_reverse.starts_with("error: token expired"));
	assert!(verbose_reverse.contains("∴ error: auth error"));
	assert!(verbose_reverse.contains("∴ error: publish failed"));
}

#[test]
fn provider_cause_delegation() {
	#[derive(Debug, Clone, abpl_macros::Error)]
	#[abpl_provider(ProvidesExitCode(1.into(), exit_code, std::process::ExitCode))]
	enum InnerKind {
		Boom,
	}
	impl core::fmt::Display for InnerKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("inner boom")
		}
	}

	#[derive(Debug, Clone, abpl_macros::Error)]
	#[abpl_provider(ProvidesExitCode(2.into(), exit_code, std::process::ExitCode))]
	enum OuterKind {
		#[cause(Inner)]
		#[abpl_provider(exit_code(cause))]
		Wrapping,
		Standalone,
	}
	impl core::fmt::Display for OuterKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("outer wrapping")
		}
	}

	// Delegates to the cause's own `exit_code()` (1), not Outer's default (2).
	let wrapping_with_cause = Outer::new_with_cause(OuterKind::Wrapping, Some(Inner::new(InnerKind::Boom)));
	assert_eq!(wrapping_with_cause.exit_code(), 1.into());

	// No cause set at all -> falls back to Outer's own default (2).
	let wrapping_without_cause = Outer::new(OuterKind::Wrapping);
	assert_eq!(wrapping_without_cause.exit_code(), 2.into());

	// A variant that never opted into `cause` delegation just uses the default (2).
	let standalone = Outer::new(OuterKind::Standalone);
	assert_eq!(standalone.exit_code(), 2.into());
}

#[test]
fn map_err_with_is_lazy_and_receives_the_cause() {
	#[derive(Debug, Clone, abpl_macros::Error)]
	enum WithKind {
		#[cause(std::num::TryFromIntError)]
		#[cause(std::num::ParseIntError)]
		Two(u32, u32),
		#[cause(std::num::TryFromIntError)]
		One(u32),
	}
	impl core::fmt::Display for WithKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			write!(f, "{self:?}")
		}
	}

	// The closure must never run on the `Ok` path.
	let mut invoked = false;
	let ok: Result<u32, With> = Ok::<u32, std::num::TryFromIntError>(1u32).map_err_two_with(|_cause| {
		invoked = true;
		(0, 0)
	});
	assert!(ok.is_ok());
	assert!(!invoked, "closure ran even though the Result was Ok");

	// Multi-field variant: closure returns a tuple, and receives the actual cause.
	let try_from_cause = u32::try_from(-1i64 as u64).unwrap_err();
	let expected_len = try_from_cause.to_string().len() as u32;
	let err = Err::<u32, _>(try_from_cause)
		.map_err_two_with(|cause| (cause.to_string().len() as u32, 42))
		.unwrap_err();
	assert!(matches!(err.kind(), WithKind::Two(a, 42) if *a == expected_len));

	// Same variant, different declared cause type -- same trait, different `Self::Cause`.
	let parse_cause = "not a number".parse::<u32>().unwrap_err();
	let expected_len = parse_cause.to_string().len() as u32;
	let err = Err::<u32, _>(parse_cause)
		.map_err_two_with(|cause: &std::num::ParseIntError| (cause.to_string().len() as u32, 7))
		.unwrap_err();
	assert!(matches!(err.kind(), WithKind::Two(a, 7) if *a == expected_len));

	// Single-field variant: closure returns the bare value, not a 1-tuple.
	let err = u32::try_from(-1i64 as u64).map_err_one_with(|_cause| 99).unwrap_err();
	assert!(matches!(err.kind(), WithKind::One(99)));
}

#[test]
fn causeless_constructors_and_direct_construction() {
	use std::error::Error as _;

	#[derive(Debug, Clone, abpl_macros::Error)]
	enum PlainKind {
		Unit,
		Tuple(u32, u32),
		Named { a: u32, b: u32 },
	}
	impl core::fmt::Display for PlainKind {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			write!(f, "{self:?}")
		}
	}

	// Direct constructors for cause-free variants -- no `.new(kind)` ceremony required.
	let unit = Plain::unit();
	assert!(matches!(unit.kind(), PlainKind::Unit));
	assert!(unit.source().is_none());

	let tuple = Plain::tuple(1, 2);
	assert!(matches!(tuple.kind(), PlainKind::Tuple(1, 2)));

	let named = Plain::named(3, 4);
	assert!(matches!(named.kind(), PlainKind::Named { a: 3, b: 4 }));
}

#[test]
fn unserializable_error_walks_the_source_chain() {
	use crate::error::UnserializableError;

	#[derive(Debug)]
	struct Root;
	impl core::fmt::Display for Root {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("root cause")
		}
	}
	impl std::error::Error for Root {}

	#[derive(Debug)]
	struct Wrapper(Root);
	impl core::fmt::Display for Wrapper {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("wrapper failed")
		}
	}
	impl std::error::Error for Wrapper {
		fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
			Some(&self.0)
		}
	}

	let wrapped = Wrapper(Root);
	let inner_message = wrapped.0.to_string();
	let inner_debug = format!("{:?}", wrapped.0);

	let erased = UnserializableError::from_error(&wrapped);
	assert_eq!(erased.error_message, wrapped.to_string());
	assert_eq!(erased.error_kind, format!("{wrapped:?}"));
	let cause = erased.error_cause.as_ref().expect("source() should have been walked");
	assert_eq!(cause.error_message, inner_message);
	assert_eq!(cause.error_kind, inner_debug);
	assert!(cause.error_cause.is_none());

	// `Display` just forwards to `error_message`.
	assert_eq!(format!("{erased}"), erased.error_message);
}

#[test]
fn error_trace_display_variants() {
	use crate::error::ErrorTrace;

	assert_eq!(format!("{}", ErrorTrace::None), "\n");

	#[track_caller]
	fn make_location() -> ErrorTrace {
		ErrorTrace::new_location()
	}
	let location = make_location();
	let rendered = format!("{location}");
	assert!(rendered.starts_with(" @"));
	assert!(rendered.ends_with('\n'));
	assert!(rendered.contains(file!()));

	let backtrace = ErrorTrace::new_backtrace();
	// Always constructible and displayable regardless of `RUST_BACKTRACE`; content isn't
	// asserted since backtrace capture is itself environment-dependent.
	let _ = format!("{backtrace}");
}
