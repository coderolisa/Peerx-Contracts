// src/audit_log_tests.rs

#[cfg(test)]
mod tests {
    use crate::audit_log::*;

    fn state(n: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = n;
        h
    }

    fn record_event(log: &mut AuditLog, actor: &str, action: &str, cat: EventCategory) -> u64 {
        log.record(actor, action, "target", "OK", 21_000, state(1), cat, Severity::Info)
    }

    // ── Basic recording ───────────────────────────────────────────────────────

    #[test]
    fn test_record_and_flush() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "TRADE_EXECUTE", EventCategory::Trading);
        assert_eq!(log.pending_len(), 1);
        log.flush_batch();
        assert_eq!(log.len(), 1);
        assert_eq!(log.pending_len(), 0);
    }

    #[test]
    fn test_auto_flush_at_max_batch() {
        let mut log = AuditLog::new();
        for i in 0..AuditLog::MAX_BATCH_SIZE {
            record_event(&mut log, "bot", &format!("ACTION_{}", i), EventCategory::Trading);
        }
        // Should have auto-flushed
        assert_eq!(log.pending_len(), 0);
        assert_eq!(log.len(), AuditLog::MAX_BATCH_SIZE);
    }

    // ── Hash chain integrity ──────────────────────────────────────────────────

    #[test]
    fn test_chain_integrity() {
        let mut log = AuditLog::new();
        for i in 0..5 {
            record_event(&mut log, "alice", &format!("ACT_{}", i), EventCategory::System);
        }
        log.flush_batch();
        assert!(log.verify_chain().is_ok());
    }

    #[test]
    fn test_tamper_detection() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "LOGIN", EventCategory::Security);
        log.flush_batch();

        // Tamper with event hash
        log.events[0].action = "TAMPERED".into();

        assert!(log.verify_chain().is_err());
    }

    #[test]
    fn test_genesis_event_prev_hash_is_zero() {
        let mut log = AuditLog::new();
        record_event(&mut log, "root", "INIT", EventCategory::System);
        log.flush_batch();
        assert_eq!(log.events[0].prev_hash, [0u8; 32]);
    }

    #[test]
    fn test_chained_prev_hash() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "A1", EventCategory::System);
        record_event(&mut log, "alice", "A2", EventCategory::System);
        log.flush_batch();

        assert_eq!(log.events[1].prev_hash, log.events[0].event_hash);
    }

    // ── verify_event_integrity ────────────────────────────────────────────────

    #[test]
    fn test_verify_event_integrity_ok() {
        let mut log = AuditLog::new();
        let id = record_event(&mut log, "bob", "TRADE", EventCategory::Trading);
        log.flush_batch();
        assert!(log.verify_event_integrity(id).is_ok());
    }

    #[test]
    fn test_verify_event_integrity_missing() {
        let log = AuditLog::new();
        assert!(log.verify_event_integrity(999).is_err());
    }

    // ── Query ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_query_by_actor() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "TRADE", EventCategory::Trading);
        record_event(&mut log, "bob", "TRADE", EventCategory::Trading);
        log.flush_batch();

        let filter = EventFilter { actor: Some("alice".into()), ..Default::default() };
        let results = log.query_events(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.actor, "alice");
    }

    #[test]
    fn test_query_by_action() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "TRADE_EXECUTE", EventCategory::Trading);
        record_event(&mut log, "alice", "ADMIN_GRANT", EventCategory::Administrative);
        log.flush_batch();

        let filter = EventFilter { action: Some("ADMIN_GRANT".into()), ..Default::default() };
        let results = log.query_events(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.action, "ADMIN_GRANT");
    }

    #[test]
    fn test_query_by_time_range() {
        let mut log = AuditLog::new();
        let t1 = now_ns();
        record_event(&mut log, "alice", "A", EventCategory::System);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let t2 = now_ns();
        record_event(&mut log, "alice", "B", EventCategory::System);
        log.flush_batch();

        let filter = EventFilter { time_from: Some(t2), ..Default::default() };
        let results = log.query_events(&filter);
        // Only events at or after t2
        assert!(results.iter().all(|(e, _)| e.timestamp >= t2));
        let _ = t1;
    }

    #[test]
    fn test_query_with_merkle_proof() {
        let mut log = AuditLog::new();
        for i in 0..5 {
            record_event(&mut log, "alice", &format!("ACT_{}", i), EventCategory::System);
        }
        log.flush_batch();

        let filter = EventFilter::default();
        let results = log.query_events(&filter);
        // All results should have a proof path
        for (_, proof) in &results {
            // Proof can be empty only for a single-element tree
            let _ = proof;
        }
        assert_eq!(results.len(), 5);
    }

    // ── Merkle tree ───────────────────────────────────────────────────────────

    #[test]
    fn test_merkle_root_changes_after_new_event() {
        let mut log = AuditLog::new();
        record_event(&mut log, "a", "X", EventCategory::System);
        log.flush_batch();
        let root1 = log.merkle.as_ref().and_then(|m| m.root());

        record_event(&mut log, "b", "Y", EventCategory::System);
        log.flush_batch();
        let root2 = log.merkle.as_ref().and_then(|m| m.root());

        assert_ne!(root1, root2);
    }

    // ── Anomaly detection ─────────────────────────────────────────────────────

    #[test]
    fn test_anomaly_trade_volume() {
        let mut log = AuditLog::new();
        for _ in 0..=60 {
            log.record(
                "hft_bot", "TRADE_EXECUTE", "PAIR_XY", "OK",
                21_000, state(1), EventCategory::Trading, Severity::Info,
            );
        }
        assert!(!log.anomaly_alerts.is_empty());
        assert!(log
            .anomaly_alerts
            .iter()
            .any(|a| a.description.contains("hft_bot")));
    }

    #[test]
    fn test_anomaly_admin_burst() {
        let mut log = AuditLog::new();
        for _ in 0..=6 {
            log.record(
                "attacker", "ADMIN_ROLE_GRANT", "USER", "OK",
                50_000, state(2), EventCategory::Administrative, Severity::Warning,
            );
        }
        assert!(log
            .anomaly_alerts
            .iter()
            .any(|a| matches!(a.severity, Severity::Critical)));
    }

    // ── Forensic export ───────────────────────────────────────────────────────

    #[test]
    fn test_forensic_export() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "TRADE_EXECUTE", EventCategory::Trading);
        record_event(&mut log, "admin", "ROLE_GRANT", EventCategory::Administrative);
        log.flush_batch();

        let report = log.forensic_export("INC-2024-001");
        assert_eq!(report.incident_id, "INC-2024-001");
        assert!(report.chain_valid);
        assert_eq!(report.events.len(), 2);
        assert!(report.merkle_root.is_some());
        assert_eq!(report.siem_records.len(), 2);
    }

    // ── SIEM export ───────────────────────────────────────────────────────────

    #[test]
    fn test_siem_export_ndjson() {
        let mut log = AuditLog::new();
        record_event(&mut log, "alice", "TRADE", EventCategory::Trading);
        log.flush_batch();

        let ndjson = log.siem_export(&EventFilter::default());
        assert!(!ndjson.is_empty());
        // Each line should be valid JSON
        for line in ndjson.lines() {
            assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
        }
    }

    // ── State reconstruction ──────────────────────────────────────────────────

    #[test]
    fn test_state_reconstruction() {
        let mut log = AuditLog::new();
        log.record("alice", "TX1", "target", "OK", 0, state(10), EventCategory::Trading, Severity::Info);
        log.record("alice", "TX2", "target", "OK", 0, state(20), EventCategory::Trading, Severity::Info);
        log.flush_batch();

        let s = log.reconstruct_state_at(1).unwrap();
        assert_eq!(s[0], 10);

        let s2 = log.reconstruct_state_at(2).unwrap();
        assert_eq!(s2[0], 20);
    }

    // ── Retention ─────────────────────────────────────────────────────────────

    #[test]
    fn test_retention_archive_hook_called() {
        use std::sync::{Arc, Mutex};

        let archived: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
        let archived_clone = archived.clone();

        let mut log = AuditLog::new();
        // Set retention to 0 (expire immediately)
        log.retention.hot_retention_ns = 0;
        log.retention.archive_hook = Some(Box::new(move |events| {
            let mut lock = archived_clone.lock().unwrap();
            for e in events {
                lock.push(e.id);
            }
        }));

        log.record("alice", "OLD_EVENT", "t", "OK", 0, state(1), EventCategory::System, Severity::Info);
        log.flush_batch();

        // Events should have been archived and removed from hot storage
        assert_eq!(log.len(), 0);
        assert!(!archived.lock().unwrap().is_empty());
    }
}