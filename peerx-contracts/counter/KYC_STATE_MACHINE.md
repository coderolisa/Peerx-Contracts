# KYC State Machine

This contract enforces a finite state machine for user KYC records and treats `Verified` and `Rejected` as terminal states.

## States

- `Unverified`
- `Pending`
- `InReview`
- `AdditionalInfoRequired`
- `Verified`
- `Rejected`

## Allowed Transitions

Only the following transitions are permitted through the normal operator flow:

- `Unverified -> Pending`
- `Pending -> InReview`
- `InReview -> AdditionalInfoRequired`
- `InReview -> Verified`
- `InReview -> Rejected`
- `AdditionalInfoRequired -> InReview`

All skipped, reverse, or lateral transitions revert with deterministic contract errors.

## Terminal State Immutability

- `Verified` and `Rejected` are terminal states.
- Terminal records set `finalized_at` when they are reached.
- Once finalized, the record cannot be changed through the normal operator path.
- Any attempt to mutate a terminal state through `kyc_update_status` reverts with `KYCTerminalStateImmutable`.

## Operator and Governance Controls

- Only addresses with the KYC operator role can assign or update KYC statuses.
- Operators cannot self-verify or self-assign KYC outcomes.
- Governance overrides are the only supported path for changing a terminal KYC outcome.
- Governance overrides are timelocked and must be proposed before they can be executed.

## Sensitive Entry Point Policy

The contract uses shared KYC guard helpers to ensure verified status is enforced consistently across sensitive entry points:

- `swap`
- `safe_swap`
- `add_liquidity`
- `remove_liquidity`
- `stake`
- `claim_staking_bonuses`
- `claim_stake`
- `unstake_early`
- `pool_add_liquidity`
- `pool_remove_liquidity`
- `pool_swap`
- Batch execution paths that can invoke swap or liquidity operations

CI runs `scripts/check_kyc_guards.sh` to verify these entry points continue to call the shared KYC guard logic.
