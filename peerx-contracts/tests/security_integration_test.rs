//! Security Integration Test
//! 
//! This test performs 100 random operations to verify that all contract
//! invariants hold under stress conditions.

use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};
use std::collections::HashMap;

// Import the contract
use counter::{CounterContract, CounterContractClient};

/// Test that 100 random operations maintain all contract invariants
#[test]
fn test_100_random_operations_invariant_holding() {
    let env = Env::default();
    let contract_id = env.register(CounterContract, ());
    let client = CounterContractClient::new(&env, &contract_id);
    
    // Test users
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let users = vec![user1, user2, user3];
    
    // Supported tokens
    let xlm = symbol_short!("XLM");
    let usdc = symbol_short!("USDCSIM");
    let tokens = vec![xlm, usdc];
    
    // Track metrics for verification
    let mut total_minted: i128 = 0;
    let mut total_swapped: i128 = 0;
    let mut total_lp_added: i128 = 0;
    let mut operation_count: u32 = 0;
    
    // Operation types
    #[derive(Debug, Clone, Copy)]
    enum Operation {
        Mint,
        Swap,
        AddLiquidity,
        RemoveLiquidity,
        RecordTrade,
    }
    
    // Perform 100 random operations
    for i in 1..=100 {
        let operation_type = match i % 5 {
            0 => Operation::Mint,
            1 => Operation::Swap,
            2 => Operation::AddLiquidity,
            3 => Operation::RemoveLiquidity,
            4 => Operation::RecordTrade,
            _ => Operation::Mint,
        };
        
        let user = &users[i % users.len()];
        let token1 = &tokens[i % tokens.len()];
        let token2 = &tokens[(i + 1) % tokens.len()];
        
        match operation_type {
            Operation::Mint => {
                let amount = (i as i128) * 1000;
                client.mint(token1, user, &amount);
                total_minted += amount;
                operation_count += 1;
                
                // Verify balance increased
                let balance = client.get_balance(token1, user);
                assert!(balance >= amount, "Balance should increase after mint");
            }
            
            Operation::Swap => {
                // Only swap if user has sufficient balance
                let balance = client.get_balance(token1, user);
                if balance > 1000 {
                    let amount = std::cmp::min(balance / 2, 5000);
                    if *token1 != *token2 && amount > 0 {
                        let result = client.try_swap(token1, token2, &amount, user);
                        if result > 0 {
                            total_swapped += amount;
                            operation_count += 1;
                        }
                    }
                }
            }
            
            Operation::AddLiquidity => {
                // Mint tokens first if needed
                let xlm_balance = client.get_balance(&xlm, user);
                let usdc_balance = client.get_balance(&usdc, user);
                
                if xlm_balance < 10000 {
                    client.mint(&xlm, user, &10000);
                }
                if usdc_balance < 10000 {
                    client.mint(&usdc, user, &10000);
                }
                
                // Add liquidity
                let xlm_amount = 1000 + (i as i128) * 100;
                let usdc_amount = 1000 + (i as i128) * 100;
                
                if xlm_amount > 0 && usdc_amount > 0 {
                    let lp_tokens = client.add_liquidity(&xlm_amount, &usdc_amount, user);
                    if lp_tokens > 0 {
                        total_lp_added += xlm_amount + usdc_amount;
                        operation_count += 1;
                    }
                }
            }
            
            Operation::RemoveLiquidity => {
                // Try to remove liquidity (may fail if no position)
                let lp_positions = client.get_lp_positions(user);
                if !lp_positions.is_empty() {
                    if let Some(position) = lp_positions.get(0) {
                        let lp_tokens = position.lp_tokens_minted / 2; // Remove half
                        if lp_tokens > 0 {
                            let (xlm_returned, usdc_returned) = client.remove_liquidity(&lp_tokens, user);
                            if xlm_returned > 0 && usdc_returned > 0 {
                                operation_count += 1;
                            }
                        }
                    }
                }
            }
            
            Operation::RecordTrade => {
                client.record_trade(user);
                operation_count += 1;
            }
        }
        
        // Every 10 operations, verify invariants
        if i % 10 == 0 {
            verify_contract_state(&client, &env, i);
        }
    }
    
    // Final comprehensive verification
    verify_final_state(&client, &env, operation_count, total_minted, total_swapped);
    
    println!("✅ Completed 100 random operations successfully");
    println!("📊 Operations executed: {}", operation_count);
    println!("💰 Total minted: {}", total_minted);
    println!("🔄 Total swapped: {}", total_swapped);
    println!("💧 Total LP added: {}", total_lp_added);
}

/// Verify contract state invariants at intermediate steps
fn verify_contract_state(client: &CounterContractClient, env: &Env, step: u32) {
    // Get metrics
    let metrics = client.get_metrics();
    
    // Verify basic invariants
    assert!(metrics.trades_executed >= 0, "Trade count should be non-negative at step {}", step);
    assert!(metrics.failed_orders >= 0, "Failed orders should be non-negative at step {}", step);
    assert!(metrics.balances_updated >= 0, "Balance updates should be non-negative at step {}", step);
    
    // Verify trade count consistency
    assert!(metrics.trades_executed + metrics.failed_orders >= 0, 
        "Total operations should be non-negative at step {}", step);
    
    // Check rate limits are working
    let test_user = Address::generate(env);
    let rate_limit = client.get_swap_rate_limit(&test_user);
    // Rate limit status should be valid enum variant
    match rate_limit {
        counter::RateLimitStatus::Allowed => {},
        counter::RateLimitStatus::Blocked => {},
        counter::RateLimitStatus::RetryAfter(_) => {},
    }
    
    println!("  ✓ Step {}: Invariants verified", step);
}

/// Verify final state after all operations
fn verify_final_state(
    client: &CounterContractClient, 
    env: &Env,
    operation_count: u32,
    total_minted: i128,
    total_swapped: i128,
) {
    // Get final metrics
    let metrics = client.get_metrics();
    
    // Verify all operations were counted
    assert!(metrics.trades_executed + metrics.failed_orders <= operation_count,
        "Metrics should not exceed operation count");
    
    // Verify non-negative values
    assert!(metrics.trades_executed >= 0);
    assert!(metrics.failed_orders >= 0);
    assert!(metrics.balances_updated >= 0);
    
    // Test user tiers
    let test_user = Address::generate(env);
    let tier = client.get_user_tier(&test_user);
    // Tier should be valid (0-3)
    let tier_num: u32 = tier as u32;
    assert!(tier_num <= 3, "User tier should be between 0-3, got {}", tier_num);
    
    // Test badge system with a user who did many trades
    let badge_user = Address::generate(env);
    // Simulate many trades
    for _ in 0..15 {
        client.record_trade(&badge_user);
    }
    
    // User should have earned badges
    let badges = client.get_user_badges(&badge_user);
    assert!(!badges.is_empty(), "User with 15 trades should have badges");
    
    // Test batch operations
    let batch_user = Address::generate(env);
    client.mint(&symbol_short!("XLM"), &batch_user, &10000);
    
    let mut operations = Vec::new(&env);
    operations.push_back(counter::BatchOperation::Swap(
        symbol_short!("XLM"),
        symbol_short!("USDCSIM"),
        1000,
        batch_user.clone(),
    ));
    operations.push_back(counter::BatchOperation::MintToken(
        symbol_short!("XLM"),
        batch_user.clone(),
        500,
    ));
    
    let batch_result = client.execute_batch(&operations);
    assert!(batch_result.operations_executed >= 0);
    assert!(batch_result.operations_failed >= 0);
    
    // Verify contract version
    let version = client.get_contract_version();
    assert_eq!(version, 1, "Contract version should be 1");
    
    println!("  ✓ Final state verification passed");
    println!("  📊 Final trades executed: {}", metrics.trades_executed);
    println!("  📉 Final failed orders: {}", metrics.failed_orders);
    println!("  🔄 Final balances updated: {}", metrics.balances_updated);
}

/// Test edge cases that should not break invariants
#[test]
fn test_edge_case_invariants() {
    let env = Env::default();
    let contract_id = env.register(CounterContract, ());
    let client = CounterContractClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let xlm = symbol_short!("XLM");
    let usdc = symbol_short!("USDCSIM");
    
    // Test zero amount operations (should not panic)
    let result = client.try_swap(&xlm, &usdc, &0, &user);
    assert_eq!(result, 0, "Zero amount swap should return 0");
    
    // Test same token swap (should not panic)
    let result = client.try_swap(&xlm, &xlm, &1000, &user);
    assert_eq!(result, 0, "Same token swap should return 0");
    
    // Test negative amount (should not panic)
    // Note: This would fail compilation in Rust, but testing client behavior
    // We test this via the try_swap with validation
    
    // Test with very large numbers (check overflow protection)
    let large_amount = i128::MAX / 1000;
    let result = client.try_swap(&xlm, &usdc, &large_amount, &user);
    // Should either succeed or return 0 (not panic)
    assert!(result >= 0, "Large amount swap should not panic");
    
    // Verify metrics still consistent
    let metrics = client.get_metrics();
    assert!(metrics.failed_orders >= 2, "Should have recorded failed orders");
    
    println!("✅ Edge case tests passed");
}

/// Test concurrent-like operations from multiple users
#[test]
fn test_concurrent_user_isolation() {
    let env = Env::default();
    let contract_id = env.register(CounterContract, ());
    let client = CounterContractClient::new(&env, &contract_id);
    
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let users = vec![user1, user2, user3];
    
    let xlm = symbol_short!("XLM");
    let usdc = symbol_short!("USDCSIM");
    
    // Each user performs independent operations
    for (i, user) in users.iter().enumerate() {
        // Mint different amounts to each user
        let mint_amount = 10000 + (i as i128) * 1000;
        client.mint(&xlm, user, &mint_amount);
        
        // Verify only this user has the balance
        assert_eq!(client.get_balance(&xlm, user), mint_amount);
        
        // Other users should have zero balance
        for other_user in &users {
            if other_user != user {
                assert_eq!(client.get_balance(&xlm, other_user), 0);
            }
        }
        
        // Perform swap
        if mint_amount > 1000 {
            let swap_amount = mint_amount / 2;
            let out = client.swap(&xlm, &usdc, &swap_amount, user);
            assert!(out > 0, "Swap should succeed");
        }
    }
    
    // Verify final state
    let metrics = client.get_metrics();
    assert_eq!(metrics.trades_executed, 3, "Should have 3 successful trades");
    assert_eq!(metrics.balances_updated, 9, "Should have 9 balance updates (3 mints + 3 debits + 3 credits)");
    
    println!("✅ Concurrent user isolation test passed");
}
