run *args:
    cargo run {{args}}

clippy:
    cargo clippy --all-targets -- -D warnings

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo +nightly fmt --all -- --check