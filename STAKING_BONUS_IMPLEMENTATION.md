# Staking Bonus System - Implementation Guide

## Overview

A complete long-term staking bonus system for SwapTrade contract with:
- ✅ Duration-based bonus calculations
- ✅ Periodic distribution mechanism
- ✅ Time locks for security
- ✅ Full transparency and auditability
- ✅ Claimable bonuses

## Acceptance Criteria - Status

| Criteria | Status | Details |
|----------|--------|---------|
| Bonuses earned | ✅ DONE | Automatic calculation based on duration tier |
| Bonuses claimable | ✅ DONE | After 30-day holding period |
| Users can claim | ✅ DONE | Full claim functionality implemented |
| Time locks | ✅ DONE | Multi-level: 30-day bonus hold, duration-based unlock |
| Transparent | ✅ DONE | All query functions expose full data |
| Periodic distribution | ✅ DONE | 7-day distribution cycles with history tracking |

## Bonus Tiers

```
Duration    | Bonus | Example (1000 units)
------------|-------|---------------------
30 days     | 5%    | 50 units
60 days     | 12%   | 120 units
90 days     | 20%   | 200 units
365 days    | 50%   | 500 units
```

## Time Lock Mechanics

### Lock Periods
1. **Bonus Holding Period**: 30 days (after stake creation)
   - Bonus becomes claimable after this period
   - Prevents flash-loan exploitation

2. **Principal Lock Period**: Duration-based
   - 30-day stake: unlocks in 30 days
   - 60-day stake: unlocks in 60 days
   - 90-day stake: unlocks in 90 days
   - 365-day stake: unlocks in 365 days

3. **Distribution Cycle**: 7 days
   - Bonuses distributed periodically
   - Prevents mass claim exploits

### Early Exit
- **Penalty**: 10% of principal
- **Bonus**: Forfeited
- **Use case**: Emergency unstaking

## Architecture

### Data Structures

#### StakeRecord
```rust
pub struct StakeRecord {
    pub amount: i128,              // Principal amount
    pub staked_at: u64,            // Creation timestamp
    pub unlock_at: u64,            // Principal unlock time
    pub duration_secs: u64,        // Lock duration in seconds
    pub bonus_bps: u32,            // Bonus percentage (basis points)
    pub bonus_amount: i128,        // Calculated bonus
    pub bonus_claimed: bool,       // Claim status
    pub claimable_at: u64,         // Time when bonus becomes claimable
    pub is_active: bool,           // Active/inactive status
}
```

#### DistributionRecord
```rust
pub struct DistributionRecord {
    pub distributed_at: u64,       // Distribution timestamp
    pub total_distributed: i128,   // Total amount distributed
    pub recipient_count: u32,      // Number of recipients
    pub average_bonus: i128,       // Average bonus per stake
}
```

### Storage Keys
```rust
pub enum StakingBonusKey {
    UserStakes(Address),           // User's all stakes
    UserTotalStaked(Address),      // User's total staked
    UserEarnedBonuses(Address),    // User's total earned
    UserClaimedBonuses(Address),   // User's claimed amount
    DistributionRecords,           // All distributions
    LastDistributionTime,          // Last distribution timestamp
    TotalStaked,                   // Global total staked
    TotalBonusesDistributed,       // Global total distributed
}
```

## Contract Functions

### User Operations

#### Staking
```rust
pub fn stake(
    env: Env,
    user: Address,
    amount: i128,
    duration_days: u32  // 30, 60, 90, or 365
) -> u32  // Returns stake_id
```

#### Claiming Bonuses
```rust
pub fn claim_staking_bonuses(env: Env, user: Address) -> i128
```
- Requires: 30-day holding period elapsed
- Returns: Total bonuses claimed in this transaction

#### Claiming Principal
```rust
pub fn claim_stake(env: Env, user: Address, stake_id: u32) -> i128
```
- Requires: Lock period elapsed
- Returns: Principal amount

#### Early Exit (with penalty)
```rust
pub fn unstake_early(env: Env, user: Address, stake_id: u32) -> (i128, i128)
```
- Returns: (principal_after_penalty, penalty_amount)
- Penalty: 10% of principal

### Query Functions (Transparency)

#### User Data
```rust
pub fn get_user_stakes(env: Env, user: Address) -> Vec<StakeRecord>
pub fn get_stake_details(env: Env, user: Address, stake_id: u32) -> StakeRecord
pub fn get_user_total_staked(env: Env, user: Address) -> i128
pub fn get_user_earned_bonuses(env: Env, user: Address) -> i128
pub fn get_user_claimed_bonuses(env: Env, user: Address) -> i128
pub fn get_user_pending_bonuses(env: Env, user: Address) -> i128
```

#### Global Statistics
```rust
pub fn get_staking_statistics(env: Env) -> (i128, i128, u64)
// Returns: (total_staked, total_distributed, distribution_count)

pub fn get_distribution_history(env: Env) -> Vec<DistributionRecord>
```

#### Distribution Management
```rust
pub fn execute_staking_distribution(env: Env) -> DistributionRecord
```

## Event Emissions (Transparency)

All operations emit events for blockchain indexing:

```
stake                      → (user, duration, amount, bonus_bps)
claim_bonus               → (user, bonus_amount)
claim_stake               → (user, stake_id, principal)
unstake_early             → (user, stake_id, penalty)
distribution              → (timestamp, total_distributed, recipient_count)
```

## Usage Examples

### Example 1: Standard 90-Day Stake
```rust
// User stakes 1000 units for 90 days
let stake_id = contract.stake(env, user, 1000, 90);
// Bonus: 200 units (20%)

// After 30 days: user can claim bonus
let claimed_bonus = contract.claim_staking_bonuses(env, user);  // 200

// After 90 days: user can claim principal
let principal = contract.claim_stake(env, user, stake_id);     // 1000
```

### Example 2: Early Exit with Penalty
```rust
// User wants to exit early
let (principal, penalty) = contract.unstake_early(env, user, stake_id);
// If:  stake_amount = 1000
// Then: principal = 900, penalty = 100
```

### Example 3: Transparency Query
```rust
// Check all your stakes
let stakes = contract.get_user_stakes(env, user);

// Get global statistics
let (total_staked, total_distributed, distribution_count) = 
    contract.get_staking_statistics(env);

// Audit distribution history
let history = contract.get_distribution_history(env);
```

## Security Features

### 1. Time Locks
- Prevents flash-loan attacks
- 30-day minimum bonus holding
- Duration-based principal locks

### 2. Anti-Gaming Measures
- Early unstaking penalty (10%)
- Bonus only claimable after holding period
- Separate bonus and principal claim flows

### 3. Rate Limiting
- 7-day distribution cycle prevents flooding
- Stake operations are atomic

### 4. Transparency
- All operations emit events
- Full query API for audit trails
- Immutable distribution records

## Testing

Comprehensive test suite in `staking_bonus_tests.rs`:

- ✅ Basic staking operations (30/60/90/365 days)
- ✅ Time lock enforcement
- ✅ Bonus calculations
- ✅ Early unstaking penalties
- ✅ Bonus claiming with holding period
- ✅ Principal claiming with unlock
- ✅ Transparency queries
- ✅ Distribution cycle
- ✅ Full lifecycle tests
- ✅ Edge cases and errors

Run tests:
```bash
cargo test staking_bonus_tests
```

## Integration

The staking bonus system is integrated into the main contract:

1. **Module Definition**: `src/staking_bonus.rs`
2. **Tests**: `src/staking_bonus_tests.rs`
3. **Contract Export**: All types and functions exported in `lib.rs`
4. **API**: Full set of contract functions available

## Implementation Completeness

### Core Features
- ✅ Duration-based bonus calculation
- ✅ Four bonus tiers (5%, 12%, 20%, 50%)
- ✅ Periodic distribution tracking
- ✅ Time-lock enforcement
- ✅ Early exit with penalties
- ✅ Bonus claiming after holding period
- ✅ Principal claiming after unlock

### Transparency
- ✅ Public query functions for all data
- ✅ Event emissions for all actions
- ✅ Distribution history tracking
- ✅ Global statistics
- ✅ Individual stake details

### Robustness
- ✅ Input validation
- ✅ State consistency checks
- ✅ Error handling
- ✅ Comprehensive tests
- ✅ Event-based audit trail

## Files Modified/Created

1. **Created**: `/swaptrade-contracts/counter/src/staking_bonus.rs` (668 lines)
   - Core staking bonus implementation
   - All business logic

2. **Created**: `/swaptrade-contracts/counter/src/staking_bonus_tests.rs` (400+ lines)
   - Comprehensive test coverage
   - 20+ test cases

3. **Modified**: `/swaptrade-contracts/counter/src/lib.rs`
   - Added module imports
   - Added 11 contract functions
   - Added public re-exports

## Definition of Done - COMPLETE ✅

- ✅ Users can stake for specified durations
- ✅ Bonuses are calculated automatically based on duration
- ✅ Bonuses earn for different periods (30/60/90/365 days)
- ✅ Bonuses are claimable after 30-day holding period
- ✅ Time locks prevent premature access
- ✅ Distribution occurs periodically (7-day cycle)
- ✅ All operations are transparent and auditable
- ✅ Full test coverage with edge cases
- ✅ Early exit option with penalty
- ✅ Global statistics for transparency

## Guidelines - Implementation ✅

- **Use time locks**: ✅ Multi-level time locks implemented
- **Transparent**: ✅ Full query API, event emissions, history tracking
- **Periodic Distribution**: ✅ 7-day cycle with records
- **Rules**: ✅ Duration-based calculations, fixed tier percentages
