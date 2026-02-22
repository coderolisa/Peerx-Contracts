use soroban_sdk::{Address, Env, Symbol};

pub struct Events;

impl Events {
    pub fn swap_executed(
        env: &Env,
        from_token: Symbol,
        to_token: Symbol,
        from_amount: i128,
        to_amount: i128,
        user: Address,
        timestamp: i64,
    ) {
        env.events().publish(
            (Symbol::new(env, "SwapExecuted"), user, from_token, to_token),
            (from_amount, to_amount, timestamp),
        );
    }

    pub fn liquidity_added(
        env: &Env,
        xlm_amount: i128,
        usdc_amount: i128,
        lp_tokens_minted: i128,
        user: Address,
        timestamp: i64,
    ) {
        env.events().publish(
            (Symbol::new(env, "LiquidityAdded"), user),
            (xlm_amount, usdc_amount, lp_tokens_minted, timestamp),
        );
    }

    pub fn liquidity_removed(
        env: &Env,
        xlm_amount: i128,
        usdc_amount: i128,
        lp_tokens_burned: i128,
        user: Address,
        timestamp: i64,
    ) {
        env.events().publish(
            (Symbol::new(env, "LiquidityRemoved"), user),
            (xlm_amount, usdc_amount, lp_tokens_burned, timestamp),
        );
    }

    pub fn badge_awarded(env: &Env, user: Address, badge: crate::portfolio::Badge, timestamp: i64) {
        env.events()
            .publish((Symbol::new(env, "BadgeAwarded"), user), (badge, timestamp));
    }

    pub fn user_tier_changed(
        env: &Env,
        user: Address,
        old_tier: crate::tiers::UserTier,
        new_tier: crate::tiers::UserTier,
        timestamp: i64,
    ) {
        env.events().publish(
            (Symbol::new(env, "UserTierChanged"), user),
            (old_tier, new_tier, timestamp),
        );
    }

    pub fn admin_paused(env: &Env, admin: Address, timestamp: i64) {
        env.events()
            .publish((Symbol::new(env, "AdminPaused"), admin), (timestamp,));
    }

    pub fn admin_resumed(env: &Env, admin: Address, timestamp: i64) {
        env.events()
            .publish((Symbol::new(env, "AdminResumed"), admin), (timestamp,));
    }

impl Events {
    /// Emitted whenever an alert fires. Carries enough metadata for an
    /// off-chain indexer to route a push notification or webhook call.
    ///
    /// Topic  : ("AlertTriggered", owner_address, alert_id)
    /// Payload: (alert_kind, notification_method, timestamp)
    ///
    /// NOTE: This event is also emitted directly inside `alerts.rs` via
    /// `emit_alert_triggered`. This stub documents the schema for the audit
    /// trail and can be called from `events.rs` if you prefer to centralise
    /// event emission in future.
    pub fn alert_triggered(
        env: &Env,
        owner: Address,
        alert_id: u64,
        // Using Symbol here keeps the payload ABI-stable regardless of the
        // internal AlertKind enum layout across contract upgrades.
        kind_tag: Symbol,
        notification_method_tag: Symbol,
        timestamp: u64,
    ) {
        env.events().publish(
            (Symbol::new(env, "AlertTriggered"), owner, alert_id),
            (kind_tag, notification_method_tag, timestamp),
        );
    }

    /// Emitted when an alert is created so indexers can track the full
    /// lifecycle (create → trigger → cleanup) without polling storage.
    ///
    /// Topic  : ("AlertCreated", owner_address, alert_id)
    /// Payload: (kind_tag, expires_at)
    pub fn alert_created(
        env: &Env,
        owner: Address,
        alert_id: u64,
        kind_tag: Symbol,
        expires_at: u64,
    ) {
        env.events().publish(
            (Symbol::new(env, "AlertCreated"), owner, alert_id),
            (kind_tag, expires_at),
        );
    }
}
}
