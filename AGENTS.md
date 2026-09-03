# AGENTS.md

## Project context

- Build: `cargo build --locked`
- Test: `cargo test --all-targets --locked`
- Lint: `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`
- Documentation: `RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked`
- Security: `cargo audit` and `cargo audit --file fuzz/Cargo.lock`
- Scope: Linux-first recorded USB descriptor inspection; no real hardware in the MVP.

## Working rules

Keep parsing bounded and deterministic. Do not add a real USB backend, kernel
driver, consumer application, or cross-platform claim without new evidence.
Treat permission errors and device disappearance as visible states. Run the
full validation commands before describing a change as complete.
