#![no_main]
use libfuzzer_sys::fuzz_target;
use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let base_fee = (u16::from_le_bytes([data[0], data[1]]) as u64).saturating_add(1);
    let spike_probability = (data[2] as f64) / 255.0;
    let ledger_count = (data[3] as u64).saturating_add(1).min(10_000);
    let config = FeeModelConfig {
        base_fee,
        spike_probability,
        ledger_count,
        ..Default::default()
    };
    if config.validate().is_ok() {
        let _ = FeeModel::run(&config);
    }
});
