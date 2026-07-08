# CI

PeerX uses a single GitHub Actions workflow to gate merges into `main`:

| File | Jobs | Triggers |
|---|---|---|
| `ci.yml` | `quality`, `build`, `test` | push & PR against `main` |

## Jobs at a glance

### `quality` — lint and static-check gate

Runs three checks in order:

1. `cargo fmt --all -- --check`
   Enforces rustfmt formatting across the workspace. Any whitespace /
   import-ordering drift fails this job.
2. `cargo check --workspace --verbose`
   Type-checks every workspace member without producing artifacts. Catches
   dead imports, unused warnings (via `-D warnings` if added later),
   syntactic rot.
3. `bash scripts/check_kyc_guards.sh`
   Lightweight static audit that every sensitive contract entry point
   (`swap`, `safe_swap`, `add_liquidity`, `remove_liquidity`, `stake`, ...)
   invokes one of the shared KYC / auth guards
   (`require_authenticated_verified_user` / `require_verified_user`).
   Runs in pure bash + awk + grep; no Rust toolchain build needed.

### `build` — debug build gate

Runs `cargo build --workspace --verbose`. Catches any compile error that
`cargo check` somehow misses (the two usually agree; this job catches
proc-macro and build-script fallout that the type-checker glosses over).

### `test` — test-suite gate

Runs four commands:

1. `cargo test --workspace --lib --verbose` — every `#[test]` annotated
   function in the workspace's library targets.
2. `cargo test --manifest-path peerx-contracts/counter/Cargo.toml --lib kyc_tests -- --nocapture`
   Targeted: give the lib test a focused KYC-name filter so failures are
   easier to triage in CI logs.
3. `cargo test --manifest-path peerx-contracts/counter/Cargo.toml error_code_tests -- --nocapture`
   The error-code regression suite — every `PeerXError` variant must
   resolve to the exact u32 value documented in `errors.rs`. (Note: this
   suite was NOT in `formal_verification.yml`; it's a peerx-cargo test
   that pairs with the renumbered `SwapTradeError -> PeerXError` enum.)
4. `cargo test --manifest-path peerx-contracts/counter/Cargo.toml --test formal_verification_tests formal_verification -- --nocapture --test-threads=1`
   Property-based formal-verification integration tests, single-threaded.

## Why one workflow, three jobs

*Single canonical workflow file* means a contributor reading
`.github/workflows/` sees exactly one place to look. Compare to the
three-file historical setup (`ci.yml`, `format.yml`,
`formal_verification.yml`) where every workflow was a candidate source
of failure, fmt enforcement was duplicated, and the long-running
formal-verification job ran in parallel with test, tripling compute
without adding signal.

## What this workflow does NOT cover

| Intentionally left out | Why |
|---|---|
| Release build | Not a merge gate; release profile sometimes masks errors that debug catches. Deploy pipelines handle this. **However:** for Soroban the release-mode WASM IS the on-chain deploy artifact, and the workspace `[profile.release]` has `lto = "fat"` / `opt-level = "z"` / `debug-assertions = false` — LTO-time const evaluation can surface errors missed by debug builds. Plan to reintroduce a release-build gate before mainnet deployment. |
| Auto-format-and-push on main | Was a force-push bot with `403 permission denied` on its GitHub token AND a bad practice (CI reformatting committed code creates noisy diffs). |
| The `exhaustive_` 10k-sequence property test | Took 30 min on CI; redundant with the targeted `formal_verification` filter step. **However:** this was the most coverage-dense test in the prior CI setup (10k randomly-generated sequences per run) and dropping it is a real coverage loss. The fast gated set still catches typical regressions; flagging this for future reintroduction is correct. |
| The `witness_case` step | Most `witness_case` tests are `#[ignore]` — annotated and don't run by default. |

## Coverage delta vs. deleted `formal_verification.yml`

The deleted workflow had five steps. Here is what is preserved vs.
genuinely lost in the slim `ci.yml` `test` job:

| Step in old `formal_verification.yml` | Coverage in current `ci.yml` |
|---|---|
| `formal_verification` filter | ✅ step 4 above |
| `kyc_tests::` filter | ✅ step 2 above |
| `error_code_tests` filter | ✅ step 3 above (new addition, not a duplicate of anything old) |
| `exhaustive_` 10k-sequences filter | ❌ dropped — the slowest but most coverage-dense test |
| `witness_case` filter | ❌ dropped — most are `#[ignore]`'d anyway |
| Final release build | ❌ dropped (already handled by Build Release discussion) |

The current `ci.yml` `test` job covers a subset of the old
`formal_verification.yml`, **not** the full superset. Reintroducing the
`exhaustive_` property test (e.g., on a schedule, or as an opt-in PR
label) would close the gap.

## Editor setup (contributor)

Install rustfmt and run before pushing:

```bash
rustup component add rustfmt   # one-time
cargo fmt --all                # local fix-up
# or:
cargo fmt --all -- --check     # CI's gate
```

## Local reproduction

```bash
# Quality mirrors GitHub Actions:
cargo fmt --all -- --check
cargo check --workspace
bash scripts/check_kyc_guards.sh

# Build:
cargo build --workspace

# Test:
cargo test --workspace --lib
cargo test --manifest-path peerx-contracts/counter/Cargo.toml --lib kyc_tests
cargo test --manifest-path peerx-contracts/counter/Cargo.toml error_code_tests
cargo test --manifest-path peerx-contracts/counter/Cargo.toml --test formal_verification_tests formal_verification
```
