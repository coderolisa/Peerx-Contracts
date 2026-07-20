use peerx_cli::health::types::{HealthReport, Check, CheckStatus, HealthStatus};

#[test]
fn test_health_report_creation() {
    let report = HealthReport::new();
    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.checks.len(), 0);
    assert_eq!(report.summary.total, 0);
}

#[test]
fn test_health_status_exit_codes() {
    assert_eq!(HealthStatus::Healthy.to_exit_code(), 0);
    assert_eq!(HealthStatus::Degraded.to_exit_code(), 1);
    assert_eq!(HealthStatus::Unhealthy.to_exit_code(), 2);
}

#[test]
fn test_check_status_methods() {
    assert!(CheckStatus::Pass.is_healthy());
    assert!(!CheckStatus::Warn.is_healthy());
    assert!(!CheckStatus::Fail.is_healthy());
    assert!(!CheckStatus::Error.is_healthy());
    
    assert!(!CheckStatus::Pass.is_critical());
    assert!(!CheckStatus::Warn.is_critical());
    assert!(CheckStatus::Fail.is_critical());
    assert!(CheckStatus::Error.is_critical());
}

#[test]
fn test_report_finalize_healthy() {
    let mut report = HealthReport::new();
    
    report.add_check(Check {
        name: "test1".to_string(),
        status: CheckStatus::Pass,
        message: "OK".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "test2".to_string(),
        status: CheckStatus::Pass,
        message: "OK".to_string(),
        duration_ms: 20,
        details: None,
    });
    
    report.finalize();
    
    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.summary.passed, 2);
    assert_eq!(report.summary.total, 2);
    assert_eq!(report.total_duration_ms, 30);
}

#[test]
fn test_report_finalize_degraded() {
    let mut report = HealthReport::new();
    
    report.add_check(Check {
        name: "test1".to_string(),
        status: CheckStatus::Pass,
        message: "OK".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "test2".to_string(),
        status: CheckStatus::Warn,
        message: "Warning".to_string(),
        duration_ms: 20,
        details: None,
    });
    
    report.finalize();
    
    assert_eq!(report.status, HealthStatus::Degraded);
    assert_eq!(report.summary.passed, 1);
    assert_eq!(report.summary.warnings, 1);
    assert_eq!(report.summary.total, 2);
}

#[test]
fn test_report_finalize_unhealthy() {
    let mut report = HealthReport::new();
    
    report.add_check(Check {
        name: "test1".to_string(),
        status: CheckStatus::Pass,
        message: "OK".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "test2".to_string(),
        status: CheckStatus::Fail,
        message: "Failed".to_string(),
        duration_ms: 20,
        details: None,
    });
    
    report.finalize();
    
    assert_eq!(report.status, HealthStatus::Unhealthy);
    assert_eq!(report.summary.passed, 1);
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.total, 2);
}

#[test]
fn test_report_with_errors() {
    let mut report = HealthReport::new();
    
    report.add_check(Check {
        name: "test1".to_string(),
        status: CheckStatus::Error,
        message: "Error occurred".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.finalize();
    
    assert_eq!(report.status, HealthStatus::Unhealthy);
    assert_eq!(report.summary.errors, 1);
    assert_eq!(report.summary.total, 1);
}

#[test]
fn test_summary_counts() {
    let mut report = HealthReport::new();
    
    report.add_check(Check {
        name: "pass1".to_string(),
        status: CheckStatus::Pass,
        message: "OK".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "pass2".to_string(),
        status: CheckStatus::Pass,
        message: "OK".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "warn1".to_string(),
        status: CheckStatus::Warn,
        message: "Warning".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "fail1".to_string(),
        status: CheckStatus::Fail,
        message: "Failed".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.add_check(Check {
        name: "error1".to_string(),
        status: CheckStatus::Error,
        message: "Error".to_string(),
        duration_ms: 10,
        details: None,
    });
    
    report.finalize();
    
    assert_eq!(report.summary.total, 5);
    assert_eq!(report.summary.passed, 2);
    assert_eq!(report.summary.warnings, 1);
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.errors, 1);
    assert_eq!(report.status, HealthStatus::Unhealthy);
}
