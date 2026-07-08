// tests/governance_multisig_tests.rs
//! Tests for multi-signature governance execution (Issue #166)
//! and prevention of double execution (Issue #167)

#[cfg(test)]
mod tests {
    use crate::governance::{
        MultiSigCoordinator, MultiSigProposal, MULTISIG_THRESHOLD, MULTISIG_TOTAL,
    };

    #[test]
    fn test_multisig_requires_minimum_signatures() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        // Create a proposal
        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Proposal should not be approved yet (only 1 signature from proposer)
        let proposal = coordinator.proposals.get(&proposal_id).unwrap();
        assert!(!proposal.is_approved());
        assert_eq!(proposal.approval_count(), 1);

        // Add more signatures
        coordinator.approve(&proposal_id, "signer_1").unwrap();
        coordinator.approve(&proposal_id, "signer_2").unwrap();

        // Now should be approved (3 signatures)
        let proposal = coordinator.proposals.get(&proposal_id).unwrap();
        assert!(proposal.is_approved());
        assert_eq!(proposal.approval_count(), MULTISIG_THRESHOLD);
    }

    #[test]
    fn test_multisig_fails_without_quorum() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Only add 1 more signature (total 2, need 3)
        coordinator.approve(&proposal_id, "signer_1").unwrap();

        // Try to execute - should fail
        let result = coordinator.execute(&proposal_id, payload);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Insufficient approvals"));
    }

    #[test]
    fn test_multisig_succeeds_with_required_signatures() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Add required signatures
        coordinator.approve(&proposal_id, "signer_1").unwrap();
        coordinator.approve(&proposal_id, "signer_2").unwrap();

        // Execute should succeed
        let result = coordinator.execute(&proposal_id, payload);
        assert!(result.is_ok());

        // Verify proposal is marked as executed
        let proposal = coordinator.proposals.get(&proposal_id).unwrap();
        assert!(proposal.executed);
    }

    #[test]
    fn test_multisig_prevents_duplicate_signatures() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Try to approve twice with same signer
        let result1 = coordinator.approve(&proposal_id, "signer_1");
        assert!(result1.is_ok());

        let result2 = coordinator.approve(&proposal_id, "signer_1");
        assert!(result2.is_err());
        assert!(result2
            .unwrap_err()
            .contains("already approved"));
    }

    #[test]
    fn test_multisig_rejects_unauthorized_signer() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers);

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Unauthorized signer tries to approve
        let result = coordinator.approve(&proposal_id, "unauthorized_signer");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not an authorized signer"));
    }

    #[test]
    fn test_multisig_tracks_approval_count_correctly() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Check initial count (proposer auto-approves)
        let proposal = coordinator.proposals.get(&proposal_id).unwrap();
        assert_eq!(proposal.approval_count(), 1);

        // Add signatures one by one
        for i in 1..MULTISIG_TOTAL {
            coordinator
                .approve(&proposal_id, &format!("signer_{}", i))
                .unwrap();
            let proposal = coordinator.proposals.get(&proposal_id).unwrap();
            assert_eq!(proposal.approval_count(), i + 1);
        }
    }

    #[test]
    fn test_multisig_execution_prevented_after_rejection() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        // Mark as rejected
        let proposal = coordinator.proposals.get_mut(&proposal_id).unwrap();
        proposal.rejected = true;

        // Try to execute
        let result = coordinator.execute(&proposal_id, payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Proposal rejected"));
    }

    #[test]
    fn test_multisig_stores_total_signers_at_creation() {
        let signers: Vec<String> = (0..MULTISIG_TOTAL)
            .map(|i| format!("signer_{}", i))
            .collect();
        let mut coordinator = MultiSigCoordinator::new(signers.clone());

        let payload = b"test payload";
        let proposal_id = coordinator
            .propose("signer_0", "Test proposal", payload)
            .unwrap();

        let proposal = coordinator.proposals.get(&proposal_id).unwrap();
        assert_eq!(proposal.total_signers, MULTISIG_TOTAL);
        assert_eq!(proposal.required_signatures, MULTISIG_THRESHOLD);
    }
}
