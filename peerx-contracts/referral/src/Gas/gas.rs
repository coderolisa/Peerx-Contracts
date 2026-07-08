use soroban_sdk::{Env, Address};
use crate::storage::DataKey;

pub fn record_volume(env: &Env, user: Address, amount: i128) {
    let current: i128 = env
        .storage()
        .instance()
        .get(&DataKey::Volume(user.clone()))
        .unwrap_or(0);

    env.storage()
        .instance()
        .set(&DataKey::Volume(user.clone()), &(current + amount));

    // assign commission
    if let Some(referrer) = env.storage().instance().get::<_, Address>(&DataKey::Referrer(user.clone())) {
        let rate: i128 = env.storage().instance().get(&DataKey::CommissionRate).unwrap();

        let commission = amount * rate / 100;

        let current_commission: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Commission(referrer.clone()))
            .unwrap_or(0);

        env.storage().instance().set(
            &DataKey::Commission(referrer),
            &(current_commission + commission),
        );
    }
}

use soroban_sdk::{Env, Address};
use crate::storage::{DataKey, Status};

pub fn release_batch(env: &Env) {
    let mut start: u32 = env.storage().instance().get(&DataKey::QueueStart).unwrap_or(0);
    let end: u32 = env.storage().instance().get(&DataKey::QueueEnd).unwrap_or(0);
    let batch: u32 = env.storage().instance().get(&DataKey::BatchSize).unwrap();

    let mut count = 0;

    while start < end && count < batch {
        let user: Address = env.storage().instance().get(&DataKey::Queue(start)).unwrap();

        env.storage()
            .instance()
            .set(&DataKey::Status(user), &Status::Invited);

        start += 1;
        count += 1;
    }

    env.storage().instance().set(&DataKey::QueueStart, &start);
}

use soroban_sdk::{Env, Address};
use crate::storage::{DataKey, ReportStatus};

pub fn claim(env: &Env, user: Address, report_id: u32) -> i128 {
    user.require_auth();

    let reporter: Address = env.storage().instance().get(&DataKey::Reporter(report_id)).unwrap();

    if user != reporter {
        panic!("Not report owner");
    }

    let status: ReportStatus = env.storage().instance().get(&DataKey::Status(report_id)).unwrap();

    match status {
        ReportStatus::Approved => {
            let reward: i128 = env.storage().instance().get(&DataKey::Reward(report_id)).unwrap();

            env.storage().instance().set(&DataKey::Status(report_id), &ReportStatus::Paid);

            // ⚠️ integrate token transfer here in real system
            reward
        }
        _ => panic!("Not eligible"),
    }
}