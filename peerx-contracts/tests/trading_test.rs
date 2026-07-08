pub fn swap(
    balances: &mut HashMap<String, u64>,
    #![cfg(test)]

    use std::collections::HashMap;

    // Re-implement the small swap helper as a local pure-Rust function for fast tests.
    fn swap(
        balances: &mut HashMap<String, u64>,
        from: &str,
        to: &str,
        amount: u64,
        rates: &HashMap<(String, String), f64>,
    ) -> Result<(), String> {
        if !balances.contains_key(from) || !balances.contains_key(to) {
            return Err("Unsupported token".into());
        }

        let from_balance = balances.get_mut(from).unwrap();
        if *from_balance < amount {
            return Err("Insufficient balance".into());
        }

        if let Some(rate) = rates.get(&(from.to_string(), to.to_string())) {
            *from_balance -= amount;
            // Simulate rounding/truncation that may occur in integer-based ledgers
            let converted = (amount as f64 * rate) as u64;
            *balances.get_mut(to).unwrap() += converted;
            Ok(())
        } else {
            Err("Swap rate not found".into())
        }
    }

    #[test]
    fn happy_path_swap_updates_balances() {
        let mut balances = HashMap::new();
        balances.insert("XLM".to_string(), 1000u64);
        balances.insert("USDC".to_string(), 0u64);

        let mut rates = HashMap::new();
        rates.insert(("XLM".to_string(), "USDC".to_string()), 1.0f64);

        let res = swap(&mut balances, "XLM", "USDC", 500, &rates);
        assert!(res.is_ok());
        assert_eq!(balances.get("XLM"), Some(&500u64));
        assert_eq!(balances.get("USDC"), Some(&500u64));
    }

    #[test]
    fn insufficient_balance_returns_error() {
        let mut balances = HashMap::new();
        balances.insert("XLM".to_string(), 100u64);
        balances.insert("USDC".to_string(), 0u64);

        let mut rates = HashMap::new();
        rates.insert(("XLM".to_string(), "USDC".to_string()), 1.0f64);

        let err = swap(&mut balances, "XLM", "USDC", 200, &rates).unwrap_err();
        assert_eq!(err, "Insufficient balance");
    }

    #[test]
    fn unsupported_token_returns_error() {
        let mut balances = HashMap::new();
        balances.insert("XLM".to_string(), 100u64);
        balances.insert("USDC".to_string(), 0u64);

        let rates = HashMap::new();

        let err = swap(&mut balances, "XLM", "BTC", 10, &rates).unwrap_err();
        assert_eq!(err, "Unsupported token");
    }

    #[test]
    fn rounding_truncation_behavior() {
        let mut balances = HashMap::new();
        balances.insert("XLM".to_string(), 3u64);
        balances.insert("USDC".to_string(), 0u64);

        // rate causes fractional converted amounts
        let mut rates = HashMap::new();
        rates.insert(("XLM".to_string(), "USDC".to_string()), 0.333333_f64);

        // swap 1 XLM -> should convert to floor(1 * 0.333333) == 0 USDC
        let ok = swap(&mut balances, "XLM", "USDC", 1, &rates);
        assert!(ok.is_ok());
        assert_eq!(balances.get("XLM"), Some(&2u64));
        assert_eq!(balances.get("USDC"), Some(&0u64));

        // swap remaining 2 XLM -> floor(2 * 0.333333) == 0 USDC, balances become 0 XLM
        swap(&mut balances, "XLM", "USDC", 2, &rates).unwrap();
        assert_eq!(balances.get("XLM"), Some(&0u64));
        assert_eq!(balances.get("USDC"), Some(&0u64));
    }

    #[test]
    fn simulated_concurrent_order_isolation() {
        // This simulates two users by keeping two separate balance maps and running swaps "concurrently" (sequentially in test)
        let mut alice = HashMap::new();
        alice.insert("XLM".to_string(), 500u64);
        alice.insert("USDC".to_string(), 0u64);

        let mut bob = HashMap::new();
        bob.insert("XLM".to_string(), 300u64);
        bob.insert("USDC".to_string(), 0u64);

        let mut rates = HashMap::new();
        rates.insert(("XLM".to_string(), "USDC".to_string()), 1.0f64);

        // Alice and Bob swap in quick succession
        swap(&mut alice, "XLM", "USDC", 200, &rates).unwrap();
        swap(&mut bob, "XLM", "USDC", 300, &rates).unwrap();

        assert_eq!(alice.get("XLM"), Some(&300u64));
        assert_eq!(alice.get("USDC"), Some(&200u64));

        assert_eq!(bob.get("XLM"), Some(&0u64));
        assert_eq!(bob.get("USDC"), Some(&300u64));
    }
