// tests/state_snapshot_tests.rs
//! Tests for state snapshot pattern and consistency (Issue #168)

#[cfg(test)]
mod tests {
    use crate::state_snapshot::{
        AtomicOperation, ReadConsistencyGuard, StateConsistencyChecker, StateSnapshotManager,
    };
    use soroban_sdk::testutils::Ledger;
    use soroban_sdk::Env;

    #[test]
    fn test_snapshot_creation() {
        let env = Env::default();
        let snapshot = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::create_snapshot(&env)
        });

        assert!(snapshot.timestamp > 0);
        assert!(snapshot.block_number > 0);
        assert!(snapshot.snapshot_id > 0);
    }

    #[test]
    fn test_snapshot_validation_same_block() {
        let env = Env::default();
        let snapshot = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::create_snapshot(&env)
        });

        // Should be valid in same block
        let valid = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::validate_snapshot(&env, &snapshot)
        });
        assert!(valid);
    }

    #[test]
    fn test_snapshot_validation_next_block() {
        let env = Env::default();
        let snapshot = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::create_snapshot(&env)
        });

        // Advance one block
        env.ledger().with_mut(|li| {
            li.sequence += 1;
        });

        // Should still be valid
        let valid = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::validate_snapshot(&env, &snapshot)
        });
        assert!(valid);
    }

    #[test]
    fn test_snapshot_invalid_after_multiple_blocks() {
        let env = Env::default();
        let snapshot = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::create_snapshot(&env)
        });

        // Advance multiple blocks
        env.ledger().with_mut(|li| {
            li.sequence += 5;
        });

        // Should be invalid
        let valid = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::validate_snapshot(&env, &snapshot)
        });
        assert!(!valid);
    }

    #[test]
    fn test_read_consistency_guard() {
        let env = Env::default();
        let guard = env.as_contract(&Address::generate(&env), || {
            ReadConsistencyGuard::new(&env)
        });

        // Should be valid initially
        let valid = env.as_contract(&Address::generate(&env), || {
            guard.validate(&env)
        });
        assert!(valid);

        // Advance one block - still valid
        env.ledger().with_mut(|li| {
            li.sequence += 1;
        });

        let valid = env.as_contract(&Address::generate(&env), || {
            guard.validate(&env)
        });
        assert!(valid);
    }

    #[test]
    fn test_state_consistency_checker_validates_transition() {
        let allowed_transitions = vec![
            (0, 1),
            (1, 2),
            (2, 3),
        ];

        // Valid transition
        assert!(StateConsistencyChecker::validate_transition(
            &0,
            &1,
            &allowed_transitions
        ));

        // Invalid transition
        assert!(!StateConsistencyChecker::validate_transition(
            &0,
            &2,
            &allowed_transitions
        ));
    }

    #[test]
    fn test_state_consistency_checker_preconditions() {
        let result = StateConsistencyChecker::validate_preconditions(|| {
            true
        });
        assert!(result);

        let result = StateConsistencyChecker::validate_preconditions(|| {
            false
        });
        assert!(!result);
    }

    #[test]
    fn test_execute_with_validation_success() {
        let result = StateConsistencyChecker::execute_with_validation(
            || 42,
            |value| *value == 42,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_execute_with_validation_failure() {
        let result = StateConsistencyChecker::execute_with_validation(
            || 42,
            |value| *value == 100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_ids_increment() {
        let env = Env::default();
        
        let snapshot1 = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::create_snapshot(&env)
        });

        let snapshot2 = env.as_contract(&Address::generate(&env), || {
            StateSnapshotManager::create_snapshot(&env)
        });

        assert_eq!(snapshot2.snapshot_id, snapshot1.snapshot_id + 1);
    }

    #[test]
    fn test_atomic_operation_executes_successfully() {
        let env = Env::default();
        
        let result = env.as_contract(&Address::generate(&env), |env| {
            AtomicOperation::execute(&env, |env, snapshot| {
                // Operation can use both env and snapshot
                assert!(snapshot.timestamp > 0);
                env.ledger().sequence()
            })
        });

        assert!(result > 0);
    }

    #[test]
    fn test_read_consistency_guard_panics_on_invalid() {
        let env = Env::default();
        let guard = env.as_contract(&Address::generate(&env), || {
            ReadConsistencyGuard::new(&env)
        });

        // Advance multiple blocks to make it invalid
        env.ledger().with_mut(|li| {
            li.sequence += 10;
        });

        // Should panic when ensuring consistency
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&Address::generate(&env), || {
                guard.ensure_consistent(&env);
            })
        }));
        assert!(result.is_err());
    }
}
