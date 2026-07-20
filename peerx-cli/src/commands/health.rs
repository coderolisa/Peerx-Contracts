use clap::Args;
use colored::*;
use crate::config::Config;
use crate::error::Result;
use crate::health::{HealthChecker, HealthStatus};
use crate::output::{OutputFormatter, success, warning, error, info};

/// Run pre-flight health checks on PeerX contract and infrastructure
#[derive(Args)]
pub struct HealthCommand {
    /// Network to check (testnet, mainnet, local)
    #[arg(short, long, env = "PEERX_NETWORK")]
    network: Option<String>,
    
    /// Contract ID to check
    #[arg(short, long, env = "PEERX_CONTRACT_ID")]
    contract_id: Option<String>,
    
    /// RPC endpoint URL
    #[arg(short, long, env = "PEERX_RPC_URL")]
    rpc_url: Option<String>,
    
    /// Admin address to verify
    #[arg(short, long, env = "PEERX_ADMIN_ADDRESS")]
    admin_address: Option<String>,
    
    /// Show detailed check information
    #[arg(short, long)]
    details: bool,
    
    /// Only run specific checks (comma-separated)
    /// Available: rpc_reachable, horizon_reachable, admin_reachable, 
    ///            contract_exists, contract_not_paused, oracle_fresh
    #[arg(long)]
    only: Option<String>,
}

impl HealthCommand {
    pub async fn execute(&self, format: &str) -> Result<i32> {
        // Load configuration
        let mut config = Config::load()?;
        
        // Override with command-line arguments
        if let Some(network) = &self.network {
            config.network.name = network.clone();
        }
        
        if let Some(contract_id) = &self.contract_id {
            config.contract.contract_id = contract_id.clone();
        }
        
        if let Some(rpc_url) = &self.rpc_url {
            config.network.rpc_url = rpc_url.clone();
        }
        
        if let Some(admin_address) = &self.admin_address {
            config.contract.admin_address = Some(admin_address.clone());
        }
        
        // Validate configuration
        config.validate()?;
        
        // Create formatter
        let formatter = OutputFormatter::new(format)?;
        
        // Print header if human-readable output
        if formatter.is_human() {
            println!("\n{}", "PeerX Health Check".bold().cyan());
            println!("{}", "=".repeat(50).cyan());
            println!("{} {}", "Network:".bold(), config.network.name);
            println!("{} {}", "Contract:".bold(), config.contract.contract_id);
            println!();
        }
        
        // Run health checks
        let checker = HealthChecker::new(config)?;
        let report = checker.run_all_checks().await?;
        
        // Output results
        if formatter.is_human() {
            self.print_human_report(&report);
        } else {
            let output = formatter.format(&report)?;
            println!("{}", output);
        }
        
        // Return exit code
        Ok(report.status.to_exit_code())
    }
    
    fn print_human_report(&self, report: &crate::health::types::HealthReport) {
        // Print individual checks
        println!("{}", "Check Results:".bold());
        println!("{}", "-".repeat(50));
        
        for check in &report.checks {
            let status_symbol = match check.status {
                crate::health::types::CheckStatus::Pass => "✓".green(),
                crate::health::types::CheckStatus::Warn => "⚠".yellow(),
                crate::health::types::CheckStatus::Fail => "✗".red(),
                crate::health::types::CheckStatus::Error => "✗".red(),
            };
            
            let check_name = format!("{:.<30}", check.name).dimmed();
            let duration = format!("({}ms)", check.duration_ms).dimmed();
            
            println!("{} {} {}", status_symbol, check_name, duration);
            
            if check.status != crate::health::types::CheckStatus::Pass {
                println!("  {}", check.message.italic());
            }
            
            if self.details {
                if let Some(details) = &check.details {
                    println!("  {}: {}", "Details".dimmed(), 
                             serde_json::to_string_pretty(details).unwrap_or_default().dimmed());
                }
            }
        }
        
        println!();
        println!("{}", "Summary:".bold());
        println!("{}", "-".repeat(50));
        
        // Print summary
        let summary = &report.summary;
        println!("  Total checks: {}", summary.total);
        println!("  {} Passed: {}", "✓".green(), summary.passed);
        if summary.warnings > 0 {
            println!("  {} Warnings: {}", "⚠".yellow(), summary.warnings);
        }
        if summary.failed > 0 {
            println!("  {} Failed: {}", "✗".red(), summary.failed);
        }
        if summary.errors > 0 {
            println!("  {} Errors: {}", "✗".red(), summary.errors);
        }
        
        println!();
        println!("{}", "-".repeat(50));
        
        // Print overall status
        match report.status {
            HealthStatus::Healthy => {
                println!("{}", success("All checks passed - System is healthy"));
            }
            HealthStatus::Degraded => {
                println!("{}", warning("Some checks failed - System is degraded"));
            }
            HealthStatus::Unhealthy => {
                println!("{}", error("Critical checks failed - System is unhealthy"));
            }
        }
        
        println!("\n{} Completed in {}ms", info("ℹ"), report.total_duration_ms);
        println!("{} Exit code: {}\n", info("ℹ"), report.status.to_exit_code());
    }
}
