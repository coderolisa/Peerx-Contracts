use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Overall health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All checks passed
    Healthy,
    
    /// Some non-critical checks failed
    Degraded,
    
    /// Critical checks failed
    Unhealthy,
}

impl HealthStatus {
    /// Convert to exit code
    /// 0 = healthy, 1 = degraded (warnings), 2 = unhealthy (critical)
    pub fn to_exit_code(&self) -> i32 {
        match self {
            HealthStatus::Healthy => 0,
            HealthStatus::Degraded => 1,
            HealthStatus::Unhealthy => 2,
        }
    }
}

/// Status of an individual health check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed
    Pass,
    
    /// Check failed but non-critical
    Warn,
    
    /// Check failed and is critical
    Fail,
    
    /// Check could not be completed
    Error,
}

impl CheckStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, CheckStatus::Pass)
    }
    
    pub fn is_critical(&self) -> bool {
        matches!(self, CheckStatus::Fail | CheckStatus::Error)
    }
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// Name of the check
    pub name: String,
    
    /// Check status
    pub status: CheckStatus,
    
    /// Human-readable message
    pub message: String,
    
    /// Duration in milliseconds
    pub duration_ms: u64,
    
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Complete health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall health status
    pub status: HealthStatus,
    
    /// Timestamp of the check
    pub timestamp: DateTime<Utc>,
    
    /// Individual check results
    pub checks: Vec<Check>,
    
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    
    /// Summary statistics
    pub summary: HealthSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total: usize,
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub errors: usize,
}

impl HealthReport {
    pub fn new() -> Self {
        Self {
            status: HealthStatus::Healthy,
            timestamp: Utc::now(),
            checks: Vec::new(),
            total_duration_ms: 0,
            summary: HealthSummary {
                total: 0,
                passed: 0,
                warnings: 0,
                failed: 0,
                errors: 0,
            },
        }
    }
    
    pub fn add_check(&mut self, check: Check) {
        // Update summary
        self.summary.total += 1;
        match check.status {
            CheckStatus::Pass => self.summary.passed += 1,
            CheckStatus::Warn => self.summary.warnings += 1,
            CheckStatus::Fail => self.summary.failed += 1,
            CheckStatus::Error => self.summary.errors += 1,
        }
        
        self.checks.push(check);
    }
    
    pub fn finalize(&mut self) {
        // Determine overall status based on check results
        if self.summary.failed > 0 || self.summary.errors > 0 {
            self.status = HealthStatus::Unhealthy;
        } else if self.summary.warnings > 0 {
            self.status = HealthStatus::Degraded;
        } else {
            self.status = HealthStatus::Healthy;
        }
        
        // Calculate total duration
        self.total_duration_ms = self.checks.iter().map(|c| c.duration_ms).sum();
    }
}

impl Default for HealthReport {
    fn default() -> Self {
        Self::new()
    }
}
