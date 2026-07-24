use std::time::{Duration, Instant};

/// A timed span that records execution duration and metadata.
#[derive(Debug, Clone)]
pub struct AnalysisSpan {
    /// Span identifier.
    pub id: String,
    /// Parent span identifier.
    pub parent_id: Option<String>,
    /// Name of the analysis operation.
    pub operation: String,
    /// Start time.
    start: Instant,
    /// End time (None if still running).
    end: Option<Instant>,
    /// Key-value attributes.
    pub attributes: Vec<(String, String)>,
}

impl AnalysisSpan {
    /// Start a new span.
    pub fn start(operation: &str, id: &str, parent_id: Option<String>) -> Self {
        Self {
            id: id.to_string(),
            parent_id,
            operation: operation.to_string(),
            start: Instant::now(),
            end: None,
            attributes: Vec::new(),
        }
    }

    /// Add an attribute to the span.
    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.push((key.to_string(), value.to_string()));
        self
    }

    /// Finish the span and return the elapsed duration.
    pub fn finish(&mut self) -> Duration {
        self.end = Some(Instant::now());
        self.elapsed()
    }

    /// Return the elapsed duration (since start, or between start and end if finished).
    pub fn elapsed(&self) -> Duration {
        match self.end {
            Some(e) => e.duration_since(self.start),
            None => self.start.elapsed(),
        }
    }

    /// Return a JSON representation.
    pub fn to_json(&self) -> String {
        let parent = self
            .parent_id
            .as_ref()
            .map(|p| format!("\"parent_id\":\"{}\"", p))
            .unwrap_or_else(|| "\"parent_id\":null".into());
        let attrs: Vec<String> = self
            .attributes
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", k, v))
            .collect();
        format!(
            r#"{{"id":"{}","op":"{}","dur_ms":{},{},{},"attrs":{{{}}}}}"#,
            self.id,
            self.operation,
            self.elapsed().as_millis() as u64,
            parent,
            if self.end.is_some() { "\"finished\":true" } else { "\"finished\":false" },
            attrs.join(","),
        )
    }
}

/// Instruments analysis operations with span tracing.
pub struct AnalysisInstrumentation {
    spans: Vec<AnalysisSpan>,
    enabled: bool,
}

impl AnalysisInstrumentation {
    pub fn new(enabled: bool) -> Self {
        Self {
            spans: Vec::new(),
            enabled,
        }
    }

    /// Create and register a new span.
    pub fn span(&mut self, operation: &str, id: &str, parent_id: Option<String>) -> AnalysisSpan {
        let span = AnalysisSpan::start(operation, id, parent_id);
        self.spans.push(span.clone());
        span
    }

    /// Record the completion of a span.
    pub fn record(&mut self, span: &mut AnalysisSpan) {
        if self.enabled {
            span.finish();
        }
    }

    /// Return all completed spans.
    pub fn completed_spans(&self) -> Vec<&AnalysisSpan> {
        self.spans.iter().filter(|s| s.end.is_some()).collect()
    }

    /// Return spans for a specific operation.
    pub fn by_operation(&self, operation: &str) -> Vec<&AnalysisSpan> {
        self.spans.iter().filter(|s| s.operation == operation).collect()
    }

    /// Return the total time spent on all completed spans.
    pub fn total_time(&self) -> Duration {
        self.spans
            .iter()
            .filter_map(|s| s.end.map(|e| e.duration_since(s.start)))
            .sum()
    }

    /// Number of spans created.
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Reset all spans.
    pub fn reset(&mut self) {
        self.spans.clear();
    }

    /// Export all spans as a JSON array.
    pub fn to_json(&self) -> String {
        let items: Vec<String> = self.spans.iter().map(|s| s.to_json()).collect();
        format!("[{}]", items.join(","))
    }
}

/// Instrument the percentile analysis module with spans.
pub fn instrument_percentile(
    instrumentation: &mut AnalysisInstrumentation,
    data_points: usize,
) -> AnalysisSpan {
    let span = instrumentation.span(
        "percentile",
        &format!("pctl-{}", data_points),
        None,
    );
    span
}

/// Instrument the rolling window analysis module with spans.
pub fn instrument_rolling_window(
    instrumentation: &mut AnalysisInstrumentation,
    window_size: usize,
    parent_id: Option<String>,
) -> AnalysisSpan {
    let span = instrumentation.span(
        "rolling_window",
        &format!("rw-{}-{}", window_size, rand_id()),
        parent_id,
    );
    span
}

/// Instrument the spike classifier with spans.
pub fn instrument_spike_classifier(
    instrumentation: &mut AnalysisInstrumentation,
    total_points: usize,
    parent_id: Option<String>,
) -> AnalysisSpan {
    let mut span = instrumentation.span(
        "spike_classifier",
        &format!("spike-{}-{}", total_points, rand_id()),
        parent_id,
    );
    span = span.with_attr("total_points", &total_points.to_string());
    span
}

fn rand_id() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_start_creates_span() {
        let span = AnalysisSpan::start("test", "id-1", None);
        assert_eq!(span.operation, "test");
        assert_eq!(span.id, "id-1");
        assert!(span.end.is_none());
    }

    #[test]
    fn span_with_attr_adds_attribute() {
        let span = AnalysisSpan::start("op", "id", None)
            .with_attr("key", "value");
        assert_eq!(span.attributes.len(), 1);
        assert_eq!(span.attributes[0].0, "key");
    }

    #[test]
    fn finish_records_end_time() {
        let mut span = AnalysisSpan::start("op", "id", None);
        let dur = span.finish();
        assert!(span.end.is_some());
        assert!(dur >= Duration::ZERO);
    }

    #[test]
    fn elapsed_returns_duration() {
        let span = AnalysisSpan::start("op", "id", None);
        assert!(span.elapsed() >= Duration::ZERO);
    }

    #[test]
    fn json_includes_id_and_op() {
        let mut span = AnalysisSpan::start("op", "my-id", None);
        span.finish();
        let json = span.to_json();
        assert!(json.contains("my-id"));
        assert!(json.contains("op"));
    }

    #[test]
    fn instrumentation_creates_and_records_spans() {
        let mut inst = AnalysisInstrumentation::new(true);
        let mut s = inst.span("op", "id", None);
        inst.record(&mut s);
        assert_eq!(inst.completed_spans().len(), 1);
    }

    #[test]
    fn instrumentation_disabled_still_creates_spans() {
        let mut inst = AnalysisInstrumentation::new(false);
        let mut s = inst.span("op", "id", None);
        inst.record(&mut s);
        assert_eq!(inst.span_count(), 1);
    }

    #[test]
    fn by_operation_filters() {
        let mut inst = AnalysisInstrumentation::new(true);
        inst.span("a", "1", None);
        inst.span("b", "2", None);
        assert_eq!(inst.by_operation("a").len(), 1);
    }

    #[test]
    fn total_time_accumulates() {
        let mut inst = AnalysisInstrumentation::new(true);
        let mut s1 = inst.span("op", "1", None);
        let mut s2 = inst.span("op", "2", None);
        inst.record(&mut s1);
        inst.record(&mut s2);
        assert!(inst.total_time() > Duration::ZERO);
    }

    #[test]
    fn reset_clears_all() {
        let mut inst = AnalysisInstrumentation::new(true);
        inst.span("op", "1", None);
        inst.reset();
        assert_eq!(inst.span_count(), 0);
    }

    #[test]
    fn to_json_returns_array() {
        let mut inst = AnalysisInstrumentation::new(true);
        let mut s = inst.span("op", "id", None);
        inst.record(&mut s);
        let json = inst.to_json();
        assert!(json.starts_with('['));
    }

    #[test]
    fn instrument_percentile_creates_span() {
        let mut inst = AnalysisInstrumentation::new(true);
        let s = instrument_percentile(&mut inst, 100);
        assert_eq!(s.operation, "percentile");
    }

    #[test]
    fn instrument_spike_classifier_with_attrs() {
        let mut inst = AnalysisInstrumentation::new(true);
        let s = instrument_spike_classifier(&mut inst, 500, None);
        assert!(s.attributes.iter().any(|(k, _)| k == "total_points"));
    }
}
