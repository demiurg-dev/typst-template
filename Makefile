check: check-lint check-lint-features check-format check-doc test test-features

check-format:
	cargo sort -w -g -o package,dependencies,features --check
	cargo +nightly fmt --check

format:
	cargo sort -w -g -o package,dependencies,features
	cargo +nightly fmt

check-lint:
	cargo clippy --no-deps --all-targets --all-features -- -D warnings

# Lint the feature-gated code paths --all-features can't reach: the `time`-backed
# clock (shadowed by `chrono`) and the no-feature build.
check-lint-features:
	cargo clippy -p typst-template --no-deps --no-default-features -- -D warnings
	cargo clippy -p typst-template --no-deps --no-default-features --features time -- -D warnings
	cargo clippy -p typst-template --no-deps --no-default-features --features time-tz -- -D warnings

# Note: plain `cargo test` (no --all-targets) so doctests run too — the doc
# examples are part of the test surface for a published crate. This uses the
# `chrono` clock backend (chrono wins under --all-features); `test-features`
# covers the others.
test:
	cargo test --all-features

# Test the runtime feature configurations `test` doesn't reach: no time backend,
# and the `time` backend (with `chrono` off).
test-features:
	cargo test --no-default-features --features derive
	cargo test --no-default-features --features derive,time,time-tz

# Fail on rustdoc warnings (e.g. broken intra-doc links) — doctests don't catch
# these. Part of `check`.
check-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps

# Build the docs.rs-style documentation (all features) and open it.
doc:
	cargo doc --all-features --no-deps --open
