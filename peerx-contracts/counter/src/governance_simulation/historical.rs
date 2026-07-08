use soroban_sdk::{Env, Symbol};

pub fn get_outcome(env: &Env, proposal_id: Symbol) -> i32 {
    // Example: return historical approval percentage
    env.storage().get_unchecked(&format!("historical_{}", proposal_id)).unwrap_or(0)
}