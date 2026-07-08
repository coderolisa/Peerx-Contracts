use soroban_sdk::{Env, Symbol, Map};

pub fn simulate(env: &Env, proposal_id: Symbol) -> i32 {
    // Example: calculate simulated effect of proposal
    let effects: Map<Symbol, i32> = env.storage().get_unchecked(&"proposal_effects").unwrap_or_default();
    *effects.get(&proposal_id).unwrap_or(&0)
}