use soroban_sdk::{
    Env, testutils::{Accounts, Ledger},
};
use crate::contract::PeerXContract;

struct TestContext {
    env: Env,
    admin: soroban_sdk::Address,
    users: Vec<soroban_sdk::Address>,
}


fn setup() -> TestContext {
    let env = Env::default();
    env.mock_all_auths();

    let admin = env.accounts().generate();
    let users = (0..5)
        .map(|_| env.accounts().generate())
        .collect();

    // Optional: initialize contract
    // PeerXContract::initialize(&env, &admin);

    TestContext { env, admin, users }
}

#[test]
fn test_user_onboarding_and_initial_balances() {
    // Scenario:
    // User creates account → mints XLM + USDC → verifies balances

    let ctx = setup();
    let user = &ctx.users[0];

    // Mint tokens
    PeerXContract::mint_xlm(&ctx.env, user, &1000);
    PeerXContract::mint_usdc(&ctx.env, user, &500);

    // Verify balances
    let xlm = PeerXContract::balance_xlm(&ctx.env, user);
    let usdc = PeerXContract::balance_usdc(&ctx.env, user);

    assert_eq!(xlm, 1000);
    assert_eq!(usdc, 500);
}


#[test]
fn test_user_trading_flow_and_pnl() {
    // Scenario:
    // User performs multiple swaps → verifies trade history and PnL

    let ctx = setup();
    let user = &ctx.users[0];

    PeerXContract::mint_usdc(&ctx.env, user, &1000);

    // Perform swaps
    PeerXContract::swap_usdc_to_xlm(&ctx.env, user, &100);
    PeerXContract::swap_xlm_to_usdc(&ctx.env, user, &50);
    PeerXContract::swap_usdc_to_xlm(&ctx.env, user, &200);

    let history = PeerXContract::trade_history(&ctx.env, user);
    let pnl = PeerXContract::pnl(&ctx.env, user);

    assert_eq!(history.len(), 3);
    assert!(pnl != 0);
}


#[test]
fn test_multi_user_trading_isolated_balances() {
    // Scenario:
    // 3 users trade sequentially → balances remain isolated

    let ctx = setup();
    let (u1, u2, u3) = (&ctx.users[0], &ctx.users[1], &ctx.users[2]);

    for user in [u1, u2, u3] {
        PeerXContract::mint_usdc(&ctx.env, user, &500);
        PeerXContract::swap_usdc_to_xlm(&ctx.env, user, &100);
    }

    assert_ne!(
        PeerXContract::balance_xlm(&ctx.env, u1),
        PeerXContract::balance_xlm(&ctx.env, u2),
    );
}


#[test]
fn test_lp_and_trader_coexistence() {
    // Scenario:
    // LP provides liquidity → trader swaps from pool → pool updated

    let ctx = setup();
    let lp = &ctx.users[0];
    let trader = &ctx.users[1];

    PeerXContract::mint_usdc(&ctx.env, lp, &5000);
    PeerXContract::add_liquidity(&ctx.env, lp, &2000);

    PeerXContract::mint_usdc(&ctx.env, trader, &500);
    PeerXContract::swap_usdc_to_xlm(&ctx.env, trader, &200);

    let pool = PeerXContract::pool_state(&ctx.env);
    assert!(pool.total_liquidity > 0);
}


#[test]
fn test_badge_progression_after_multiple_trades() {
    // Scenario:
    // User trades enough times → earns multiple badges

    let ctx = setup();
    let user = &ctx.users[0];

    PeerXContract::mint_usdc(&ctx.env, user, &2000);

    for _ in 0..10 {
        PeerXContract::swap_usdc_to_xlm(&ctx.env, user, &50);
    }

    let badges = PeerXContract::badges(&ctx.env, user);
    assert!(badges.len() >= 2);
}


#[test]
fn test_state_persistence_across_restart() {
    // Scenario:
    // Contract state persists across env reset

    let ctx = setup();
    let user = &ctx.users[0];

    PeerXContract::mint_usdc(&ctx.env, user, &500);

    let snapshot = ctx.env.clone();

    // Simulate restart
    let new_env = snapshot;

    let balance = PeerXContract::balance_usdc(&new_env, user);
    assert_eq!(balance, 500);
}

#[test]
fn test_invalid_trade_does_not_corrupt_state() {
    // Scenario:
    // Invalid trade attempt → balances unchanged

    let ctx = setup();
    let user = &ctx.users[0];

    PeerXContract::mint_usdc(&ctx.env, user, &100);

    let result = std::panic::catch_unwind(|| {
        PeerXContract::swap_usdc_to_xlm(&ctx.env, user, &10_000);
    });

    assert!(result.is_err());

    let balance = PeerXContract::balance_usdc(&ctx.env, user);
    assert_eq!(balance, 100);
}


ctx.env.ledger().set(Ledger {
    timestamp: ctx.env.ledger().timestamp() + 60,
    ..Default::default()
});
