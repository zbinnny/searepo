lint:
	cargo clippy --all --tests --examples --benches --all-features -- -D warnings

check:
	cargo c --all --tests --examples --benches --all-features

fmt:
	cargo fmt

