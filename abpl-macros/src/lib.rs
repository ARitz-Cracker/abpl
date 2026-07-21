use proc_macro::TokenStream;

pub(crate) mod attributes;
mod error;

/// Derives a full error type from a "kind" enum.
///
/// The annotated enum's name must end in `Kind` (e.g. `ReadFileKind`); the macro strips that
/// suffix and generates a wrapper struct of the remaining name (`ReadFile`) that pairs the kind
/// with a type-erased cause and a capture-site trace:
///
/// ```ignore
/// pub struct ReadFile {
///     kind: ReadFileKind,
///     cause: Option<Rc<dyn core::error::Error>>,
///     trace: abpl::error::ErrorTrace,
/// }
/// ```
///
/// `{Kind}` itself must implement `Debug`, `Clone`, and `Display` (the latter is not derived --
/// write it yourself, or delegate field-by-field; it becomes the wrapper's own message). The
/// wrapper gets, for free:
/// - `{Name}::new(kind)` / `{Name}::new_with_cause(kind, Option<impl Error + 'static>)` (both
///   `#[track_caller]`), and `{Name}::kind(&self) -> &{Kind}`.
/// - `impl Display for {Name}`, with format modifiers for causal-chain rendering (see below).
/// - `impl core::error::Error for {Name}`, with `source()` returning the cause.
/// - `{Name}::{variant}(fields...)` for every variant with no `#[cause(..)]` at all (see
///   "Constructing values" below).
///
/// # Constructing values
///
/// How you build a `{Name}` depends on whether the variant declares a `#[cause(..)]`:
///
/// - **No cause on the variant**: a direct, `#[track_caller]` constructor is generated per
///   variant, named after it in `snake_case` -- `{Name}::{variant}(field_a, field_b, ...)` for a
///   struct/tuple variant, `{Name}::{variant}()` for a unit variant. It's just
///   `Self::new({Kind}::{Variant} { .. })` under the hood, so `#[track_caller]` matters here too:
///   without it, a captured `location`/`backtrace` trace would point inside this generated
///   function instead of at your actual call site.
///   ```ignore
///   ReadFile::not_found(path.to_string())
///   ```
/// - **Variant declares `#[cause(Type)]`** (repeatable on the same variant; `unserializable`
///   only affects the JSON representation, see below):
///   - If the variant is a unit variant and `Type` isn't shared with any other variant in the
///     enum, this generates `impl From<Type> for {Name}`, so `some_io_call()?` works directly
///     via `?`'s `.into()` -- no explicit constructor call at all.
///     ```ignore
///     fn read(path: &str) -> Result<String, ReadFile> {
///         Ok(std::fs::read_to_string(path)?) // `?` converts `io::Error` via `From`
///     }
///     ```
///   - Otherwise (the variant has fields, or `Type` is declared on more than one variant), a
///     `ResultInto{Name}{Variant}<T>` trait is generated with a `map_err_{variant}` method, so
///     you write `some_call().map_err_variant(field_a, field_b)?` instead of a hand-rolled
///     `map_err` closure -- and unlike `map_err`, this preserves `#[track_caller]` span info
///     through the `?`.
///     ```ignore
///     fn read(path: &str) -> Result<String, ReadFile> {
///         std::fs::read_to_string(path).map_err_io(path.to_string())
///     }
///     ```
///     If the variant has fields, the same trait also gets `map_err_{variant}_with`, taking a
///     closure `FnOnce(&Cause) -> Fields` instead of the fields directly (`Cause` is whichever
///     declared `#[cause(..)]` type actually caused the `Err`; `Fields` is the bare field type
///     for a single-field variant, or a tuple in declaration order for more than one). The
///     closure only runs on the `Err` path, so it's the right choice when the fields are
///     expensive to produce (a clone, a formatted string, ...) and you don't want to pay for
///     that on the common success path:
///     ```ignore
///     fn read(path: &str) -> Result<String, ReadFile> {
///         std::fs::read_to_string(path).map_err_io_with(|_cause| path.to_string())
///     }
///     ```
///
/// # `#[abpl_error(...)]`
///
/// Top-level attribute, all arguments optional and combinable:
/// - `location` / `backtrace`: capture `ErrorTrace::new_location()` / `::new_backtrace()` on
///   construction (mutually exclusive). Default: no trace captured.
/// - `serialize` / `deserialize` / `utoipa`: opt into `Serialize`/`Deserialize`/`utoipa::ToSchema`
///   for `{Name}` (see "JSON representation" below). Each is independent -- e.g. `serialize`
///   alone is fine if you never need to read the JSON back.
///
/// # `#[abpl_provider(...)]`
///
/// Lets `{Name}` implement arbitrary traits with per-variant return values, dispatched through
/// `{Kind}`. Top-level: `#[abpl_provider(SomeTrait(default_value, method_name, ReturnType))]`
/// declares that `{Name}: SomeTrait { fn method_name(&self) -> ReturnType }`, using
/// `default_value` for any variant that doesn't override it. Per-variant:
/// `#[abpl_provider(method_name(some_value))]` overrides the value for that variant (or
/// `SomeTrait(some_value)` if `method_name` is ambiguous across multiple declared traits).
///
/// The return value `cause` is a special sentinel: instead of a literal expression, it tries
/// each of the variant's declared `#[cause(..)]` types in order (via downcast against the
/// type-erased cause), delegating to *that type's own* `method_name()` if one matches --
/// falling back to the trait's default if no cause is set, or none of the declared types match
/// the runtime value. This requires the matched cause type to itself implement `SomeTrait`,
/// enforced at the generated call site (like any other trait bound, this can be a slightly
/// confusing error if you get it wrong).
///
/// # Display format modifiers
///
/// `{Name}`'s `Display` impl repurposes the standard formatting flags to control how much of the
/// causal chain (via `core::error::Error::source()`) gets rendered -- this works across crate
/// boundaries for free, since it only relies on `source()` and re-entrant `Display`, never the
/// concrete type of a cause:
///
/// | Format     | Renders |
/// |------------|---------|
/// | `{}`       | Just this error's own message. |
/// | `{:-}`     | Full causal chain, root first: `read failed ∵ file not found`. |
/// | `{:+}`     | Same chain, reversed: `file not found ∴ read failed`. |
/// | `{:-.N}` / `{:+.N}` | Same as above, limited to `N` entries via the precision modifier. |
/// | `{:#}`     | This error's own verbose block: message plus its trace (location/backtrace). |
/// | `{:-#}` / `{:+#}` | Verbose chain: each hop renders itself via `{:#}` (falling back to a plain one-liner if that hop's `Display` ignores `#`), indented under a `∵`/`∴` marker. |
///
/// # JSON representation
///
/// With `serialize`/`deserialize` enabled, `{Name}` serializes as an internally-tagged object:
///
/// ```json
/// {
///   "errorMessage": "failed to read \"/etc/config.toml\"",
///   "errorTrace": " @src/config.rs:42:9\n",
///   "errorKind": "io",
///   "errorCause": {
///     "errorMessage": "No such file or directory (os error 2)",
///     "errorKind": "Os { code: 2, kind: NotFound, message: \"No such file or directory\" }",
///     "errorCause": null
///   },
///   "errorDetail": { "path": "/etc/config.toml" }
/// }
/// ```
///
/// `errorKind` is the variant name in `camelCase`; `errorDetail` mirrors that variant's own fields (`null` for
/// a unit variant). `errorCause` is omitted entirely when there's no cause. A cause marked
/// `unserializable` (or any foreign type that isn't one of the variant's declared, serializable
/// `#[cause(..)]` types) always erases to the recursive `{errorMessage, errorKind, errorCause}`
/// shape shown above, walking the whole `source()` chain -- this never requires `Clone` on the
/// cause type. A declared, non-`unserializable` cause instead serializes with its own full
/// fidelity (whatever shape that type's own `Serialize` produces) rather than being erased.
/// Since every macro-generated error shares this exact shape, nesting one as another's
/// non-erased cause produces a JSON tree a type-agnostic frontend can walk generically via
/// `errorCause` -- as long as you stick to nesting other `{derive(abpl::Error)}` types (or
/// otherwise guarantee the same shape); a plain foreign `#[cause(..)]` type won't have
/// `errorMessage`/`errorCause` fields of its own to walk into.
///
/// # Example
///
/// ```ignore
/// #[derive(Debug, Clone, abpl::Error, serde::Serialize, serde::Deserialize)]
/// #[abpl_error(location, serialize, deserialize)]
/// enum ReadFileKind {
///     #[cause(std::io::Error, unserializable)]
///     Io { path: String },
///     NotFound { path: String },
/// }
///
/// impl std::fmt::Display for ReadFileKind {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             Self::Io { path } => write!(f, "failed to read {path:?}"),
///             Self::NotFound { path } => write!(f, "{path:?} not found"),
///         }
///     }
/// }
///
/// fn read(path: &str) -> Result<String, ReadFile> {
///     let path = path.to_string();
///     if !std::path::Path::new(&path).exists() {
///         return Err(ReadFile::not_found(path)); // no #[cause(..)] on `NotFound` -> direct constructor
///     }
///     std::fs::read_to_string(&path).map_err_io(path)
/// }
/// ```
#[proc_macro_derive(Error, attributes(cause, abpl_error, abpl_provider))]
pub fn derive_error(input: TokenStream) -> TokenStream {
	error::derive(syn::parse_macro_input!(input as syn::DeriveInput))
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}
