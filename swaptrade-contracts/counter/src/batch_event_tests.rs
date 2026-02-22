#[cfg(test)]
mod batch_event_tests {
    use crate::portfolio::{Portfolio, Asset, Badge};
    use crate::events::Events;
    use soroban_sdk::{Env, testutils::{Address as _, Events as _}, Address, Symbol};

    #[test]
    fn test_multiple_badges_batched() {
        let env = Env::default();
        let mut portfolio = Portfolio::new(&env);
        let user = Address::generate(&env);
        
        portfolio.mint(&env, Asset::XLM, user.clone(), 1000);
        portfolio.record_initial_balance(user.clone(), 100);
        
        // Award multiple badges
        for _ in 0..10 {
            portfolio.record_trade(&env, user.clone());
        }
        portfolio.check_and_award_badges(&env, user.clone());
        
        // Flush events
        Events::flush_badge_events(&env);
        
        // Verify badges were awarded
        assert!(portfolio.has_badge(&env, user.clone(), Badge::FirstTrade));
        assert!(portfolio.has_badge(&env, user.clone(), Badge::Trader));
        
        // Verify event was emitted
        let events = env.events().all();
        let badge_events: Vec<_> = events.iter()
            .filter(|e| {
                if let Ok((topics, _)) = e {
                    topics.len() > 0 && topics.get(0).unwrap() == Symbol::new(&env, "BadgesAwarded")
                } else {
                    false
                }
            })
            .collect();
        
        // Should have exactly 1 batched event
        assert_eq!(badge_events.len(), 1);
    }

    #[test]
    fn test_single_badge_batched() {
        let env = Env::default();
        let mut portfolio = Portfolio::new(&env);
        let user = Address::generate(&env);
        
        portfolio.record_trade(&env, user.clone());
        Events::flush_badge_events(&env);
        
        assert!(portfolio.has_badge(&env, user.clone(), Badge::FirstTrade));
    }

    #[test]
    fn test_empty_buffer_no_event() {
        let env = Env::default();
        
        Events::flush_badge_events(&env);
        
        let events = env.events().all();
        let badge_events: Vec<_> = events.iter()
            .filter(|e| {
                if let Ok((topics, _)) = e {
                    topics.len() > 0 && topics.get(0).unwrap() == Symbol::new(&env, "BadgesAwarded")
                } else {
                    false
                }
            })
            .collect();
        
        assert_eq!(badge_events.len(), 0);
    }

    #[test]
    fn test_badges_stored_correctly_with_batching() {
        let env = Env::default();
        let mut portfolio = Portfolio::new(&env);
        let user = Address::generate(&env);
        
        portfolio.record_trade(&env, user.clone());
        assert!(portfolio.has_badge(&env, user.clone(), Badge::FirstTrade));
        
        Events::flush_badge_events(&env);
        
        // Badge should still be stored after flush
        assert!(portfolio.has_badge(&env, user.clone(), Badge::FirstTrade));
    }

    #[test]
    fn test_lp_badge_batched() {
        let env = Env::default();
        let mut portfolio = Portfolio::new(&env);
        let user = Address::generate(&env);
        
        portfolio.record_lp_deposit(user.clone());
        portfolio.check_and_award_badges(&env, user.clone());
        
        Events::flush_badge_events(&env);
        
        assert!(portfolio.has_badge(&env, user.clone(), Badge::LiquidityProvider));
    }
}
