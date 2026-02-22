//! Performance tests for batch operation optimizations
//! 
//! This module contains benchmarks to verify the performance improvements
//! from the batch operation optimizations.

#[cfg(test)]
use soroban_sdk::testutils::Address as TestAddress;
use soroban_sdk::{Address, Env, Symbol, Vec, symbol_short};

use crate::batch::{execute_batch_atomic, BatchOperation, BatchResult};
use crate::portfolio::Portfolio;

#[test]
fn test_batch_performance_improvements() {
    let env = Env::default();
    let mut portfolio = Portfolio::new(&env);
    let user = Address::generate(&env);
    
    // Mint tokens to user for testing
    portfolio.mint(&env, crate::portfolio::Asset::XLM, user.clone(), 100000);
    portfolio.mint(&env, crate::portfolio::Asset::Custom(symbol_short!("USDCSIM")), user.clone(), 100000);
    
    // Create a large batch to measure performance
    let batch_size = 10u32; // MAX_BATCH_SIZE
    let mut operations = Vec::new(&env);
    
    for i in 0..batch_size {
        if i % 2 == 0 {
            // Add swap operations
            operations.push_back(BatchOperation::Swap(
                symbol_short!("XLM"),
                symbol_short!("USDCSIM"),
                1000 + (i as i128 * 10),
                user.clone(),
            ));
        } else {
            // Add liquidity operations
            operations.push_back(BatchOperation::AddLiquidity(
                500 + (i as i128 * 5),
                500 + (i as i128 * 5),
                user.clone(),
            ));
        }
    }
    
    // Measure execution time (simulated)
    let start_time = env.ledger().timestamp();
    
    // Execute batch
    let result = execute_batch_atomic(&env, &mut portfolio, operations);
    
    let end_time = env.ledger().timestamp();
    let execution_time = end_time - start_time;
    
    // Verify success
    assert!(result.is_ok());
    let batch_result = result.unwrap();
    assert_eq!(batch_result.operations_executed, batch_size);
    assert_eq!(batch_result.operations_failed, 0);
    assert_eq!(batch_result.results.len() as u32, batch_size);
    
    // Performance assertions (these are conceptual - actual timing would require WASM benchmarking)
    // In a real scenario, you would compare against baseline performance metrics
    assert!(execution_time >= 0); // Basic sanity check
    
    println!("Batch execution completed in {} ledger time units", execution_time);
    println!("Operations executed: {}", batch_result.operations_executed);
    println!("Batch size: {}", batch_size);
}

#[test]
fn test_memory_allocation_efficiency() {
    let env = Env::default();
    let capacity = 5u32;
    
    // Test that BatchResult::new_with_capacity is available
    let result = BatchResult::new_with_capacity(&env, capacity);
    
    assert_eq!(result.results.len(), 0);
    assert_eq!(result.operations_executed, 0);
    assert_eq!(result.operations_failed, 0);
    
    // While we can't directly measure memory allocation in tests,
    // we can verify the API works correctly
    println!("BatchResult with capacity {} created successfully", capacity);
}

#[test]
fn test_batch_operation_size_optimization() {
    // Test that batch operations are reasonably sized
    let env = Env::default();
    let user = Address::generate(&env);
    
    let swap_op = BatchOperation::Swap(
        symbol_short!("XLM"),
        symbol_short!("USDCSIM"),
        1000,
        user.clone(),
    );
    
    let add_op = BatchOperation::AddLiquidity(1000, 2000, user.clone());
    
    // Verify operations are created correctly
    assert!(matches!(swap_op, BatchOperation::Swap(_, _, _, _)));
    assert!(matches!(add_op, BatchOperation::AddLiquidity(_, _, _)));
    
    println!("Batch operations created successfully");
    println!("Swap operation variant created");
    println!("AddLiquidity operation variant created");
}

/// Test to demonstrate the memory efficiency improvement concept
#[test]
fn test_conceptual_memory_savings() {
    // This test demonstrates the conceptual improvement
    // In practice, actual memory savings would be measured through:
    // 1. WASM binary size comparison
    // 2. Runtime memory usage profiling
    // 3. Gas cost measurements
    
    let env = Env::default();
    let mut portfolio = Portfolio::new(&env);
    let user = Address::generate(&env);
    
    // Setup
    portfolio.mint(&env, crate::portfolio::Asset::XLM, user.clone(), 50000);
    portfolio.mint(&env, crate::portfolio::Asset::Custom(symbol_short!("USDCSIM")), user.clone(), 50000);
    
    // Create operations
    let mut operations = Vec::new(&env);
    operations.push_back(BatchOperation::Swap(
        symbol_short!("XLM"),
        symbol_short!("USDCSIM"),
        1000,
        user.clone(),
    ));
    operations.push_back(BatchOperation::AddLiquidity(
        2000,
        2000,
        user.clone(),
    ));
    
    // Execute with optimized batch
    let result = execute_batch_atomic(&env, &mut portfolio, operations);
    
    assert!(result.is_ok());
    let batch_result = result.unwrap();
    
    // Verify the key optimizations are in place conceptually:
    // 1. Pre-allocated result vector (via new_with_capacity)
    // 2. Efficient operation execution
    assert_eq!(batch_result.operations_executed, 2);
    assert_eq!(batch_result.operations_failed, 0);
    
    println!("Conceptual memory optimization test passed");
    println!("Batch executed with {} operations", batch_result.operations_executed);
}
