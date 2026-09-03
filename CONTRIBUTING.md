# Contributing

Keep changes narrow and evidence-backed. Add or update a fixture and a test
for each protocol behavior. Do not imply real-device compatibility from mock
or recorded-session coverage.

Before review, run:

```text
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked
cargo package --locked
cargo audit
cargo audit --file fuzz/Cargo.lock
```

The repository uses trunk-based development. A human reviews and commits
changes.
