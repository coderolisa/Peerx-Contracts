#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Report(u32),          // report_id => report
    ReportCount,
    Reward(u32),          // report_id => reward amount
    Status(u32),          // report_id => status
    Reporter(u32),        // report_id => user
}


