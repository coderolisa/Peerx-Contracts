// src/audit_log.rs
// Comprehensive audit trail with cryptographic chain-of-custody

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

// ─── Event Taxonomy ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventCategory {
    Administrative,
    Trading,
    Security,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Critical,
    Emergency,
}

// ─── Core Event Schema ────────────────────────────────────────────────────────

/// The canonical on-chain event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Monotonically-increasing, globally unique identifier
    pub id: u64,
    /// Unix timestamp (nanoseconds)
    pub timestamp: u128,
    /// Address / identity that triggered the event
    pub actor: String,
    /// Machine-readable action verb (e.g. "TRADE_EXECUTE", "ADMIN_ROLE_GRANT")
    pub action: String,
    /// Resource acted upon
    pub target: String,
    /// "OK" or error description
    pub result: String,
    /// Gas consumed executing the transaction (0 for off-chain events)
    pub gas_used: u64,
    /// Keccak/SHA-256 of contract state *after* this event
    pub state_hash: [u8; 32],
    pub category: EventCategory,
    pub severity: Severity,
    /// SHA-256 hash of the *previous* event (all-zeros for genesis)
    pub prev_hash: [u8; 32],
    /// SHA-256 fingerprint of this event (computed after all other fields are set)
    pub event_hash: [u8; 32],
}

impl AuditEvent {
    /// Compute the canonical hash for this event (excluding the `event_hash` field itself).
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.id.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.actor.as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.target.as_bytes());
        hasher.update(self.result.as_bytes());
        hasher.update(self.gas_used.to_le_bytes());
        hasher.update(self.state_hash);
        hasher.update(self.prev_hash);
        hasher.finalize().into()
    }

    pub fn is_self_consistent(&self) -> bool {
        self.event_hash == self.compute_hash()
    }
}

// ─── Merkle Tree (for range-query proofs) ─────────────────────────────────────

pub struct MerkleTree {
    /// Leaf layer: each leaf is an event_hash
    leaves: Vec<[u8; 32]>,
    /// Remaining levels up to the root
    levels: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn build(hashes: &[[u8; 32]]) -> Self {
        if hashes.is_empty() {
            return Self { leaves: vec![], levels: vec![] };
        }
        let leaves = hashes.to_vec();
        let mut levels: Vec<Vec<[u8; 32]>> = vec![leaves.clone()];
        let mut current = leaves.clone();
        while current.len() > 1 {
            let mut next = Vec::new();
            for chunk in current.chunks(2) {
                let mut h = Sha256::new();
                h.update(chunk[0]);
                h.update(chunk.get(1).unwrap_or(&chunk[0])); // duplicate last if odd
                next.push(h.finalize().into());
            }
            levels.push(next.clone());
            current = next;
        }
        Self { leaves, levels }
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.levels.last().and_then(|l| l.first()).copied()
    }

    /// Returns the Merkle proof path for leaf at `index`.
    pub fn proof(&self, index: usize) -> Vec<[u8; 32]> {
        let mut proof = Vec::new();
        let mut idx = index;
        for level in &self.levels[..self.levels.len().saturating_sub(1)] {
            let sibling = if idx % 2 == 0 {
                level.get(idx + 1).unwrap_or(&level[idx])
            } else {
                &level[idx - 1]
            };
            proof.push(*sibling);
            idx /= 2;
        }
        proof
    }
}

// ─── Query Filters ────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct EventFilter {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub category: Option<EventCategory>,
    pub severity_min: Option<Severity>,
    pub time_from: Option<u128>,
    pub time_to: Option<u128>,
}

// ─── Anomaly Detection ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub alert_id: u64,
    pub detected_at: u128,
    pub description: String,
    pub related_event_ids: Vec<u64>,
    pub severity: Severity,
}

struct AnomalyDetector {
    /// (actor, window_start_ns) → trade count
    trade_window: HashMap<String, (u128, u64)>,
    admin_window: HashMap<String, (u128, u64)>,
    alert_counter: u64,
}

impl AnomalyDetector {
    const TRADE_WINDOW_NS: u128 = 60_000_000_000; // 1 minute
    const MAX_TRADES_PER_WINDOW: u64 = 50;
    const MAX_ADMIN_PER_WINDOW: u64 = 5;

    fn new() -> Self {
        Self {
            trade_window: HashMap::new(),
            admin_window: HashMap::new(),
            alert_counter: 0,
        }
    }

    fn inspect(&mut self, event: &AuditEvent) -> Option<AnomalyAlert> {
        match event.category {
            EventCategory::Trading => self.check_trade_volume(event),
            EventCategory::Administrative => self.check_admin_burst(event),
            _ => None,
        }
    }

    fn check_trade_volume(&mut self, event: &AuditEvent) -> Option<AnomalyAlert> {
        let entry = self.trade_window.entry(event.actor.clone()).or_insert((event.timestamp, 0));
        if event.timestamp - entry.0 > Self::TRADE_WINDOW_NS {
            *entry = (event.timestamp, 1);
            None
        } else {
            entry.1 += 1;
            if entry.1 > Self::MAX_TRADES_PER_WINDOW {
                self.alert_counter += 1;
                Some(AnomalyAlert {
                    alert_id: self.alert_counter,
                    detected_at: now_ns(),
                    description: format!(
                        "Actor '{}' exceeded {} trades/min (current: {})",
                        event.actor, Self::MAX_TRADES_PER_WINDOW, entry.1
                    ),
                    related_event_ids: vec![event.id],
                    severity: Severity::Warning,
                })
            } else {
                None
            }
        }
    }

    fn check_admin_burst(&mut self, event: &AuditEvent) -> Option<AnomalyAlert> {
        let entry = self.admin_window.entry(event.actor.clone()).or_insert((event.timestamp, 0));
        if event.timestamp - entry.0 > Self::TRADE_WINDOW_NS {
            *entry = (event.timestamp, 1);
            None
        } else {
            entry.1 += 1;
            if entry.1 > Self::MAX_ADMIN_PER_WINDOW {
                self.alert_counter += 1;
                Some(AnomalyAlert {
                    alert_id: self.alert_counter,
                    detected_at: now_ns(),
                    description: format!(
                        "Suspicious admin burst from '{}': {} actions/min",
                        event.actor, entry.1
                    ),
                    related_event_ids: vec![event.id],
                    severity: Severity::Critical,
                })
            } else {
                None
            }
        }
    }
}

// ─── Retention Policy ─────────────────────────────────────────────────────────

pub struct RetentionPolicy {
    /// How long (ns) to keep events in hot storage
    pub hot_retention_ns: u128,
    /// Archive callback – in production this would push to cold storage / SIEM
    pub archive_hook: Option<Box<dyn Fn(&[AuditEvent]) + Send + Sync>>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            hot_retention_ns: 90 * 24 * 3600 * 1_000_000_000u128, // 90 days
            archive_hook: None,
        }
    }
}

// ─── SIEM Export ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SiemRecord {
    pub event_id: u64,
    pub timestamp_iso: String,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub result: String,
    pub category: String,
    pub severity: String,
    pub integrity_hash: String,
}

impl From<&AuditEvent> for SiemRecord {
    fn from(e: &AuditEvent) -> Self {
        Self {
            event_id: e.id,
            timestamp_iso: format_ns(e.timestamp),
            actor: e.actor.clone(),
            action: e.action.clone(),
            target: e.target.clone(),
            result: e.result.clone(),
            category: format!("{:?}", e.category),
            severity: format!("{:?}", e.severity),
            integrity_hash: hex::encode(e.event_hash),
        }
    }
}

// ─── Forensic Export ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ForensicReport {
    pub incident_id: String,
    pub generated_at: u128,
    pub events: Vec<AuditEvent>,
    pub merkle_root: Option<String>,
    pub chain_valid: bool,
    pub siem_records: Vec<SiemRecord>,
}

// ─── Main AuditLog Contract ───────────────────────────────────────────────────

pub struct AuditLog {
    events: Vec<AuditEvent>,
    /// event_id → index in `events`
    index: HashMap<u64, usize>,
    counter: u64,
    /// Cached Merkle tree (rebuilt on demand / after each batch flush)
    merkle: Option<MerkleTree>,
    /// Pending batch (flushed at MAX_BATCH_SIZE or on explicit flush)
    pending_batch: Vec<AuditEvent>,
    anomaly_detector: AnomalyDetector,
    pub anomaly_alerts: Vec<AnomalyAlert>,
    pub retention: RetentionPolicy,
}

impl AuditLog {
    pub const MAX_BATCH_SIZE: usize = 100;

    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            index: HashMap::new(),
            counter: 0,
            merkle: None,
            pending_batch: Vec::new(),
            anomaly_detector: AnomalyDetector::new(),
            anomaly_alerts: Vec::new(),
            retention: RetentionPolicy::default(),
        }
    }

    // ── Recording ────────────────────────────────────────────────────────────

    pub fn record(
        &mut self,
        actor: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        result: impl Into<String>,
        gas_used: u64,
        state_hash: [u8; 32],
        category: EventCategory,
        severity: Severity,
    ) -> u64 {
        let prev_hash = self.events.last().map(|e| e.event_hash).unwrap_or([0u8; 32]);
        self.counter += 1;

        let mut event = AuditEvent {
            id: self.counter,
            timestamp: now_ns(),
            actor: actor.into(),
            action: action.into(),
            target: target.into(),
            result: result.into(),
            gas_used,
            state_hash,
            category,
            severity,
            prev_hash,
            event_hash: [0u8; 32],
        };
        event.event_hash = event.compute_hash();

        // Anomaly detection
        if let Some(alert) = self.anomaly_detector.inspect(&event) {
            self.anomaly_alerts.push(alert);
        }

        self.pending_batch.push(event);

        if self.pending_batch.len() >= Self::MAX_BATCH_SIZE {
            self.flush_batch();
        }

        self.counter
    }

    /// Drain the pending batch into committed storage and rebuild Merkle tree.
    pub fn flush_batch(&mut self) {
        if self.pending_batch.is_empty() {
            return;
        }
        for event in self.pending_batch.drain(..) {
            let idx = self.events.len();
            self.index.insert(event.id, idx);
            self.events.push(event);
        }
        self.rebuild_merkle();
        self.apply_retention();
    }

    // ── Query ─────────────────────────────────────────────────────────────────

    pub fn query_events(&self, filter: &EventFilter) -> Vec<(&AuditEvent, Vec<[u8; 32]>)> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                filter.actor.as_ref().map_or(true, |a| &e.actor == a)
                    && filter.action.as_ref().map_or(true, |a| &e.action == a)
                    && filter.category.as_ref().map_or(true, |c| &e.category == c)
                    && filter
                        .severity_min
                        .as_ref()
                        .map_or(true, |s| &e.severity >= s)
                    && filter.time_from.map_or(true, |t| e.timestamp >= t)
                    && filter.time_to.map_or(true, |t| e.timestamp <= t)
            })
            .map(|(idx, e)| {
                let proof = self
                    .merkle
                    .as_ref()
                    .map(|m| m.proof(idx))
                    .unwrap_or_default();
                (e, proof)
            })
            .collect()
    }

    // ── Integrity Verification ────────────────────────────────────────────────

    pub fn verify_event_integrity(&self, event_id: u64) -> Result<[u8; 32], String> {
        let idx = *self
            .index
            .get(&event_id)
            .ok_or_else(|| format!("Event {} not found", event_id))?;
        let event = &self.events[idx];

        // 1. Self-consistency
        if !event.is_self_consistent() {
            return Err(format!("Event {} hash mismatch – tampered!", event_id));
        }

        // 2. Chain linkage
        if idx > 0 {
            let prev = &self.events[idx - 1];
            if event.prev_hash != prev.event_hash {
                return Err(format!(
                    "Event {} chain broken at predecessor {}",
                    event_id,
                    event_id - 1
                ));
            }
        }

        // 3. Return Merkle proof root
        Ok(self
            .merkle
            .as_ref()
            .and_then(|m| m.root())
            .unwrap_or([0u8; 32]))
    }

    /// Verify the entire chain from genesis to tip.
    pub fn verify_chain(&self) -> Result<(), String> {
        for (i, event) in self.events.iter().enumerate() {
            if !event.is_self_consistent() {
                return Err(format!("Chain broken: event {} hash invalid", event.id));
            }
            if i > 0 && event.prev_hash != self.events[i - 1].event_hash {
                return Err(format!("Chain broken: event {} prev_hash mismatch", event.id));
            }
        }
        Ok(())
    }

    // ── Forensic Export ───────────────────────────────────────────────────────

    pub fn forensic_export(&self, incident_id: impl Into<String>) -> ForensicReport {
        let all_events: Vec<AuditEvent> = self.events.clone();
        let chain_valid = self.verify_chain().is_ok();
        let siem_records = all_events.iter().map(SiemRecord::from).collect();
        let merkle_root = self
            .merkle
            .as_ref()
            .and_then(|m| m.root())
            .map(hex::encode);

        ForensicReport {
            incident_id: incident_id.into(),
            generated_at: now_ns(),
            events: all_events,
            merkle_root,
            chain_valid,
            siem_records,
        }
    }

    /// Export events matching a filter as SIEM-ready JSON strings (NDJSON).
    pub fn siem_export(&self, filter: &EventFilter) -> String {
        self.query_events(filter)
            .iter()
            .map(|(e, _)| serde_json::to_string(&SiemRecord::from(*e)).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ── State Reconstruction ─────────────────────────────────────────────────

    /// Replay all events up to `until_id` to reconstruct historical state hashes.
    pub fn reconstruct_state_at(&self, until_id: u64) -> Option<[u8; 32]> {
        self.events
            .iter()
            .take_while(|e| e.id <= until_id)
            .last()
            .map(|e| e.state_hash)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn rebuild_merkle(&mut self) {
        let hashes: Vec<[u8; 32]> = self.events.iter().map(|e| e.event_hash).collect();
        self.merkle = if hashes.is_empty() {
            None
        } else {
            Some(MerkleTree::build(&hashes))
        };
    }

    fn apply_retention(&mut self) {
        let cutoff = now_ns().saturating_sub(self.retention.hot_retention_ns);
        let expired: Vec<AuditEvent> = self
            .events
            .iter()
            .filter(|e| e.timestamp < cutoff)
            .cloned()
            .collect();

        if !expired.is_empty() {
            if let Some(hook) = &self.retention.archive_hook {
                hook(&expired);
            }
            self.events.retain(|e| e.timestamp >= cutoff);
            // Rebuild index
            self.index.clear();
            for (i, e) in self.events.iter().enumerate() {
                self.index.insert(e.id, i);
            }
            self.rebuild_merkle();
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn pending_len(&self) -> usize {
        self.pending_batch.len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Utility ─────────────────────────────────────────────────────────────────

pub fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn format_ns(ns: u128) -> String {
    let secs = ns / 1_000_000_000;
    format!("{}", secs) // simplified; production would use chrono
}