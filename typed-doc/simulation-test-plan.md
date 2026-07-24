# Simulation Module — Test Plan

## Scope

Test coverage for `packages/devkit/src/simulation/` covering `fee_model`,
`network_load`, and `congestion_predictor`.

## 1. FeeModel tests

### Unit tests (in-module)

| Test | Description | Expected |
|---|---|---|
| `test_new_validated_ok` | Valid config produces `FeeModel` | Ok |
| `test_new_validated_zero_base_fee` | base_fee=0 returns error | Err |
| `test_new_validated_negative_base_fee` | base_fee < 0 returns error | Err |
| `test_new_validated_spike_prob_out_of_range` | spike_probability > 1.0 returns error | Err |
| `test_generate_returns_expected_count` | generate(n) returns n FeePoints | len == n |
| `test_generate_all_timestamps_ascending` | Timestamps are strictly increasing | pass |
| `test_generate_all_ledgers_ascending` | Ledger seqs are strictly increasing | pass |
| `test_sample_fee_in_range` | sampled fee is >= 0 | pass |
| `test_run_scenarios_returns_all_scenarios` | run_scenarios returns correct count | len matches |
| `test_generate_baseline_has_no_spikes` | baseline has zero spike flags | all false |
| `test_inject_spikes_creates_spikes` | inject_spikes marks some as spikes | some true |
| `test_gaussian_noise_zero_mean` | gaussian noise has approx zero mean | |mean| < 0.1 |
| `test_mode_fee_returns_most_common` | mode_fee identifies statistical mode | correct value |
| `test_generate_timestamps_count` | generate_timestamps returns n timestamps | len == n |
| `test_export_csv_produces_header` | CSV output starts with header row | correct |
| `test_deterministic_seed` | same seed produces same output | identical |

### Integration tests (`tests/`)

| Test | Description |
|---|---|
| `simulation_fee_model_deterministic` | Same seed + config → identical run output |
| `simulation_run_with_network_load` | FeeModel + NetworkLoad produce coordinated results |
| `simulation_full_pipeline` | Full simulation pipeline produces valid FeeRecords |

## 2. NetworkLoad tests

| Test | Description | Expected |
|---|---|---|
| `test_new_config_ok` | Valid config produces NetworkLoad | Ok |
| `test_generate_ledger_count` | generate(n) returns n ledgers | len == n |
| `test_simulate_tx_counts_non_negative` | All tx_count >= 0 | pass |
| `test_diurnal_multiplier_peaks_daytime` | Midday multiplier > midnight | true |
| `test_capacity_pressure_fee_positive` | pressure fee is > 0 | true |

## 3. CongestionPredictor tests

| Test | Description | Expected |
|---|---|---|
| `test_predict_low_congestion` | Low usage → Low level | CongestionLevel::Low |
| `test_predict_high_congestion` | High usage → High/Critical level | >= High |
| `test_congestion_score_range` | Score is 0.0..=1.0 | in range |
| `test_congestion_label_mapping` | Each level maps to correct label | correct |

## Acceptance criteria

- [ ] All unit tests pass
- [ ] Integration tests cover inter-module scenarios
- [ ] Deterministic seed test ensures reproducibility
- [ ] Edge cases (zero, negative, overflow) are covered
- [ ] Tests run in CI via `cargo test -p stellar-devkit`
