// src/nonce.rs
//! Replay-attack prevention via per-user nonces (#158).
//!
//! Each signed request must carry a nonce that has never been used before.
//! Used nonces are stored permanently so duplicate submissions are rejected.

use soroban_sdk::{contracttype, symbol_short, Address, Env};

use crate::errors::SwapTradeError;

/// Storage key for a used nonce: (user, nonce_value).
#[contracttype]
#[derive(Clone, Debug)]
pub enum NonceKey {
    Used(Address, u64),
}

pub struct NonceGuard;

impl NonceGuard {
    /// Consume `nonce` for `user`.
    ///
    /// Returns `Ok(())` the first time a nonce is seen.
    /// Returns `Err(RateLimitExceeded)` if the nonce was already used.
    pub fn consume(env: &Env, user: &Address, nonce: u64) -> Result<(), SwapTradeError> {
        let key = NonceKey::Used(user.clone(), nonce);
        if env.storage().persistent().has(&key) {
            return Err(SwapTradeError::RateLimitExceeded);
        }
        env.storage().persistent().set(&key, &true);
        env.events().publish(
            (symbol_short!("nonce"), symbol_short!("used")),
            (user.clone(), nonce),
        );
        Ok(())
    }

    /// Check whether a nonce has already been used (read-only).
    pub fn is_used(env: &Env, user: &Address, nonce: u64) -> bool {
        env.storage()
            .persistent()
            .has(&NonceKey::Used(user.clone(), nonce))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        let contract_id = env.register(crate::CounterContract, ());
        let user = Address::generate(&env);
        (env, contract_id, user)
    }

    #[test]
    fn test_fresh_nonce_accepted() {
        let (env, contract_id, user) = setup();
        env.as_contract(&contract_id, || {
            assert!(NonceGuard::consume(&env, &user, 1).is_ok());
        });
    }

    #[test]
    fn test_duplicate_nonce_rejected() {
        let (env, contract_id, user) = setup();
        env.as_contract(&contract_id, || {
            NonceGuard::consume(&env, &user, 42).unwrap();
            assert_eq!(
                NonceGuard::consume(&env, &user, 42),
                Err(SwapTradeError::RateLimitExceeded)
            );
        });
    }

    #[test]
    fn test_different_users_same_nonce() {
        let (env, contract_id, _) = setup();
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);
        env.as_contract(&contract_id, || {
            NonceGuard::consume(&env, &user_a, 7).unwrap();
            // Same nonce is fine for a different user.
            assert!(NonceGuard::consume(&env, &user_b, 7).is_ok());
        });
    }

    #[test]
    fn test_is_used_reflects_state() {
        let (env, contract_id, user) = setup();
        env.as_contract(&contract_id, || {
            assert!(!NonceGuard::is_used(&env, &user, 99));
            NonceGuard::consume(&env, &user, 99).unwrap();
            assert!(NonceGuard::is_used(&env, &user, 99));
        });
    }

    #[test]
    fn test_replay_attack_simulation() {
        let (env, contract_id, user) = setup();
        env.as_contract(&contract_id, || {
            // First submission succeeds.
            NonceGuard::consume(&env, &user, 1000).unwrap();
            // Replayed submission fails.
            assert_eq!(
                NonceGuard::consume(&env, &user, 1000),
                Err(SwapTradeError::RateLimitExceeded)
            );
        });
    }
}
