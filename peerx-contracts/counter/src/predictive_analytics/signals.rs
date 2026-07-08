use soroban_sdk::{Env, Symbol, Map};

pub fn store_signal(env: &Env, asset: Symbol, signal: i32, confidence: u8) {
    // Retrieve existing signals map or create a new one
    let mut signals: Map<Symbol, (i32, u8)> = env.storage().get_unchecked(&"signals").unwrap_or_default();
    signals.set(asset.clone(), (signal, confidence));
    env.storage().set(&"signals", &signals);
}

pub fn get_signal(env: &Env, asset: Symbol) -> (i32, u8) {
    let signals: Map<Symbol, (i32, u8)> = env.storage().get_unchecked(&"signals").unwrap_or_default();
    signals.get(asset).unwrap_or((0, 0))
}