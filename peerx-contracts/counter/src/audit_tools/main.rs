// audit_tools/src/main.rs
// Forensic analysis CLI for AuditLog exports

use std::fs;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use sha2::{Sha256, Digest};

// ─── CLI Definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "audit-tools")]
#[command(about = "Forensic analysis CLI for cryptographic audit trail exports")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify the integrity of an exported audit report (JSON)
    Verify {
        #[arg(help = "Path to forensic_report.json")]
        file: PathBuf,
    },
    /// Query events from an exported report
    Query {
        #[arg(help = "Path to forensic_report.json")]
        file: PathBuf,
        #[arg(long, help = "Filter by actor")]
        actor: Option<String>,
        #[arg(long, help = "Filter by action")]
        action: Option<String>,
        #[arg(long, help = "Filter by category (Administrative|Trading|Security|System)")]
        category: Option<String>,
        #[arg(long, help = "Unix epoch seconds (from)")]
        from: Option<u64>,
        #[arg(long, help = "Unix epoch seconds (to)")]
        to: Option<u64>,
    },
    /// Show chain statistics for an exported report
    Stats {
        #[arg(help = "Path to forensic_report.json")]
        file: PathBuf,
    },
    /// Export events to NDJSON for SIEM ingestion
    SiemExport {
        #[arg(help = "Path to forensic_report.json")]
        file: PathBuf,
        #[arg(short, long, help = "Output file (stdout if omitted)")]
        output: Option<PathBuf>,
    },
    /// Re-derive Merkle root from event hashes to confirm report root
    MerkleCheck {
        #[arg(help = "Path to forensic_report.json")]
        file: PathBuf,
    },
}

// ─── Shared data structures (mirrors audit_log.rs – kept minimal for the tool) ──

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct AuditEvent {
    id: u64,
    timestamp: u128,
    actor: String,
    action: String,
    target: String,
    result: String,
    gas_used: u64,
    state_hash: [u8; 32],
    category: String,
    severity: String,
    prev_hash: [u8; 32],
    event_hash: [u8; 32],
}

impl AuditEvent {
    fn recompute_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.id.to_le_bytes());
        h.update(self.timestamp.to_le_bytes());
        h.update(self.actor.as_bytes());
        h.update(self.action.as_bytes());
        h.update(self.target.as_bytes());
        h.update(self.result.as_bytes());
        h.update(self.gas_used.to_le_bytes());
        h.update(self.state_hash);
        h.update(self.prev_hash);
        h.finalize().into()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SiemRecord {
    event_id: u64,
    timestamp_iso: String,
    actor: String,
    action: String,
    target: String,
    result: String,
    category: String,
    severity: String,
    integrity_hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ForensicReport {
    incident_id: String,
    generated_at: u128,
    events: Vec<AuditEvent>,
    merkle_root: Option<String>,
    chain_valid: bool,
    siem_records: Vec<SiemRecord>,
}

// ─── Merkle helper ────────────────────────────────────────────────────────────

fn merkle_root(hashes: &[[u8; 32]]) -> Option<[u8; 32]> {
    if hashes.is_empty() {
        return None;
    }
    let mut current: Vec<[u8; 32]> = hashes.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for chunk in current.chunks(2) {
            let mut h = Sha256::new();
            h.update(chunk[0]);
            h.update(chunk.get(1).unwrap_or(&chunk[0]));
            next.push(h.finalize().into());
        }
        current = next;
    }
    current.into_iter().next()
}

// ─── Command implementations ──────────────────────────────────────────────────

fn load_report(path: &PathBuf) -> ForensicReport {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| { eprintln!("Cannot read {}: {}", path.display(), e); std::process::exit(1); });
    serde_json::from_str(&json)
        .unwrap_or_else(|e| { eprintln!("Invalid report JSON: {}", e); std::process::exit(1); })
}

fn cmd_verify(file: &PathBuf) {
    let report = load_report(file);
    println!("=== Verifying report: {} ===", report.incident_id);
    println!("Events: {}", report.events.len());

    let mut errors = 0usize;

    for (i, event) in report.events.iter().enumerate() {
        // Self-hash
        let expected = event.recompute_hash();
        if expected != event.event_hash {
            println!("  ✗ Event {} (id={}) – self-hash MISMATCH", i, event.id);
            errors += 1;
        }

        // Chain linkage
        if i > 0 {
            let prev_hash = report.events[i - 1].event_hash;
            if event.prev_hash != prev_hash {
                println!("  ✗ Event {} (id={}) – prev_hash MISMATCH", i, event.id);
                errors += 1;
            }
        } else if event.prev_hash != [0u8; 32] {
            println!("  ✗ Genesis event has non-zero prev_hash");
            errors += 1;
        }
    }

    if errors == 0 {
        println!("✓ All {} events verified. Chain intact.", report.events.len());
    } else {
        println!("✗ {} integrity error(s) found.", errors);
        std::process::exit(2);
    }
}

fn cmd_query(
    file: &PathBuf,
    actor: Option<String>,
    action: Option<String>,
    category: Option<String>,
    from: Option<u64>,
    to: Option<u64>,
) {
    let report = load_report(file);
    let from_ns = from.map(|s| s as u128 * 1_000_000_000);
    let to_ns = to.map(|s| s as u128 * 1_000_000_000);

    let results: Vec<&AuditEvent> = report.events.iter().filter(|e| {
        actor.as_ref().map_or(true, |a| &e.actor == a)
            && action.as_ref().map_or(true, |a| &e.action == a)
            && category.as_ref().map_or(true, |c| e.category.eq_ignore_ascii_case(c))
            && from_ns.map_or(true, |t| e.timestamp >= t)
            && to_ns.map_or(true, |t| e.timestamp <= t)
    }).collect();

    println!("{} event(s) matched:", results.len());
    for e in results {
        println!(
            "  [{:>6}] ts={:>20}  {:20}  {:30}  {} → {}  (gas={})",
            e.id, e.timestamp, e.actor, e.action, e.target, e.result, e.gas_used
        );
    }
}

fn cmd_stats(file: &PathBuf) {
    let report = load_report(file);
    println!("=== Report Statistics ===");
    println!("Incident ID  : {}", report.incident_id);
    println!("Generated at : {} ns", report.generated_at);
    println!("Total events : {}", report.events.len());
    println!("Chain valid  : {}", report.chain_valid);
    println!("Merkle root  : {}", report.merkle_root.as_deref().unwrap_or("(none)"));

    // Category breakdown
    let mut cat_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &report.events {
        *cat_counts.entry(e.category.as_str()).or_insert(0) += 1;
    }
    println!("\nCategory breakdown:");
    let mut cats: Vec<_> = cat_counts.iter().collect();
    cats.sort_by_key(|&(k, _)| k);
    for (cat, count) in cats {
        println!("  {:20} : {}", cat, count);
    }

    // Severity breakdown
    let mut sev_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &report.events {
        *sev_counts.entry(e.severity.as_str()).or_insert(0) += 1;
    }
    println!("\nSeverity breakdown:");
    let mut sevs: Vec<_> = sev_counts.iter().collect();
    sevs.sort_by_key(|&(k, _)| k);
    for (sev, count) in sevs {
        println!("  {:20} : {}", sev, count);
    }
}

fn cmd_siem_export(file: &PathBuf, output: Option<PathBuf>) {
    let report = load_report(file);
    let ndjson = report.siem_records.iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    match output {
        Some(path) => {
            fs::write(&path, &ndjson)
                .unwrap_or_else(|e| { eprintln!("Write error: {}", e); std::process::exit(1); });
            println!("Wrote {} SIEM records to {}", report.siem_records.len(), path.display());
        }
        None => println!("{}", ndjson),
    }
}

fn cmd_merkle_check(file: &PathBuf) {
    let report = load_report(file);
    let hashes: Vec<[u8; 32]> = report.events.iter().map(|e| e.event_hash).collect();
    let derived = merkle_root(&hashes).map(hex::encode);

    println!("Claimed  root: {}", report.merkle_root.as_deref().unwrap_or("(none)"));
    println!("Computed root: {}", derived.as_deref().unwrap_or("(none)"));

    if report.merkle_root == derived {
        println!("✓ Merkle root matches.");
    } else {
        println!("✗ Merkle root MISMATCH – report may have been altered.");
        std::process::exit(2);
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Verify { file } => cmd_verify(&file),
        Command::Query { file, actor, action, category, from, to } =>
            cmd_query(&file, actor, action, category, from, to),
        Command::Stats { file } => cmd_stats(&file),
        Command::SiemExport { file, output } => cmd_siem_export(&file, output),
        Command::MerkleCheck { file } => cmd_merkle_check(&file),
    }
}