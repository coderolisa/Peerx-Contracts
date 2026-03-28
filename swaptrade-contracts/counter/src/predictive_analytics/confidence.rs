use soroban_sdk::Env;

/// Simple confidence calculation (example: scaled by volume and signal strength)
pub fn calculate(signal: i32, volume: i32) -> u8 {
    let base_conf: u8 = match signal {
        1 => 80,   // BUY
        -1 => 70,  // SELL
        _ => 50,   // HOLD
    };

    // Scale by volume (capped at 100)
    let scaled = base_conf as u32 + (volume as u32 / 1000);
    if scaled > 100 {
        100
    } else {
        scaled as u8
    }
}