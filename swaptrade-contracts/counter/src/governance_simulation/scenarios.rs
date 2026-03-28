use soroban_sdk::{Env, Symbol};

pub fn simulate_scenario(env: &Env, proposal_id: Symbol, scenario_id: i32) -> i32 {
    // Example: adjust proposal effect based on scenario
    let base: i32 = env.storage().get_unchecked(&format!("scenario_{}_{}", proposal_id, scenario_id)).unwrap_or(50);
    base
}