lint:
	cargo check --lib --all-features
	cargo test --doc --all-features
	cargo clippy --lib --tests --all-features -- -D warnings
	cargo fmt --all --check
