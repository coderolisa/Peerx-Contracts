# PR: feat(formal-verification): add property tests, invariant predicates, and CI enforcement

## Summary

This PR introduces a formal verification framework for the SwapTrade contract, including property-style tests, invariant predicates exported from the contract, documentation, and CI enforcement.

## Changes

- Added generative / property-style verification tests in `swaptrade-contracts/counter/tests/formal_verification_tests.rs`.
- Exported invariant predicate functions from `swaptrade-contracts/counter/portfolio.rs` and removed a duplicate `debit()` implementation.
- Added `FORMAL_VERIFICATION.md` (formal spec + proofs) and `FORMAL_VERIFICATION_GUIDE.md` (how-to, commands, audit roadmap).
- CI workflows:
  - `.github/workflows/formal_verification.yml` — runs property tests, exhaustive runs, witness cases, clippy, and cargo-audit.
  - `.github/workflows/format.yml` — checks formatting on PRs and auto-formats on pushes to `main`.
- Added `scripts/verify_formal.sh` to run the full formal verification suite locally.
- Added `proptest` as a dev-dependency in `swaptrade-contracts/counter/Cargo.toml` (ready for future generative tests).

## How to test locally

```bash
# ensure Rust toolchain and rustfmt are installed
rustup update stable
rustup component add rustfmt

# run unit/property tests for counter crate
cd swaptrade-contracts/counter
cargo test --all -- --nocapture

# run the formal verification script (exhaustive tests may take time)
cd /workspaces/swaptrade-contract
chmod +x scripts/verify_formal.sh
./scripts/verify_formal.sh

# check/format code
cargo fmt --all
cargo clippy -- -D warnings
```

## Notes

- I couldn't run `cargo` or `rustfmt` in this environment; CI will execute the workflows on PRs.
- The formal spec documents limitations (e.g., Soroban map iteration) and recommends off-chain aggregation or a user registry for full on-chain sum checks.

## Request

Please run CI; if formatting changes are required, the `.github/workflows/format.yml` will apply them on `main` or fail PRs. If you prefer formatting fixes as PRs instead of direct pushes, I can update the workflow to open a formatting PR.
