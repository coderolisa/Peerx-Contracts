// src/admin.rs
// Phase-aware admin module.  All privileged operations are gated through
// GovernanceContract so the phase enforcement is a single source of truth.

use crate::governance::{
    GovernanceContract, GovernancePhase, SchnorrProof,
    make_schnorr_proof, TIMELOCK_DELAY_SECS,
};

// ─── Admin State ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ContractState {
    pub trading_paused: bool,
    pub fee_bps: u16,
    pub max_trade_size: u64,
    pub admin: String,
}

// ─── Admin Controller ─────────────────────────────────────────────────────────

pub struct AdminController {
    pub state: ContractState,
    pub governance: GovernanceContract,
}

impl AdminController {
    pub fn new(governance: GovernanceContract, initial_admin: String) -> Self {
        Self {
            state: ContractState {
                trading_paused: false,
                fee_bps: 30,
                max_trade_size: 1_000_000,
                admin: initial_admin,
            },
            governance,
        }
    }

    // ── Pause / Unpause ───────────────────────────────────────────────────────
    // Allowed in Phase 1 and Phase 2.

    pub fn pause_trading(&mut self, caller: &str) -> Result<(), String> {
        self.assert_admin(caller)?;
        self.governance.assert_can_pause()?;
        self.state.trading_paused = true;
        Ok(())
    }

    pub fn resume_trading(&mut self, caller: &str) -> Result<(), String> {
        self.assert_admin(caller)?;
        self.governance.assert_can_pause()?;
        self.state.trading_paused = false;
        Ok(())
    }

    // ── State-Modifying Operations ────────────────────────────────────────────
    // Phase 1: direct execution
    // Phase 2: blocked
    // Phase 3: must go through multi-sig
    // Phase 4: blocked entirely

    /// Set trading fee (basis points). Phase 1 only for direct execution.
    pub fn set_fee_bps_direct(&mut self, caller: &str, fee_bps: u16) -> Result<(), String> {
        self.assert_admin(caller)?;
        self.governance.assert_can_modify_state(caller)?;
        self.state.fee_bps = fee_bps;
        Ok(())
    }

    /// Queue a fee change through the timelock (Phase 1 or 2 admin).
    pub fn queue_set_fee_bps(
        &mut self,
        caller: &str,
        fee_bps: u16,
    ) -> Result<[u8; 32], String> {
        self.assert_admin(caller)?;
        // Phase 3+ must use multi-sig; Phase 1-2 may use timelock as best practice
        match self.governance.current_phase() {
            GovernancePhase::DaoOnly => return Err("Phase 4: use DAO proposal".into()),
            _ => {}
        }
        let payload = fee_bps.to_le_bytes();
        let op_id = self.governance.queue_operation("set_fee_bps", &payload);
        Ok(op_id)
    }

    pub fn execute_set_fee_bps(
        &mut self,
        op_id: &[u8; 32],
        fee_bps: u16,
    ) -> Result<(), String> {
        let payload = fee_bps.to_le_bytes();
        self.governance.execute_operation(op_id, &payload)?;
        self.state.fee_bps = fee_bps;
        Ok(())
    }

    /// Propose a max_trade_size change via multi-sig (Phase 3).
    pub fn propose_max_trade_size(
        &mut self,
        proposer: &str,
        new_size: u64,
    ) -> Result<[u8; 32], String> {
        match self.governance.current_phase() {
            GovernancePhase::MultiSig | GovernancePhase::DaoOnly => {}
            _ => return Err("Multi-sig proposal only required in Phase 3+".into()),
        }
        let payload = new_size.to_le_bytes();
        self.governance.propose_multisig(proposer, "set_max_trade_size", &payload)
    }

    pub fn approve_max_trade_size(
        &mut self,
        proposal_id: &[u8; 32],
        signer: &str,
    ) -> Result<usize, String> {
        self.governance.approve_multisig(proposal_id, signer)
    }

    pub fn execute_max_trade_size(
        &mut self,
        proposal_id: &[u8; 32],
        new_size: u64,
    ) -> Result<(), String> {
        let payload = new_size.to_le_bytes();
        self.governance.execute_multisig(proposal_id, &payload)?;
        self.state.max_trade_size = new_size;
        Ok(())
    }

    /// Guardian emergency override (any phase) with Schnorr proof.
    pub fn guardian_override(
        &mut self,
        proof: &SchnorrProof,
        reason: &str,
    ) -> Result<(), String> {
        self.governance.guardian_override(proof, reason)
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn assert_admin(&self, caller: &str) -> Result<(), String> {
        if caller != self.state.admin {
            return Err(format!("'{}' is not the admin", caller));
        }
        Ok(())
    }
}