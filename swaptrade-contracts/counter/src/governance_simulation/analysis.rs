use soroban_sdk::{Env, Symbol};

pub fn analyze_impact(env: &Env, proposal_id: Symbol) -> i32 {
    // Example: risk/impact score between 0-100
    let base_impact: i32 = env.storage().get_unchecked(&format!("impact_{}", proposal_id)).unwrap_or(50);
    base_impact
}