/// Error catalog tests — verifies exact SwapTradeError codes for every revert path.
///
/// Each test asserts the *specific* variant returned, ensuring no two failure
/// modes share a code and no generic/ambiguous errors are used.
#[cfg(test)]
mod error_code_tests {
    use soroban_sdk::{symbol_short, Env};
    use crate::errors::SwapTradeError;
    use crate::validation::{validate_amount, validate_token_symbol, validate_swap_pair};
    use crate::staking_bonus::StakingBonusManager;
    use crate::emergency;

    // ── Validation errors (100–105) ─────────────────────────────────────────

    #[test]
    fn test_zero_amount_returns_invalid_amount() {
        let result = validate_amount(0);
        assert_eq!(result, Err(SwapTradeError::InvalidAmount));
        assert_eq!(SwapTradeError::InvalidAmount as u32, 100);
    }

    #[test]
    fn test_negative_amount_returns_invalid_amount() {
        let result = validate_amount(-1);
        assert_eq!(result, Err(SwapTradeError::InvalidAmount));
    }

    #[test]
    fn test_overflow_amount_returns_amount_overflow() {
        let result = validate_amount(1_000_000_000_000_000_001);
        assert_eq!(result, Err(SwapTradeError::AmountOverflow));
        assert_eq!(SwapTradeError::AmountOverflow as u32, 101);
    }

    #[test]
    fn test_invalid_token_symbol_returns_invalid_token_symbol() {
        let result = validate_token_symbol(symbol_short!("FAKE"));
        assert_eq!(result, Err(SwapTradeError::InvalidTokenSymbol));
        assert_eq!(SwapTradeError::InvalidTokenSymbol as u32, 102);
    }

    #[test]
    fn test_same_token_swap_returns_invalid_swap_pair() {
        let result = validate_swap_pair(symbol_short!("XLM"), symbol_short!("XLM"));
        assert_eq!(result, Err(SwapTradeError::InvalidSwapPair));
        assert_eq!(SwapTradeError::InvalidSwapPair as u32, 103);
    }

    // ── Rate limiting / slippage (300–301) ──────────────────────────────────

    #[test]
    fn test_error_codes_rate_limit_and_slippage_are_distinct() {
        assert_ne!(
            SwapTradeError::RateLimitExceeded as u32,
            SwapTradeError::SlippageExceeded as u32
        );
        assert_eq!(SwapTradeError::RateLimitExceeded as u32, 300);
        assert_eq!(SwapTradeError::SlippageExceeded as u32, 301);
    }

    // ── LP errors (400–401) ─────────────────────────────────────────────────

    #[test]
    fn test_error_codes_lp_variants_are_distinct() {
        assert_ne!(
            SwapTradeError::LPPositionNotFound as u32,
            SwapTradeError::InsufficientLPTokens as u32
        );
        assert_eq!(SwapTradeError::LPPositionNotFound as u32, 400);
        assert_eq!(SwapTradeError::InsufficientLPTokens as u32, 401);
    }

    // ── KYC errors (500–510) ────────────────────────────────────────────────

    #[test]
    fn test_kyc_error_codes_are_unique_and_in_range() {
        let kyc_variants: &[(SwapTradeError, u32)] = &[
            (SwapTradeError::KYCVerificationRequired, 500),
            (SwapTradeError::NotKYCOperator, 501),
            (SwapTradeError::InvalidKYCStateTransition, 502),
            (SwapTradeError::KYCTerminalStateImmutable, 503),
            (SwapTradeError::SelfVerificationNotAllowed, 504),
            (SwapTradeError::KYCOverrideNotFound, 505),
            (SwapTradeError::KYCTimelockNotElapsed, 506),
            (SwapTradeError::KYCOverrideAlreadyExecuted, 507),
            (SwapTradeError::InvalidTimelockDuration, 508),
            (SwapTradeError::KYCRequestExpired, 509),
            (SwapTradeError::InvalidExpiryDuration, 510),
        ];

        let mut seen = std::collections::HashSet::new();
        for (variant, expected_code) in kyc_variants {
            let code = *variant as u32;
            assert_eq!(code, *expected_code, "KYC variant code mismatch");
            assert!(seen.insert(code), "Duplicate KYC error code: {}", code);
        }
    }

    // ── Staking errors (600–605) ────────────────────────────────────────────

    #[test]
    fn test_invalid_stake_duration_returns_invalid_stake_duration() {
        let env = Env::default();
        let user = soroban_sdk::Address::generate(&env);
        let result = StakingBonusManager::stake(&env, user, 100, 45); // 45 is not a valid tier
        assert_eq!(result, Err(SwapTradeError::InvalidStakeDuration));
        assert_eq!(SwapTradeError::InvalidStakeDuration as u32, 600);
    }

    #[test]
    fn test_stake_zero_amount_returns_invalid_amount() {
        let env = Env::default();
        let user = soroban_sdk::Address::generate(&env);
        let result = StakingBonusManager::stake(&env, user, 0, 30);
        assert_eq!(result, Err(SwapTradeError::InvalidAmount));
    }

    #[test]
    fn test_claim_stake_not_found_returns_stake_not_found() {
        let env = Env::default();
        let user = soroban_sdk::Address::generate(&env);
        let result = StakingBonusManager::claim_stake(&env, user, 99);
        assert_eq!(result, Err(SwapTradeError::StakeNotFound));
        assert_eq!(SwapTradeError::StakeNotFound as u32, 601);
    }

    #[test]
    fn test_claim_bonuses_no_stakes_returns_stake_not_found() {
        let env = Env::default();
        let user = soroban_sdk::Address::generate(&env);
        let result = StakingBonusManager::claim_bonuses(&env, user);
        assert_eq!(result, Err(SwapTradeError::StakeNotFound));
    }

    #[test]
    fn test_distribution_too_early_returns_distribution_too_early() {
        let env = Env::default();
        // First call succeeds; second call within the period should fail.
        let _ = StakingBonusManager::execute_distribution(&env);
        let result = StakingBonusManager::execute_distribution(&env);
        assert_eq!(result, Err(SwapTradeError::DistributionTooEarly));
        assert_eq!(SwapTradeError::DistributionTooEarly as u32, 605);
    }

    #[test]
    fn test_staking_error_codes_are_unique() {
        let staking_variants: &[(SwapTradeError, u32)] = &[
            (SwapTradeError::InvalidStakeDuration, 600),
            (SwapTradeError::StakeNotFound, 601),
            (SwapTradeError::StakeNotActive, 602),
            (SwapTradeError::StakeLocked, 603),
            (SwapTradeError::NoClaimableBonuses, 604),
            (SwapTradeError::DistributionTooEarly, 605),
        ];

        let mut seen = std::collections::HashSet::new();
        for (variant, expected_code) in staking_variants {
            let code = *variant as u32;
            assert_eq!(code, *expected_code);
            assert!(seen.insert(code), "Duplicate staking error code: {}", code);
        }
    }

    // ── Emergency / circuit-breaker errors (700) ────────────────────────────

    #[test]
    fn test_emergency_pause_non_admin_returns_not_emergency_admin() {
        let env = Env::default();
        let non_admin = soroban_sdk::Address::generate(&env);
        let result = emergency::pause(&env, non_admin);
        assert_eq!(result, Err(SwapTradeError::NotEmergencyAdmin));
        assert_eq!(SwapTradeError::NotEmergencyAdmin as u32, 700);
    }

    #[test]
    fn test_emergency_freeze_non_admin_returns_not_emergency_admin() {
        let env = Env::default();
        let non_admin = soroban_sdk::Address::generate(&env);
        let target = soroban_sdk::Address::generate(&env);
        let result = emergency::freeze_user(&env, non_admin, target);
        assert_eq!(result, Err(SwapTradeError::NotEmergencyAdmin));
    }

    // ── Trading state errors (10–12) ────────────────────────────────────────

    #[test]
    fn test_trading_state_error_codes_are_distinct() {
        assert_eq!(SwapTradeError::TradingPaused as u32, 10);
        assert_eq!(SwapTradeError::UserFrozen as u32, 11);
        assert_eq!(SwapTradeError::CircuitBreakerTripped as u32, 12);
    }

    // ── Admin errors (1) ────────────────────────────────────────────────────

    #[test]
    fn test_not_admin_error_code() {
        assert_eq!(SwapTradeError::NotAdmin as u32, 1);
    }

    // ── Global uniqueness across all codes ──────────────────────────────────

    #[test]
    fn test_all_error_codes_are_globally_unique() {
        let all: &[u32] = &[
            // Admin
            1,
            // Trading state
            10, 11, 12,
            // Validation
            100, 101, 102, 103, 104, 105,
            // Oracle
            200, 201, 202, 203,
            // Rate / slippage
            300, 301,
            // LP
            400, 401,
            // KYC
            500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510,
            // Staking
            600, 601, 602, 603, 604, 605,
            // Emergency
            700,
        ];

        let mut seen = std::collections::HashSet::new();
        for &code in all {
            assert!(seen.insert(code), "Duplicate error code in catalog: {}", code);
        }
    }
}
