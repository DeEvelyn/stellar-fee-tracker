# Simulation Module — Span Instrumentation Design

## Overview

Add OpenTelemetry-style span instrumentation to the `devkit/src/simulation/` module.
Each major operation in the simulation pipeline should produce a traced span with
start/end timestamps, duration, and key metadata.

## Spans to instrument

### FeeModel::run()
- **Span name**: `simulation.fee_model.run`
- **Attributes**: `scenario_count`, `total_points`, `spike_count`
- **Events**: `sampling_started`, `spike_injected`, `run_complete`
- **Child spans**: `fee_model.generate_fees`, `fee_model.inject_spikes`

### NetworkLoad::simulate()
- **Span name**: `simulation.network_load.simulate`
- **Attributes**: `ledger_count`, `min_tx`, `max_tx`, `capacity_pct`
- **Events**: `ledger_generated(n)`, `pressure_calculated`, `load_complete`

### CongestionPredictor::predict()
- **Span name**: `simulation.congestion.predict`
- **Attributes**: `recent_avg_fee`, `capacity_usage`, `congestion_level`
- **Events**: `input_validated`, `prediction_computed`

## Instrumentation API

Use a lightweight span trait:

```rust
pub trait SimulationSpan {
    fn start(name: &str, attrs: Vec<(&str, String)>) -> Self;
    fn event(&self, name: &str, attrs: Vec<(&str, String)>);
    fn end(&self);
    fn duration_ms(&self) -> u64;
}
```

A no-op `NullSpan` impl is provided for non-instrumented contexts.
A `TracingSpan` impl wraps `tracing::Span` for production use.

## Span context propagation

1. Each simulation run creates a root span
2. Major sub-operations create child spans
3. Spans are exported as structured JSON when the `instrumentation` feature is enabled
4. Span context includes trace_id, span_id, parent_span_id

## File structure

```
devkit/src/simulation/
├── mod.rs                    # Re-export instrumentation
├── fee_model.rs              # Instrumented FeeModel methods
├── network_load.rs           # Instrumented NetworkLoad methods
├── congestion_predictor.rs   # Instrumented CongestionPredictor methods
└── instrumentation.rs        # NEW: Span trait + TracingSpan impl
```

## Acceptance criteria

- [ ] FeeModel::run() produces a traced span with child spans
- [ ] NetworkLoad::simulate() produces a traced span
- [ ] CongestionPredictor::predict() produces a traced span
- [ ] Span events include key attributes and timestamps
- [ ] NullSpan compiles to zero overhead when feature is disabled
