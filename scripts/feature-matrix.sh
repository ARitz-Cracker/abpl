#!/usr/bin/env nix-shell
#!nix-shell -i bash -p cargo rustc cargo-hack
set -uo pipefail

# Exercises every meaningful combination of abpl's Cargo features and reports which ones
# are broken. `std` is always forced on: per the crate docs (src/lib.rs), disabling it is
# aspirational/non-functional today, so permuting it would just be noise.
#
# Covers:
#   1. `cargo check` on the plain default build (std only).
#   2. `cargo check` across the feature-powerset of every other documented, user-facing
#      feature (serde, utoipa, derive_error, app, http, future_util, thread, newtype_base64).
#   3. `cargo check --all-features` (what docs.rs builds).
#   4. `cargo test --all-features` -- the exact command the crate docs tell downstream
#      users/contributors to run to run the test suite.

cd "$(dirname "$0")/.."

FEATURES=serde,utoipa,derive_error,app,http,future_util,thread,newtype_base64
OUT_DIR="target/feature-matrix"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

fail_count=0

echo "==> [1/4] cargo check (default features)"
if ! cargo check -p abpl > "$OUT_DIR/default.log" 2>&1; then
	echo "    FAILED -- see $OUT_DIR/default.log"
	fail_count=$((fail_count + 1))
fi

echo "==> [2/4] cargo check across the feature powerset ($FEATURES), std forced on"
cargo hack check -p abpl --feature-powerset -F std --include-features "$FEATURES" --keep-going \
	> "$OUT_DIR/powerset.log" 2>&1
powerset_status=$?
total_combos=$(grep -c '^info: running' "$OUT_DIR/powerset.log")
if [ "$powerset_status" -ne 0 ]; then
	powerset_failed=$(grep -c '^        `' "$OUT_DIR/powerset.log")
	echo "    FAILED -- $powerset_failed/$total_combos combinations do not build; see $OUT_DIR/powerset.log"
	echo "    failing combinations:"
	awk '/^failed commands:/{p=1} p' "$OUT_DIR/powerset.log" | grep -oP '(?<=--features )[^`]+' | sed 's/^/      /'
	fail_count=$((fail_count + powerset_failed))
else
	echo "    all $total_combos combinations build"
fi

echo "==> [3/4] cargo check --all-features (docs.rs build)"
if ! cargo check -p abpl --all-features > "$OUT_DIR/all-features.log" 2>&1; then
	echo "    FAILED -- see $OUT_DIR/all-features.log"
	fail_count=$((fail_count + 1))
fi

echo "==> [4/4] cargo test --all-features (the documented test invocation)"
if ! cargo test -p abpl --all-features --no-fail-fast \
	> "$OUT_DIR/test-all-features.log" 2>&1; then
	echo "    FAILED -- see $OUT_DIR/test-all-features.log"
	fail_count=$((fail_count + 1))
fi

echo
if [ "$fail_count" -eq 0 ]; then
	echo "All checks passed."
else
	echo "$fail_count check(s) failed. Logs in $OUT_DIR/."
fi
exit "$fail_count"
