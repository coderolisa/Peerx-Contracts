use crate::order_types::*;
use std::collections::{HashMap, VecDeque};
use chrono::Utc;
use uuid::Uuid;
use crate::error::{MarketDataError, Result};

/// Matching algorithm variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchingAlgorithm {
    FIFO,
    ProRata,
    TimeWeighted,
    Auction,
}

/// Match result containing execution details
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub trade_id: String,
    pub taker_order_id: String,
    pub maker_order_id: String,
    pub price: f64,
    pub quantity: f64,
    pub algorithm_used: MatchingAlgorithm,
    pub execution_time_ns: u64,
}

/// Advanced matching engine with multiple algorithms
pub struct MatchingEngine {
    algorithm: MatchingAlgorithm,
    match_history: Vec<MatchResult>,
    performance_stats: PerformanceStats,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    pub total_matches: u64,
    pub total_volume: f64,
    pub avg_match_latency_ns: u64,
    pub min_match_latency_ns: u64,
    pub max_match_latency_ns: u64,
}

impl MatchingEngine {
    pub fn new(algorithm: MatchingAlgorithm) -> Self {
        Self {
            algorithm,
            match_history: Vec::new(),
            performance_stats: PerformanceStats::default(),
        }
    }

    /// Execute matching based on selected algorithm
    pub fn match_orders(
        &mut self,
        incoming_order: &Order,
        book_side: &[Order],
    ) -> Result<Vec<MatchResult>> {
        let start_time = std::time::Instant::now();

        let matches = match self.algorithm {
            MatchingAlgorithm::FIFO => self.fifo_match(incoming_order, book_side)?,
            MatchingAlgorithm::ProRata => self.pro_rata_match(incoming_order, book_side)?,
            MatchingAlgorithm::TimeWeighted => self.time_weighted_match(incoming_order, book_side)?,
            MatchingAlgorithm::Auction => self.auction_match(incoming_order, book_side)?,
        };

        let elapsed_ns = start_time.elapsed().as_nanos() as u64;
        self.update_stats(&matches, elapsed_ns);

        Ok(matches)
    }

    /// FIFO (First-In-First-Out) matching
    /// - Orders matched in chronological order (time priority)
    /// - Most common algorithm in modern exchanges
    fn fifo_match(&self, incoming_order: &Order, book_side: &[Order]) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        let mut remaining_quantity = incoming_order.remaining_quantity();
        let mut book_orders = book_side
            .iter()
            .filter(|o| o.remaining_quantity() > 0.0)
            .collect::<Vec<_>>();

        // Sort by creation time (FIFO)
        book_orders.sort_by_key(|o| o.created_at);

        for book_order in book_orders {
            if remaining_quantity <= 0.0 {
                break;
            }

            let match_qty = remaining_quantity.min(book_order.remaining_quantity());

            matches.push(MatchResult {
                trade_id: Uuid::new_v4().to_string(),
                taker_order_id: incoming_order.id.clone(),
                maker_order_id: book_order.id.clone(),
                price: book_order.price,
                quantity: match_qty,
                algorithm_used: MatchingAlgorithm::FIFO,
                execution_time_ns: 0,
            });

            remaining_quantity -= match_qty;
        }

        Ok(matches)
    }

    /// Pro-Rata matching
    /// - Orders matched proportionally to their quantity
    /// - Fair distribution when demand exceeds supply
    fn pro_rata_match(&self, incoming_order: &Order, book_side: &[Order]) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        let mut remaining_quantity = incoming_order.remaining_quantity();

        let eligible_orders: Vec<_> = book_side
            .iter()
            .filter(|o| o.remaining_quantity() > 0.0)
            .collect();

        if eligible_orders.is_empty() {
            return Ok(matches);
        }

        let total_available_qty: f64 = eligible_orders.iter().map(|o| o.remaining_quantity()).sum();

        for book_order in eligible_orders {
            if remaining_quantity <= 0.0 {
                break;
            }

            let proportion = book_order.remaining_quantity() / total_available_qty;
            let match_qty = (remaining_quantity * proportion).min(book_order.remaining_quantity());

            matches.push(MatchResult {
                trade_id: Uuid::new_v4().to_string(),
                taker_order_id: incoming_order.id.clone(),
                maker_order_id: book_order.id.clone(),
                price: book_order.price,
                quantity: match_qty,
                algorithm_used: MatchingAlgorithm::ProRata,
                execution_time_ns: 0,
            });

            remaining_quantity -= match_qty;
        }

        Ok(matches)
    }

    /// Time-Weighted matching
    /// - Longer-waiting orders get priority
    /// - Incentivizes liquidity provision
    fn time_weighted_match(
        &self,
        incoming_order: &Order,
        book_side: &[Order],
    ) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        let mut remaining_quantity = incoming_order.remaining_quantity();

        let mut book_orders: Vec<_> = book_side
            .iter()
            .filter(|o| o.remaining_quantity() > 0.0)
            .collect();

        // Sort by age (oldest first)
        book_orders.sort_by_key(|o| std::cmp::Reverse(o.created_at));

        for book_order in book_orders {
            if remaining_quantity <= 0.0 {
                break;
            }

            let match_qty = remaining_quantity.min(book_order.remaining_quantity());

            matches.push(MatchResult {
                trade_id: Uuid::new_v4().to_string(),
                taker_order_id: incoming_order.id.clone(),
                maker_order_id: book_order.id.clone(),
                price: book_order.price,
                quantity: match_qty,
                algorithm_used: MatchingAlgorithm::TimeWeighted,
                execution_time_ns: 0,
            });

            remaining_quantity -= match_qty;
        }

        Ok(matches)
    }

    /// Auction matching (call auction)
    /// - All orders cleared at single price
    /// - Used for opening/closing auctions
    fn auction_match(&self, incoming_order: &Order, book_side: &[Order]) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        let mut total_quantity = incoming_order.quantity;

        let eligible_orders: Vec<_> = book_side
            .iter()
            .filter(|o| o.remaining_quantity() > 0.0)
            .collect();

        // Calculate clearing price (volume-weighted midpoint)
        let clearing_price = if !eligible_orders.is_empty() {
            let weighted_price: f64 = eligible_orders
                .iter()
                .map(|o| o.price * o.remaining_quantity())
                .sum::<f64>()
                / eligible_orders.iter().map(|o| o.remaining_quantity()).sum::<f64>();
            weighted_price
        } else {
            incoming_order.price
        };

        // Match all orders at clearing price
        let mut remaining = total_quantity;
        for book_order in eligible_orders {
            if remaining <= 0.0 {
                break;
            }

            let match_qty = remaining.min(book_order.remaining_quantity());

            matches.push(MatchResult {
                trade_id: Uuid::new_v4().to_string(),
                taker_order_id: incoming_order.id.clone(),
                maker_order_id: book_order.id.clone(),
                price: clearing_price,
                quantity: match_qty,
                algorithm_used: MatchingAlgorithm::Auction,
                execution_time_ns: 0,
            });

            remaining -= match_qty;
        }

        Ok(matches)
    }

    fn update_stats(&mut self, matches: &[MatchResult], latency_ns: u64) {
        self.performance_stats.total_matches += matches.len() as u64;
        self.performance_stats.total_volume += matches.iter().map(|m| m.quantity).sum::<f64>();

        if !matches.is_empty() {
            self.performance_stats.avg_match_latency_ns =
                (self.performance_stats.avg_match_latency_ns * (self.performance_stats.total_matches - 1) as u64
                    + latency_ns)
                    / self.performance_stats.total_matches as u64;
            self.performance_stats.min_match_latency_ns =
                self.performance_stats.min_match_latency_ns.min(latency_ns);
            self.performance_stats.max_match_latency_ns =
                self.performance_stats.max_match_latency_ns.max(latency_ns);
        }

        for m in matches {
            self.match_history.push(m.clone());
        }
    }

    pub fn get_stats(&self) -> &PerformanceStats {
        &self.performance_stats
    }

    pub fn get_match_history(&self) -> &[MatchResult] {
        &self.match_history
    }

    pub fn switch_algorithm(&mut self, algorithm: MatchingAlgorithm) {
        self.algorithm = algorithm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_order(
        id: &str,
        side: OrderSide,
        price: f64,
        quantity: f64,
    ) -> Order {
        Order::new(
            id.to_string(),
            "BTC/USD".to_string(),
            side,
            OrderType::Limit,
            price,
            quantity,
        )
    }

    #[test]
    fn test_fifo_matching() {
        let mut engine = MatchingEngine::new(MatchingAlgorithm::FIFO);
        let incoming = create_test_order("buy1", OrderSide::Buy, 50100.0, 1.0);
        let book = vec![
            create_test_order("sell1", OrderSide::Sell, 50000.0, 0.5),
            create_test_order("sell2", OrderSide::Sell, 50000.0, 0.6),
        ];

        let matches = engine.match_orders(&incoming, &book).unwrap();
        assert_eq!(matches.len(), 2);
        assert!(matches[0].quantity > 0.0);
    }

    #[test]
    fn test_pro_rata_matching() {
        let mut engine = MatchingEngine::new(MatchingAlgorithm::ProRata);
        let incoming = create_test_order("buy1", OrderSide::Buy, 50100.0, 1.0);
        let book = vec![
            create_test_order("sell1", OrderSide::Sell, 50000.0, 0.5),
            create_test_order("sell2", OrderSide::Sell, 50000.0, 0.5),
        ];

        let matches = engine.match_orders(&incoming, &book).unwrap();
        assert_eq!(matches.len(), 2);
        // Each should get roughly 50% of order
        assert!((matches[0].quantity - 0.5).abs() < 0.01);
    }
}
