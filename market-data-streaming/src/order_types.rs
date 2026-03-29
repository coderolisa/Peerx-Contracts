use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::fmt;

/// Advanced order types for professional trading
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    Iceberg(IcebergOrder),
    TWAP(TWAPOrder),
    VWAP(VWAPOrder),
    Bracket(BracketOrder),
    StopLoss(StopLossOrder),
    TrailingStop(TrailingStopOrder),
    PostOnly,
    FillOrKill,
    ImmediateOrCancel,
}

/// Iceberg order: Large order split into visible and hidden portions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IcebergOrder {
    pub visible_quantity: f64,
    pub total_quantity: f64,
    pub current_visible: f64,
    pub hidden_quantity: f64,
    pub created_at: DateTime<Utc>,
}

/// Time-Weighted Average Price order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TWAPOrder {
    pub total_quantity: f64,
    pub execution_time_seconds: u32,
    pub slice_interval_ms: u32,
    pub num_slices: u32,
    pub executed_quantity: f64,
    pub created_at: DateTime<Utc>,
}

/// Volume-Weighted Average Price order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VWAPOrder {
    pub total_quantity: f64,
    pub reference_price: f64,
    pub participation_rate: f64, // 0.0 to 1.0
    pub lookback_period_minutes: u32,
    pub executed_quantity: f64,
    pub created_at: DateTime<Utc>,
}

/// Bracket order: Entry with profit target and stop loss
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BracketOrder {
    pub entry_price: f64,
    pub profit_target_price: f64,
    pub stop_loss_price: f64,
    pub take_profit_quantity: f64,
    pub stop_loss_quantity: f64,
    pub created_at: DateTime<Utc>,
}

/// Stop loss order with price trigger
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StopLossOrder {
    pub trigger_price: f64,
    pub trigger_type: TriggerType,
    pub order_type: Box<OrderType>,
    pub triggered: bool,
    pub trigger_timestamp: Option<DateTime<Utc>>,
}

/// Trailing stop order that follows price movements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrailingStopOrder {
    pub trail_amount: f64,
    pub trail_type: TrailType,
    pub highest_price: f64,
    pub stop_price: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TriggerType {
    LastPrice,
    BidPrice,
    AskPrice,
    MarkPrice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TrailType {
    Absolute,
    Percentage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
    PendingTriggering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub time_in_force: TimeInForce,
    pub client_id: Option<String>,
    pub metadata: Option<OrderMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum TimeInForce {
    GoodTilCancelled,
    GoodForDay,
    ImmediateOrCancel,
    FillOrKill,
    GoodTilTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderMetadata {
    pub priority: OrderPriority,
    pub hft_protection_enabled: bool,
    pub execution_algorithm: Option<String>,
    pub trader_id: Option<String>,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub taker_order_id: String,
    pub maker_order_id: String,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub side: OrderSide,
    pub timestamp: DateTime<Utc>,
    pub taker_fee: f64,
    pub maker_fee: f64,
    pub execution_type: ExecutionType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ExecutionType {
    Regular,
    IcebergSlice,
    TWAPSlice,
    VWAPSlice,
    BlockTrade,
    RFQ,
}

impl Order {
    pub fn new(
        id: String,
        symbol: String,
        side: OrderSide,
        order_type: OrderType,
        price: f64,
        quantity: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            symbol,
            side,
            order_type,
            price,
            quantity,
            filled_quantity: 0.0,
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            time_in_force: TimeInForce::GoodTilCancelled,
            client_id: None,
            metadata: None,
        }
    }

    pub fn remaining_quantity(&self) -> f64 {
        (self.quantity - self.filled_quantity).max(0.0)
    }

    pub fn is_fully_filled(&self) -> bool {
        (self.quantity - self.filled_quantity).abs() < 1e-8
    }

    pub fn update_filled(&mut self, quantity: f64) {
        self.filled_quantity += quantity;
        self.updated_at = Utc::now();

        if self.is_fully_filled() {
            self.status = OrderStatus::Filled;
        } else if self.filled_quantity > 0.0 {
            self.status = OrderStatus::PartiallyFilled;
        }
    }

    pub fn cancel(&mut self) {
        if !self.is_fully_filled() {
            self.status = OrderStatus::Cancelled;
            self.updated_at = Utc::now();
        }
    }

    pub fn get_average_price(&self) -> Option<f64> {
        if self.filled_quantity > 0.0 {
            Some((self.price * self.filled_quantity) / self.filled_quantity)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let order = Order::new(
            "ord1".to_string(),
            "BTC/USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            50000.0,
            1.0,
        );
        assert_eq!(order.status, OrderStatus::New);
        assert_eq!(order.remaining_quantity(), 1.0);
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::new(
            "ord1".to_string(),
            "BTC/USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            50000.0,
            1.0,
        );

        order.update_filled(0.5);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.remaining_quantity(), 0.5);

        order.update_filled(0.5);
        assert_eq!(order.status, OrderStatus::Filled);
        assert!(order.is_fully_filled());
    }

    #[test]
    fn test_order_cancel() {
        let mut order = Order::new(
            "ord1".to_string(),
            "BTC/USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            50000.0,
            1.0,
        );

        order.cancel();
        assert_eq!(order.status, OrderStatus::Cancelled);
    }
}
