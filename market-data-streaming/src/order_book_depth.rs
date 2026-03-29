use crate::order_types::*;
use std::collections::{BTreeMap, HashMap};
use chrono::{DateTime, Utc};
use crate::error::{MarketDataError, Result};

/// Price level aggregation for depth optimization
#[derive(Debug, Clone)]
pub struct AggregatedLevel {
    pub price: f64,
    pub total_quantity: f64,
    pub order_count: usize,
    pub weighted_price: f64,
    pub orders: Vec<String>, // Order IDs at this level
}

/// Order book depth with optimization
pub struct OrderBookDepth {
    pub symbol: String,
    pub bids: BTreeMap<OrderedFloat, AggregatedLevel>,
    pub asks: BTreeMap<OrderedFloat, AggregatedLevel>,
    pub max_depth_levels: usize,
    pub price_precision: u8,
    pub updated_at: DateTime<Utc>,
    pub sequence: u64,
}

/// Wrapper for ordered float keys
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl OrderBookDepth {
    pub fn new(symbol: String, max_depth_levels: usize, price_precision: u8) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            max_depth_levels,
            price_precision,
            updated_at: Utc::now(),
            sequence: 0,
        }
    }

    /// Add order to book and maintain depth limits
    pub fn add_order(&mut self, order: &Order) -> Result<()> {
        self.sequence += 1;
        self.updated_at = Utc::now();

        let rounded_price = self.round_price(order.price);
        let key = OrderedFloat(rounded_price);

        match order.side {
            OrderSide::Buy => {
                let level = self.bids.entry(key).or_insert_with(|| AggregatedLevel {
                    price: rounded_price,
                    total_quantity: 0.0,
                    order_count: 0,
                    weighted_price: rounded_price,
                    orders: Vec::new(),
                });
                level.total_quantity += order.remaining_quantity();
                level.order_count += 1;
                level.orders.push(order.id.clone());
                level.weighted_price = (level.weighted_price + rounded_price) / 2.0;
            }
            OrderSide::Sell => {
                let level = self.asks.entry(key).or_insert_with(|| AggregatedLevel {
                    price: rounded_price,
                    total_quantity: 0.0,
                    order_count: 0,
                    weighted_price: rounded_price,
                    orders: Vec::new(),
                });
                level.total_quantity += order.remaining_quantity();
                level.order_count += 1;
                level.orders.push(order.id.clone());
                level.weighted_price = (level.weighted_price + rounded_price) / 2.0;
            }
        }

        self.prune_depth();
        Ok(())
    }

    /// Remove order from book
    pub fn remove_order(&mut self, side: OrderSide, price: f64, order_id: &str) -> Result<()> {
        self.sequence += 1;
        self.updated_at = Utc::now();

        let rounded_price = self.round_price(price);
        let key = OrderedFloat(rounded_price);

        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        if let Some(level) = book.get_mut(&key) {
            if let Some(pos) = level.orders.iter().position(|id| id == order_id) {
                level.orders.remove(pos);
                level.order_count = level.order_count.saturating_sub(1);
            }

            if level.orders.is_empty() {
                book.remove(&key);
            }
        }

        Ok(())
    }

    /// Update filled quantity at price level
    pub fn update_level(&mut self, side: OrderSide, price: f64, filled_qty: f64) -> Result<()> {
        let rounded_price = self.round_price(price);
        let key = OrderedFloat(rounded_price);

        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        if let Some(level) = book.get_mut(&key) {
            level.total_quantity = (level.total_quantity - filled_qty).max(0.0);
            if level.total_quantity <= 0.0 {
                book.remove(&key);
            }
        }

        Ok(())
    }

    /// Get top N price levels
    pub fn get_top_levels(&self, side: OrderSide, count: usize) -> Vec<AggregatedLevel> {
        let book = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };

        if side == OrderSide::Buy {
            book.iter()
                .rev()
                .take(count)
                .map(|(_, level)| level.clone())
                .collect()
        } else {
            book.iter()
                .take(count)
                .map(|(_, level)| level.clone())
                .collect()
        }
    }

    /// Get full depth snapshot
    pub fn get_depth_snapshot(&self, depth_levels: usize) -> DepthSnapshot {
        DepthSnapshot {
            symbol: self.symbol.clone(),
            timestamp: Utc::now(),
            bids: self.get_top_levels(OrderSide::Buy, depth_levels),
            asks: self.get_top_levels(OrderSide::Sell, depth_levels),
            sequence: self.sequence,
            bid_ask_spread: self.get_spread(),
        }
    }

    /// Calculate bid-ask spread
    pub fn get_spread(&self) -> Option<f64> {
        let best_bid = self.bids.iter().next_back().map(|(k, _)| k.0);
        let best_ask = self.asks.iter().next().map(|(k, _)| k.0);

        match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get mid price
    pub fn get_mid_price(&self) -> Option<f64> {
        if let (Some((bid_key, _)), Some((ask_key, _))) =
            (self.bids.iter().next_back(), self.asks.iter().next())
        {
            Some((bid_key.0 + ask_key.0) / 2.0)
        } else {
            None
        }
    }

    /// Get total quantity at side
    pub fn get_side_volume(&self, side: OrderSide) -> f64 {
        match side {
            OrderSide::Buy => self.bids.values().map(|l| l.total_quantity).sum(),
            OrderSide::Sell => self.asks.values().map(|l| l.total_quantity).sum(),
        }
    }

    /// Calculate cumulative volume up to given quantity
    pub fn get_cumulative_volume(&self, side: OrderSide, price: f64) -> f64 {
        let book = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };

        let mut cumulative = 0.0;
        for (key, level) in book.iter() {
            if side == OrderSide::Buy && key.0 <= price {
                cumulative += level.total_quantity;
            } else if side == OrderSide::Sell && key.0 >= price {
                cumulative += level.total_quantity;
            }
        }

        cumulative
    }

    /// Imbalance ratio (buy pressure / total)
    pub fn get_imbalance_ratio(&self) -> Option<f64> {
        let bid_volume = self.get_side_volume(OrderSide::Buy);
        let ask_volume = self.get_side_volume(OrderSide::Sell);
        let total = bid_volume + ask_volume;

        if total > 0.0 {
            Some(bid_volume / total)
        } else {
            None
        }
    }

    // Private helper methods

    fn round_price(&self, price: f64) -> f64 {
        let factor = 10_f64.powi(self.price_precision as i32);
        (price * factor).round() / factor
    }

    fn prune_depth(&mut self) {
        // Remove excess levels from both sides, keeping best prices
        while self.bids.len() > self.max_depth_levels {
            if let Some((key, _)) = self.bids.iter().next() {
                let key = *key;
                self.bids.remove(&key);
            }
        }

        while self.asks.len() > self.max_depth_levels {
            if let Some((key, _)) = self.asks.iter().last() {
                let key = *key;
                self.asks.remove(&key);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepthSnapshot {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bids: Vec<AggregatedLevel>,
    pub asks: Vec<AggregatedLevel>,
    pub sequence: u64,
    pub bid_ask_spread: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_order() {
        let mut book = OrderBookDepth::new("BTC/USD".to_string(), 100, 2);
        let order = Order::new(
            "ord1".to_string(),
            "BTC/USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            50000.123,
            1.5,
        );

        book.add_order(&order).unwrap();
        assert_eq!(book.get_side_volume(OrderSide::Buy), 1.5);
    }

    #[test]
    fn test_spread_calculation() {
        let mut book = OrderBookDepth::new("BTC/USD".to_string(), 100, 2);
        
        let buy_order = Order::new(
            "buy1".to_string(),
            "BTC/USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            50000.0,
            1.0,
        );
        
        let sell_order = Order::new(
            "sell1".to_string(),
            "BTC/USD".to_string(),
            OrderSide::Sell,
            OrderType::Limit,
            50010.0,
            1.0,
        );

        book.add_order(&buy_order).unwrap();
        book.add_order(&sell_order).unwrap();

        assert_eq!(book.get_spread(), Some(10.0));
    }

    #[test]
    fn test_depth_pruning() {
        let mut book = OrderBookDepth::new("BTC/USD".to_string(), 5, 0);
        
        for i in 1..=10 {
            let order = Order::new(
                format!("order{}", i),
                "BTC/USD".to_string(),
                OrderSide::Buy,
                OrderType::Limit,
                50000.0 - i as f64,
                1.0,
            );
            book.add_order(&order).unwrap();
        }

        assert!(book.bids.len() <= 5);
    }
}
