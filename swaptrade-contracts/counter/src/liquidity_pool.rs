use soroban_sdk::{contracttype, Address, Env, Map, Symbol, Vec};
use crate::errors::ContractError;

const MIN_LIQUIDITY: i128 = 1000;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct LiquidityPool {
    pub pool_id: u64,
    pub token_a: Symbol,
    pub token_b: Symbol,
    pub reserve_a: i128,
    pub reserve_b: i128,
    pub total_lp_tokens: i128,
    pub fee_tier: u32, // basis points: 1=0.01%, 5=0.05%, 30=0.30%
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Route {
    pub pools: Vec<u64>,
    pub tokens: Vec<Symbol>,
    pub expected_output: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct PoolRegistry {
    pools: Map<u64, LiquidityPool>,
    pair_to_pool: Map<(Symbol, Symbol), u64>,
    next_pool_id: u64,
    lp_balances: Map<(u64, Address), i128>,
}

impl PoolRegistry {
    pub fn new(env: &Env) -> Self {
        Self {
            pools: Map::new(env),
            pair_to_pool: Map::new(env),
            next_pool_id: 1,
            lp_balances: Map::new(env),
        }
    }

    fn normalize_pair(token_a: Symbol, token_b: Symbol) -> (Symbol, Symbol) {
        if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        }
    }

    pub fn register_pool(
        &mut self,
        env: &Env,
        token_a: Symbol,
        token_b: Symbol,
        initial_a: i128,
        initial_b: i128,
        fee_tier: u32,
    ) -> Result<u64, ContractError> {
        if token_a == token_b {
            return Err(ContractError::InvalidSwapPair);
        }
        if initial_a <= 0 || initial_b <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let (norm_a, norm_b) = Self::normalize_pair(token_a.clone(), token_b.clone());
        
        if self.pair_to_pool.contains_key((norm_a.clone(), norm_b.clone())) {
            return Err(ContractError::InvalidSwapPair);
        }

        let pool_id = self.next_pool_id;
        let (reserve_a, reserve_b) = if token_a == norm_a {
            (initial_a, initial_b)
        } else {
            (initial_b, initial_a)
        };

        let initial_lp = Self::sqrt((reserve_a as u128) * (reserve_b as u128)) as i128;
        
        let pool = LiquidityPool {
            pool_id,
            token_a: norm_a.clone(),
            token_b: norm_b.clone(),
            reserve_a,
            reserve_b,
            total_lp_tokens: initial_lp,
            fee_tier,
        };

        self.pools.set(pool_id, pool);
        self.pair_to_pool.set((norm_a, norm_b), pool_id);
        self.next_pool_id += 1;

        Ok(pool_id)
    }

    pub fn add_liquidity(
        &mut self,
        env: &Env,
        pool_id: u64,
        amount_a: i128,
        amount_b: i128,
        provider: Address,
    ) -> Result<i128, ContractError> {
        let mut pool = self.pools.get(pool_id).ok_or(ContractError::LPPositionNotFound)?;
        
        if amount_a <= 0 || amount_b <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let lp_tokens = if pool.total_lp_tokens == 0 {
            Self::sqrt((amount_a as u128) * (amount_b as u128)) as i128
        } else {
            let lp_a = (amount_a * pool.total_lp_tokens) / pool.reserve_a;
            let lp_b = (amount_b * pool.total_lp_tokens) / pool.reserve_b;
            lp_a.min(lp_b)
        };

        if lp_tokens <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        pool.reserve_a += amount_a;
        pool.reserve_b += amount_b;
        pool.total_lp_tokens += lp_tokens;
        self.pools.set(pool_id, pool);

        let key = (pool_id, provider);
        let current = self.lp_balances.get(key.clone()).unwrap_or(0);
        self.lp_balances.set(key, current + lp_tokens);

        Ok(lp_tokens)
    }

    pub fn remove_liquidity(
        &mut self,
        env: &Env,
        pool_id: u64,
        lp_tokens: i128,
        provider: Address,
    ) -> Result<(i128, i128), ContractError> {
        let mut pool = self.pools.get(pool_id).ok_or(ContractError::LPPositionNotFound)?;
        
        let key = (pool_id, provider);
        let balance = self.lp_balances.get(key.clone()).unwrap_or(0);
        
        if balance < lp_tokens {
            return Err(ContractError::InsufficientLPTokens);
        }

        let amount_a = (lp_tokens * pool.reserve_a) / pool.total_lp_tokens;
        let amount_b = (lp_tokens * pool.reserve_b) / pool.total_lp_tokens;

        pool.reserve_a -= amount_a;
        pool.reserve_b -= amount_b;
        pool.total_lp_tokens -= lp_tokens;
        self.pools.set(pool_id, pool);

        self.lp_balances.set(key, balance - lp_tokens);

        Ok((amount_a, amount_b))
    }

    pub fn swap(
        &mut self,
        env: &Env,
        pool_id: u64,
        token_in: Symbol,
        amount_in: i128,
        min_amount_out: i128,
    ) -> Result<i128, ContractError> {
        let mut pool = self.pools.get(pool_id).ok_or(ContractError::LPPositionNotFound)?;
        
        if amount_in <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let (reserve_in, reserve_out) = if token_in == pool.token_a {
            (pool.reserve_a, pool.reserve_b)
        } else if token_in == pool.token_b {
            (pool.reserve_b, pool.reserve_a)
        } else {
            return Err(ContractError::InvalidTokenSymbol);
        };

        let amount_in_with_fee = (amount_in as u128) * (10000 - pool.fee_tier as u128) / 10000;
        let numerator = (reserve_out as u128) * amount_in_with_fee;
        let denominator = (reserve_in as u128) + amount_in_with_fee;
        let amount_out = (numerator / denominator) as i128;

        if amount_out < min_amount_out {
            return Err(ContractError::SlippageExceeded);
        }

        if token_in == pool.token_a {
            pool.reserve_a += amount_in;
            pool.reserve_b -= amount_out;
        } else {
            pool.reserve_b += amount_in;
            pool.reserve_a -= amount_out;
        }

        self.pools.set(pool_id, pool);
        Ok(amount_out)
    }

    pub fn find_best_route(
        &self,
        env: &Env,
        token_in: Symbol,
        token_out: Symbol,
        amount_in: i128,
    ) -> Option<Route> {
        // Direct pool
        let (norm_in, norm_out) = Self::normalize_pair(token_in.clone(), token_out.clone());
        if let Some(pool_id) = self.pair_to_pool.get((norm_in.clone(), norm_out.clone())) {
            if let Some(pool) = self.pools.get(pool_id) {
                let output = self.calculate_output(&pool, token_in.clone(), amount_in);
                let mut pools = Vec::new(env);
                pools.push_back(pool_id);
                let mut tokens = Vec::new(env);
                tokens.push_back(token_in);
                tokens.push_back(token_out);
                return Some(Route { pools, tokens, expected_output: output });
            }
        }

        // Multi-hop (simplified: check one intermediate)
        let mut best_route: Option<Route> = None;
        let mut best_output = 0i128;

        for i in 0..self.next_pool_id {
            if let Some(pool1) = self.pools.get(i) {
                if pool1.token_a == token_in || pool1.token_b == token_in {
                    let intermediate = if pool1.token_a == token_in { pool1.token_b.clone() } else { pool1.token_a.clone() };
                    
                    if intermediate != token_out {
                        let (norm_int, norm_out) = Self::normalize_pair(intermediate.clone(), token_out.clone());
                        if let Some(pool2_id) = self.pair_to_pool.get((norm_int, norm_out)) {
                            if let Some(pool2) = self.pools.get(pool2_id) {
                                let out1 = self.calculate_output(&pool1, token_in.clone(), amount_in);
                                let out2 = self.calculate_output(&pool2, intermediate.clone(), out1);
                                
                                if out2 > best_output {
                                    best_output = out2;
                                    let mut pools = Vec::new(env);
                                    pools.push_back(i);
                                    pools.push_back(pool2_id);
                                    let mut tokens = Vec::new(env);
                                    tokens.push_back(token_in.clone());
                                    tokens.push_back(intermediate);
                                    tokens.push_back(token_out.clone());
                                    best_route = Some(Route { pools, tokens, expected_output: out2 });
                                }
                            }
                        }
                    }
                }
            }
        }

        best_route
    }

    fn calculate_output(&self, pool: &LiquidityPool, token_in: Symbol, amount_in: i128) -> i128 {
        let (reserve_in, reserve_out) = if token_in == pool.token_a {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };

        let amount_in_with_fee = (amount_in as u128) * (10000 - pool.fee_tier as u128) / 10000;
        let numerator = (reserve_out as u128) * amount_in_with_fee;
        let denominator = (reserve_in as u128) + amount_in_with_fee;
        (numerator / denominator) as i128
    }

    pub fn get_pool(&self, pool_id: u64) -> Option<LiquidityPool> {
        self.pools.get(pool_id)
    }

    pub fn get_lp_balance(&self, pool_id: u64, provider: Address) -> i128 {
        self.lp_balances.get((pool_id, provider)).unwrap_or(0)
    }

    fn sqrt(y: u128) -> u128 {
        if y < 4 { return if y == 0 { 0 } else { 1 }; }
        let mut z = y;
        let mut x = y / 2 + 1;
        while x < z {
            z = x;
            x = (y / x + x) / 2;
        }
        z
    }
}
