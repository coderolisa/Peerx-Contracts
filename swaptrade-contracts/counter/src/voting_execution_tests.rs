// tests/voting_execution_tests.rs
//! Tests for preventing double execution of governance proposals (Issue #167)

#[cfg(test)]
mod tests {
    use crate::governance::voting::{GovernanceVoting, ProposalStatus};

    #[test]
    fn test_proposal_cannot_be_executed_twice() {
        let mut gov = GovernanceVoting::new();
        let id = gov.create_proposal(
            "Test proposal".to_string(),
            "alice".to_string(),
            2,
            3600,
            0,
        );

        // Vote to pass
        gov.vote(id, "alice", true, 100).unwrap();
        gov.vote(id, "bob", true, 200).unwrap();

        // Finalize - should pass
        let status = gov.finalize(id, 300).unwrap();
        assert_eq!(status, ProposalStatus::Passed);

        // Execute first time - should succeed
        let result = gov.execute_proposal(id);
        assert!(result.is_ok());

        // Check status is now Executed
        let proposal = gov.get(id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);

        // Try to execute again - should fail
        let result = gov.execute_proposal(id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already been executed"));
    }

    #[test]
    fn test_failed_proposal_cannot_be_executed() {
        let mut gov = GovernanceVoting::new();
        let id = gov.create_proposal(
            "Test proposal".to_string(),
            "alice".to_string(),
            2,
            3600,
            0,
        );

        // Vote against
        gov.vote(id, "alice", false, 100).unwrap();
        gov.vote(id, "bob", false, 200).unwrap();

        // Finalize - should be rejected
        let status = gov.finalize(id, 300).unwrap();
        assert_eq!(status, ProposalStatus::Rejected);

        // Try to execute - should fail
        let result = gov.execute_proposal(id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be executed"));
    }

    #[test]
    fn test_expired_proposal_cannot_be_executed() {
        let mut gov = GovernanceVoting::new();
        let id = gov.create_proposal(
            "Test proposal".to_string(),
            "alice".to_string(),
            2,
            60,
            0,
        );

        // Try to vote after expiry
        let result = gov.vote(id, "alice", true, 120);
        assert!(result.is_err());

        // Finalize - should be expired
        let status = gov.finalize(id, 120).unwrap();
        assert_eq!(status, ProposalStatus::Expired);

        // Try to execute - should fail
        let result = gov.execute_proposal(id);
        assert!(result.is_err());
    }

    #[test]
    fn test_active_proposal_cannot_be_executed() {
        let mut gov = GovernanceVoting::new();
        let id = gov.create_proposal(
            "Test proposal".to_string(),
            "alice".to_string(),
            2,
            3600,
            0,
        );

        // Only one vote - not finalized yet
        gov.vote(id, "alice", true, 100).unwrap();

        // Try to execute before finalization - should fail
        let result = gov.execute_proposal(id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be executed"));
    }

    #[test]
    fn test_execution_state_transitions_are_correct() {
        let mut gov = GovernanceVoting::new();
        let id = gov.create_proposal(
            "Test proposal".to_string(),
            "alice".to_string(),
            2,
            3600,
            0,
        );

        // Initial state
        let proposal = gov.get(id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Active);

        // Vote and finalize
        gov.vote(id, "alice", true, 100).unwrap();
        gov.vote(id, "bob", true, 200).unwrap();
        gov.finalize(id, 300).unwrap();

        let proposal = gov.get(id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Passed);

        // Execute
        gov.execute_proposal(id).unwrap();

        let proposal = gov.get(id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[test]
    fn test_multiple_proposals_have_independent_execution_state() {
        let mut gov = GovernanceVoting::new();

        let id1 = gov.create_proposal(
            "Proposal 1".to_string(),
            "alice".to_string(),
            2,
            3600,
            0,
        );

        let id2 = gov.create_proposal(
            "Proposal 2".to_string(),
            "bob".to_string(),
            2,
            3600,
            0,
        );

        // Pass both proposals
        gov.vote(id1, "alice", true, 100).unwrap();
        gov.vote(id1, "bob", true, 200).unwrap();
        gov.finalize(id1, 300).unwrap();

        gov.vote(id2, "alice", true, 100).unwrap();
        gov.vote(id2, "bob", true, 200).unwrap();
        gov.finalize(id2, 300).unwrap();

        // Execute first proposal
        gov.execute_proposal(id1).unwrap();

        // Verify first is executed, second is still passed
        let proposal1 = gov.get(id1).unwrap();
        assert_eq!(proposal1.status, ProposalStatus::Executed);

        let proposal2 = gov.get(id2).unwrap();
        assert_eq!(proposal2.status, ProposalStatus::Passed);

        // Execute second proposal
        gov.execute_proposal(id2).unwrap();

        let proposal2 = gov.get(id2).unwrap();
        assert_eq!(proposal2.status, ProposalStatus::Executed);

        // Verify first cannot be executed again
        let result = gov.execute_proposal(id1);
        assert!(result.is_err());
    }
}
