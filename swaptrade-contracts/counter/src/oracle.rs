use soroban_sdk::{contracttype, symbol_short, Env, Symbol};

const DEFAULT_PRICE_UPDATE_TOLERANCE_BPS: u32 = 10;

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    InvalidPrice = 1,
    StalePrice = 2,
    SlippageExceeded = 3,
    PriceNotSet = 4,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PriceData {
    pub price: u128,
    pub timestamp: u64,
}

pub trait PriceFeed {
    fn get_price(env: &Env, token_pair: (Symbol, Symbol)) -> Result<u128, ContractError>;
    fn last_update_time(env: &Env, token_pair: (Symbol, Symbol)) -> u64;
    fn set_price(env: &Env, token_pair: (Symbol, Symbol), price: u128);
}

fn tolerance_key(pair: &(Symbol, Symbol)) -> (Symbol, Symbol, Symbol) {
    (symbol_short!("TOL"), pair.0.clone(), pair.1.clone())
}

pub fn get_price_update_tolerance_bps(env: &Env, pair: (Symbol, Symbol)) -> u32 {
    let key = tolerance_key(&pair);
    env.storage().instance().get(&key).unwrap_or(DEFAULT_PRICE_UPDATE_TOLERANCE_BPS)
}

pub fn set_price_update_tolerance_bps(env: &Env, pair: (Symbol, Symbol), bps: u32) {
    let key = tolerance_key(&pair);
    env.storage().instance().set(&key, &bps);
}

pub fn get_stored_price(env: &Env, pair: (Symbol, Symbol)) -> Option<PriceData> {
    env.storage().instance().get(&pair)
}

fn price_delta_exceeds_tolerance(last_price: u128, new_price: u128, tolerance_bps: u32) -> bool {
    if last_price == 0 {
        return true;
    }
    let delta = if new_price >= last_price {
        new_price - last_price
    } else {
        last_price - new_price
    };
    let threshold = (last_price as u128).saturating_mul(tolerance_bps as u128) / 10_000;
    delta > threshold
}

pub fn set_stored_price(env: &Env, pair: (Symbol, Symbol), price: u128) {
    let existing = get_stored_price(env, pair.clone());
    let should_persist = match existing {
        None => true,
        Some(data) => price_delta_exceeds_tolerance(
            data.price,
            price,
            get_price_update_tolerance_bps(env, pair.clone()),
        ),
    };
    if should_persist {
        let timestamp = env.ledger().timestamp();
        let data = PriceData { price, timestamp };
        env.storage().instance().set(&pair, &data);
    }
}

pub fn get_price_safe(env: &Env, pair: (Symbol, Symbol)) -> Result<u128, ContractError> {
    match get_stored_price(env, pair) {
        Some(data) => Ok(data.price),
        None => Err(ContractError::PriceNotSet),
    }
}
