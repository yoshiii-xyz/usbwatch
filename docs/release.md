# Release gates

Run the following from a clean checkout:

```text
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked
cargo package --locked
cargo publish --dry-run --locked
cargo audit
cargo audit --file fuzz/Cargo.lock
timeout --foreground 300s env RUSTUP_TOOLCHAIN=nightly cargo fuzz build descriptor-bytes
timeout --foreground 60s env RUSTUP_TOOLCHAIN=nightly cargo fuzz run descriptor-bytes -- -max_total_time=10 -verbosity=0 -print_final_stats=1
```

The hosted CI, Security, CodeQL, and tag package workflows must pass. Release
evidence belongs in the ignored `qa/evidence/` directory and must record the
commit, tag, checksums, and known hardware boundary.
