# ABPL: Aritz's BoilerPlate Library

_A collection of junk that I only want to write once._

A Rust crate of infrastructure for small services: a reloadable-systemd-service lifecycle helper,
a hot-reloading axum wrapper, a serializable/typed error derive macro, a tokio runtime bridge for
mixing sync and async code, and various small newtypes.

Almost everything beyond `std` is opt-in behind Cargo features -- see the crate-level docs for the
full list and what each one unlocks:

```
cargo doc --all-features --open
```

## MSRV

The minimum supported Rust version is currently 1.95, matching the version of `rustc` which ships with NixOS 26.05.
This may increase as NixOS-stable updates.

## Development

This project uses Nix for its dev environment:

```
nix-shell
```

which provides the Rust toolchain, rust-analyzer, clippy, and cargo-llvm-cov.

Run the test suite with:

```
cargo test --all-features
```

and check coverage with:

```
cargo llvm-cov --all-features --html
```

## License

AGPL-3.0-or-later -- see [LICENSE](LICENSE).
