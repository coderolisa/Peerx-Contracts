//! Simple test to verify batch optimization implementation
//! 
//! This test verifies that the basic batch functionality works
//! with the new optimizations in place.

use soroban_sdk::{testutils::Address as TestAddress, Address, Env, Symbol, Vec, symbol_short};

use crate::batch::{execute_batch_atomic, BatchOperation, BatchResult};
use crate::portfolio::{Portfolio, Asset};

#[test]
fn test_batch_optimization_basic_functionality() {
    let env = Env::default();
    let mut portfolio = Portfolio::new(&env);
    let user = Address::generate(&env);
    
    // Mint tokens to user
    portfolio.mint(&env, Asset::XLM, user.clone(), 10000);
    portfolio.mint(&env, Asset::Custom(symbol_short!("USDCSIM")), user.clone(), 10000);
    
    // Test BatchResult::new_with_capacity
    let batch_result = BatchResult::new_with_capacity(&env, 5);
    assert_eq!(batch_result.results.len(), 0);
    assert_eq!(batch_result.operations_executed, 0);
    assert_eq!(batch_result.operations_failed, 0);
    
    // Create simple batch operations
    let mut operations = Vec::new(&env);
    operations.push_back(BatchOperation::Swap(
        symbol_short!("XLM"),
        symbol_short!("USDCSIM"),
        1000,
        user.clone(),
    ));
    
    // Execute batch
    let result = execute_batch_atomic(&env, &mut portfolio, operations);
    
    // Should succeed
    assert!(result.is_ok());
    let batch_result = result.unwrap();
    assert_eq!(batch_result.operations_executed, 1);
    assert_eq!(batch_result.operations_failed, 0);
    assert_eq!(batch_result.results.len(), 1);
    
    println!("✅ Batch optimization basic functionality test passed");
    println!("✅ BatchResult::new_with_capacity works correctly");
    println!("✅ execute_batch_atomic executes successfully");
}

#[test]
fn test_batch_operation_variants() {
    let env = Env::default();
    let user = Address::generate(&env);
    
    // Test all batch operation variants can be created
    let swap_op = BatchOperation::Swap(
        symbol_short!("XLM"),
        symbol_short!("USDCSIM"),
        1000,
        user.clone(),
    );
    
    let add_op = BatchOperation::AddLiquidity(1000, 2000, user.clone());
    
    let remove_op = BatchOperation::RemoveLiquidity(500, 500, user.clone());
    
    let mint_op = BatchOperation::MintToken(
        symbol_short!("TOKEN"),
        user.clone(),
        100,
    );
    
    // Verify all operations are created correctly
    assert!(matches!(swap_op, BatchOperation::Swap(_, _, _, _)));
    assert!(matches!(add_op, BatchOperation::AddLiquidity(_, _, _)));
    assert!(matches!(remove_op, BatchOperation::RemoveLiquidity(_, _, _)));
    assert!(matches!(mint_op, BatchOperation::MintToken(_, _, _)));
    
    println!("✅ All BatchOperation variants created successfully");
}
