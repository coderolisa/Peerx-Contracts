use crate::order_types::*;
use crate::order_book_depth::{OrderBookDepth, AggregatedLevel};
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Duration, Utc};
use crate::error::Result;

/// Order book level metrics
#[derive(Debug, Clone)]
pub struct LevelMetrics {
    pub price: f64,
    pub quantity: f64,
    pub side: OrderSide,
    pub cumulative_volume: f64,
    pub price_levels_to_mid: u32,
    pub order_count: u32,
    pub notional_value: f64,
}

/// Order book analytics engine
pub struct OrderBookAnalytics {
    pub symbol: String,
    pub metrics_history: VecDeque<AnalyticsSnapshot>,
    pub max_history_size: usize,
    pub trades_history: VecDeque<Trade>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub mid_price: Option<f64>,
    pub spread: Option<f64>,
    pub spread_bps: Option<f64>,
    pub imbalance_ratio: Option<f64>,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub total_volume: f64,
    pub trade_intensity: f64,
    pub price_volatility: f64,
    pub order_imbalance: f64,
    pub depth_metrics: DepthMetrics,
    pub microstructure_indicators: MicrostructureIndicators,
}

#[derive(Debug, Clone, Default)]
pub struct DepthMetrics {
    pub bid_depth_10: f64,    // Volume at 10 bps from mid
    pub ask_depth_10: f64,
    pub bid_depth_50: f64,    // Volume at 50 bps from mid
    pub ask_depth_50: f64,
    pub bid_vwap_5: f64,      // Volume-weighted average price for 5 levels
    pub ask_vwap_5: f64,
    pub orderbook_skew: f64,
}

#[derive(Debug, Clone, Default)]
pub struct MicrostructureIndicators {
    pub order_flow_imbalance: f64,
    pub vpin: f64,              // Volume-Synchronized Probability of Informed Trading
    pub spread_decomposition: SpreadDecomposition,
    pub tick_direction: i32,    // -1 for down, 0 for neutral, 1 for up
    pub order_intensity: f64,   // Orders per second
    pub cancellation_rate: f64,
    pub adverse_selection_spread: f64,
    pub inventory_spread: f64,
}

#[derive(Debug, Clone, Default)]
pub struct SpreadDecomposition {
    pub adverse_selection: f64,
    pub inventory_cost: f64,
    pub order_processing: f64,
    pub profit_margin: f64,
}

/// Real-time market metrics
#[derive(Debug, Clone)]
pub struct MarketMetrics {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub vwap: f64,
    pub twap: f64,
    pub price_momentum: f64,
    pub price_acceleration: f64,
    pub volatility_5min: f64,
    pub volatility_hourly: f64,
    pub kurt_excess: f64,
    pub skewness: f64,
    pub hurst_exponent: f64,
}

impl OrderBookAnalytics {
    pub fn new(symbol: String, max_history_size: usize) -> Self {
        Self {
            symbol,
            metrics_history: VecDeque::with_capacity(max_history_size),
            max_history_size,
            trades_history: VecDeque::with_capacity(max_history_size * 10),
        }
    }

    /// Compute comprehensive analytics snapshot
    pub fn compute_snapshot(&mut self, book_depth: &OrderBookDepth, recent_trades: &[Trade]) -> Result<AnalyticsSnapshot> {
        let now = Utc::now();
        let bid_volume = book_depth.get_side_volume(OrderSide::Buy);
        let ask_volume = book_depth.get_side_volume(OrderSide::Sell);
        let total_volume = bid_volume + ask_volume;

        let mid_price = book_depth.get_mid_price();
        let spread = book_depth.get_spread();
        let spread_bps = spread.and_then(|s| mid_price.map(|mp| (s / mp) * 10000.0));

        let imbalance_ratio = book_depth.get_imbalance_ratio();
        let trade_intensity = self.calculate_trade_intensity(recent_trades);
        let price_volatility = self.calculate_volatility(recent_trades);
        let order_imbalance = self.calculate_order_imbalance(book_depth);

        let depth_metrics = self.calculate_depth_metrics(book_depth, mid_price);
        let microstructure = self.calculate_microstructure_indicators(book_depth, recent_trades);

        let snapshot = AnalyticsSnapshot {
            timestamp: now,
            sequence: book_depth.sequence,
            mid_price,
            spread,
            spread_bps,
            imbalance_ratio,
            bid_volume,
            ask_volume,
            total_volume,
            trade_intensity,
            price_volatility,
            order_imbalance,
            depth_metrics,
            microstructure_indicators: microstructure,
        };

        self.metrics_history.push_back(snapshot.clone());
        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.pop_front();
        }

        Ok(snapshot)
    }

    /// Add trade to history
    pub fn record_trade(&mut self, trade: Trade) {
        self.trades_history.push_back(trade);
        if self.trades_history.len() > self.max_history_size * 10 {
            self.trades_history.pop_front();
        }
    }

    /// Calculate order flow imbalance
    pub fn calculate_order_imbalance(&self, book_depth: &OrderBookDepth) -> f64 {
        let bid_volume = book_depth.get_side_volume(OrderSide::Buy);
        let ask_volume = book_depth.get_side_volume(OrderSide::Sell);

        if bid_volume + ask_volume > 0.0 {
            (bid_volume - ask_volume) / (bid_volume + ask_volume)
        } else {
            0.0
        }
    }

    /// Calculate trade intensity per second
    fn calculate_trade_intensity(&self, recent_trades: &[Trade]) -> f64 {
        if recent_trades.is_empty() {
            return 0.0;
        }

        let now = Utc::now();
        let last_minute = now - Duration::seconds(60);

        let recent_count = recent_trades
            .iter()
            .filter(|t| t.timestamp > last_minute)
            .count();

        recent_count as f64 / 60.0
    }

    /// Calculate price volatility from recent trades
    fn calculate_volatility(&self, recent_trades: &[Trade]) -> f64 {
        if recent_trades.len() < 2 {
            return 0.0;
        }

        let prices: Vec<f64> = recent_trades.iter().map(|t| t.price).collect();
        let mean = prices.iter().sum::<f64>() / prices.len() as f64;
        let variance = prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;
        variance.sqrt() / mean
    }

    /// Calculate depth metrics
    fn calculate_depth_metrics(&self, book_depth: &OrderBookDepth, mid_price: Option<f64>) -> DepthMetrics {
        let mut metrics = DepthMetrics::default();

        if let Some(mid) = mid_price {
            // Calculate volumes within price bands
            let bid_10bps = book_depth.get_cumulative_volume(OrderSide::Buy, mid * 0.999);
            let ask_10bps = book_depth.get_cumulative_volume(OrderSide::Sell, mid * 1.001);
            
            metrics.bid_depth_10 = bid_10bps;
            metrics.ask_depth_10 = ask_10bps;

            let bid_50bps = book_depth.get_cumulative_volume(OrderSide::Buy, mid * 0.995);
            let ask_50bps = book_depth.get_cumulative_volume(OrderSide::Sell, mid * 1.005);
            
            metrics.bid_depth_50 = bid_50bps;
            metrics.ask_depth_50 = ask_50bps;

            // Orderbook skew
            let total_depth = bid_10bps + ask_10bps;
            if total_depth > 0.0 {
                metrics.orderbook_skew = (bid_10bps - ask_10bps) / total_depth;
            }
        }

        metrics
    }

    /// Calculate advanced microstructure indicators
    fn calculate_microstructure_indicators(
        &self,
        book_depth: &OrderBookDepth,
        recent_trades: &[Trade],
    ) -> MicrostructureIndicators {
        let mut indicators = MicrostructureIndicators::default();

        // Order flow imbalance
        indicators.order_flow_imbalance = self.calculate_order_imbalance(book_depth);

        // Order intensity (orders per second)
        let now = Utc::now();
        let last_minute = now - Duration::seconds(60);
        let recent_trades_count = recent_trades.iter().filter(|t| t.timestamp > last_minute).count();
        indicators.order_intensity = recent_trades_count as f64 / 60.0;

        // Tick direction
        if !recent_trades.is_empty() {
            if recent_trades[0].price > recent_trades[recent_trades.len() - 1].price {
                indicators.tick_direction = -1;
            } else if recent_trades[0].price < recent_trades[recent_trades.len() - 1].price {
                indicators.tick_direction = 1;
            }
        }

        // Spread decomposition (simplified)
        if let Some(spread) = book_depth.get_spread() {
            indicators.spread_decomposition.adverse_selection = spread * 0.4;
            indicators.spread_decomposition.inventory_cost = spread * 0.3;
            indicators.spread_decomposition.order_processing = spread * 0.2;
            indicators.spread_decomposition.profit_margin = spread * 0.1;
        }

        indicators
    }

    /// Calculate Volume-Weighted Average Price
    pub fn calculate_vwap(&self, trades: &[Trade]) -> Option<f64> {
        if trades.is_empty() {
            return None;
        }

        let total_volume: f64 = trades.iter().map(|t| t.quantity).sum();
        if total_volume == 0.0 {
            return None;
        }

        let sum: f64 = trades.iter().map(|t| t.price * t.quantity).sum();
        Some(sum / total_volume)
    }

    /// Calculate Time-Weighted Average Price
    pub fn calculate_twap(&self, trades: &[Trade]) -> Option<f64> {
        if trades.is_empty() {
            return None;
        }

        let avg: f64 = trades.iter().map(|t| t.price).sum::<f64>() / trades.len() as f64;
        Some(avg)
    }

    /// Get visualization data for display
    pub fn get_visualization_data(&self) -> OrderBookVisualization {
        let recent_snapshot = self.metrics_history.back().cloned();

        OrderBookVisualization {
            snapshots: self.metrics_history.iter().cloned().collect(),
            recent_snapshot,
            trades: self.trades_history.iter().cloned().collect(),
        }
    }

    /// Detect market anomalies
    pub fn detect_anomalies(&self) -> Vec<MarketAnomaly> {
        let mut anomalies = Vec::new();

        if let Some(current) = self.metrics_history.back() {
            // Check for unusual spread
            if let Some(spread_bps) = current.spread_bps {
                if spread_bps > 100.0 {
                    anomalies.push(MarketAnomaly {
                        anomaly_type: AnomalyType::WideSpread,
                        severity: 0.5 + (spread_bps / 1000.0).min(0.5),
                        timestamp: current.timestamp,
                    });
                }
            }

            // Check for extreme imbalance
            if let Some(imbalance) = current.imbalance_ratio {
                if imbalance.abs() > 0.7 {
                    anomalies.push(MarketAnomaly {
                        anomaly_type: AnomalyType::OrderImbalance,
                        severity: imbalance.abs() - 0.7,
                        timestamp: current.timestamp,
                    });
                }
            }

            // Check for excessive volatility
            if current.price_volatility > 0.05 {
                anomalies.push(MarketAnomaly {
                    anomaly_type: AnomalyType::HighVolatility,
                    severity: (current.price_volatility / 0.1).min(1.0),
                    timestamp: current.timestamp,
                });
            }
        }

        anomalies
    }

    /// Get historical metrics for a period
    pub fn get_historical_metrics(&self, minutes: usize) -> Vec<AnalyticsSnapshot> {
        let cutoff = Utc::now() - Duration::minutes(minutes as i64);
        
        self.metrics_history
            .iter()
            .filter(|s| s.timestamp > cutoff)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookVisualization {
    pub snapshots: Vec<AnalyticsSnapshot>,
    pub recent_snapshot: Option<AnalyticsSnapshot>,
    pub trades: Vec<Trade>,
}

#[derive(Debug, Clone)]
pub struct MarketAnomaly {
    pub anomaly_type: AnomalyType,
    pub severity: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnomalyType {
    WideSpread,
    OrderImbalance,
    HighVolatility,
    UnusualTradeSize,
    LargeOrderBook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_creation() {
        let mut analytics = OrderBookAnalytics::new("BTC/USD".to_string(), 1000);
        assert_eq!(analytics.symbol, "BTC/USD");
        assert_eq!(analytics.metrics_history.len(), 0);
    }

    #[test]
    fn test_trade_intensity() {
        let analytics = OrderBookAnalytics::new("BTC/USD".to_string(), 100);
        let now = Utc::now();
        let trades = vec![
            Trade {
                id: "1".to_string(),
                taker_order_id: "t1".to_string(),
                maker_order_id: "m1".to_string(),
                symbol: "BTC/USD".to_string(),
                price: 50000.0,
                quantity: 1.0,
                side: OrderSide::Buy,
                timestamp: now,
                taker_fee: 10.0,
                maker_fee: -5.0,
                execution_type: ExecutionType::Regular,
            },
        ];

        let intensity = analytics.calculate_trade_intensity(&trades);
        assert!(intensity > 0.0);
    }

    #[test]
    fn test_vwap_calculation() {
        let analytics = OrderBookAnalytics::new("BTC/USD".to_string(), 100);
        let now = Utc::now();
        let trades = vec![
            Trade {
                id: "1".to_string(),
                taker_order_id: "t1".to_string(),
                maker_order_id: "m1".to_string(),
                symbol: "BTC/USD".to_string(),
                price: 100.0,
                quantity: 1.0,
                side: OrderSide::Buy,
                timestamp: now,
                taker_fee: 0.0,
                maker_fee: 0.0,
                execution_type: ExecutionType::Regular,
            },
            Trade {
                id: "2".to_string(),
                taker_order_id: "t2".to_string(),
                maker_order_id: "m2".to_string(),
                symbol: "BTC/USD".to_string(),
                price: 200.0,
                quantity: 3.0,
                side: OrderSide::Buy,
                timestamp: now,
                taker_fee: 0.0,
                maker_fee: 0.0,
                execution_type: ExecutionType::Regular,
            },
        ];

        let vwap = analytics.calculate_vwap(&trades).unwrap();
        assert!((vwap - 175.0).abs() < 0.01); // (100*1 + 200*3) / 4 = 175
    }
}
