use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// A unique trace identifier.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TraceId(pub String);

/// A span identifier within a trace.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SpanId(pub String);

/// Context propagated across module boundaries during a trace.
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// Unique trace identifier.
    pub trace_id: TraceId,
    /// Current span identifier.
    pub span_id: SpanId,
    /// Parent span identifier (empty for root spans).
    pub parent_span_id: SpanId,
    /// Key-value baggage carried across module boundaries.
    pub baggage: HashMap<String, String>,
    /// Timestamp when the trace started (Unix nanos).
    pub start_ns: u64,
}

impl TraceContext {
    /// Create a new root trace context.
    pub fn new_root() -> Self {
        let id = Self::generate_id();
        Self {
            trace_id: TraceId(id.clone()),
            span_id: SpanId(id),
            parent_span_id: SpanId(String::new()),
            baggage: HashMap::new(),
            start_ns: now_nanos(),
        }
    }

    /// Create a child span context from this context.
    pub fn child_span(&self) -> Self {
        let child_id = Self::generate_id();
        Self {
            trace_id: self.trace_id.clone(),
            span_id: SpanId(child_id),
            parent_span_id: self.span_id.clone(),
            baggage: self.baggage.clone(),
            start_ns: now_nanos(),
        }
    }

    /// Add a baggage item to the trace context.
    pub fn with_baggage(mut self, key: &str, value: &str) -> Self {
        self.baggage.insert(key.to_string(), value.to_string());
        self
    }

    /// Get a baggage value by key.
    pub fn baggage(&self, key: &str) -> Option<&str> {
        self.baggage.get(key).map(|s| s.as_str())
    }

    /// Elapsed time in milliseconds since the trace started.
    pub fn elapsed_ms(&self) -> u64 {
        now_nanos().saturating_sub(self.start_ns) / 1_000_000
    }

    /// Serialize the trace context to a W3C traceparent-compatible header.
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id.0, self.span_id.0)
    }

    /// Parse a W3C traceparent header.
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() < 4 || parts[0] != "00" {
            return None;
        }
        Some(Self {
            trace_id: TraceId(parts[1].to_string()),
            span_id: SpanId(parts[2].to_string()),
            parent_span_id: SpanId(String::new()),
            baggage: HashMap::new(),
            start_ns: now_nanos(),
        })
    }

    fn generate_id() -> String {
        let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        format!("{:032x}", n)
    }
}

/// A registry that maintains the current trace context per thread/scope.
pub struct TraceRegistry {
    current: Mutex<Option<TraceContext>>,
    spans_created: AtomicU64,
}

impl TraceRegistry {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
            spans_created: AtomicU64::new(0),
        }
    }

    /// Set the current trace context.
    pub fn set(&self, ctx: TraceContext) {
        *self.current.lock().unwrap() = Some(ctx);
    }

    /// Get the current trace context, creating a root if none exists.
    pub fn get(&self) -> TraceContext {
        self.current
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                let root = TraceContext::new_root();
                *self.current.lock().unwrap() = Some(root.clone());
                root
            })
    }

    /// Start a new child span under the current trace.
    pub fn start_span(&self) -> TraceContext {
        let child = self.get().child_span();
        self.spans_created.fetch_add(1, Ordering::Relaxed);
        child
    }

    /// Clear the current trace context.
    pub fn clear(&self) {
        *self.current.lock().unwrap() = None;
    }

    /// Number of spans created by this registry.
    pub fn span_count(&self) -> u64 {
        self.spans_created.load(Ordering::Relaxed)
    }
}

fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Propagate trace context between two modules via a carrier map.
pub trait Propagator {
    /// Inject the trace context into a carrier (e.g., HTTP headers, map).
    fn inject(&self, ctx: &TraceContext, carrier: &mut HashMap<String, String>);
    /// Extract the trace context from a carrier.
    fn extract(&self, carrier: &HashMap<String, String>) -> Option<TraceContext>;
}

/// A propagator that uses W3C traceparent format.
pub struct W3CPropagator;

impl Propagator for W3CPropagator {
    fn inject(&self, ctx: &TraceContext, carrier: &mut HashMap<String, String>) {
        carrier.insert("traceparent".into(), ctx.to_traceparent());
        for (k, v) in &ctx.baggage {
            carrier.insert(format!("baggage-{}", k), v.clone());
        }
    }

    fn extract(&self, carrier: &HashMap<String, String>) -> Option<TraceContext> {
        let header = carrier.get("traceparent")?;
        let mut ctx = TraceContext::from_traceparent(header)?;
        for (k, v) in carrier {
            if let Some(key) = k.strip_prefix("baggage-") {
                ctx.baggage.insert(key.to_string(), v.clone());
            }
        }
        Some(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_root_creates_unique_trace_id() {
        let a = TraceContext::new_root();
        let b = TraceContext::new_root();
        assert_ne!(a.trace_id, b.trace_id);
    }

    #[test]
    fn child_span_preserves_trace_id() {
        let root = TraceContext::new_root();
        let child = root.child_span();
        assert_eq!(root.trace_id, child.trace_id);
        assert_eq!(child.parent_span_id.0, root.span_id.0);
    }

    #[test]
    fn baggage_round_trip() {
        let ctx = TraceContext::new_root().with_baggage("env", "test");
        assert_eq!(ctx.baggage("env"), Some("test"));
    }

    #[test]
    fn traceparent_round_trip() {
        let ctx = TraceContext::new_root();
        let header = ctx.to_traceparent();
        let parsed = TraceContext::from_traceparent(&header).unwrap();
        assert_eq!(ctx.trace_id, parsed.trace_id);
        assert_eq!(ctx.span_id, parsed.span_id);
    }

    #[test]
    fn registry_get_creates_root() {
        let reg = TraceRegistry::new();
        let ctx = reg.get();
        assert!(!ctx.trace_id.0.is_empty());
    }

    #[test]
    fn registry_start_span_increments_count() {
        let reg = TraceRegistry::new();
        reg.start_span();
        reg.start_span();
        assert_eq!(reg.span_count(), 2);
    }

    #[test]
    fn w3c_propagator_inject_and_extract() {
        let propagator = W3CPropagator;
        let ctx = TraceContext::new_root().with_baggage("env", "prod");
        let mut carrier = HashMap::new();
        propagator.inject(&ctx, &mut carrier);
        assert!(carrier.contains_key("traceparent"));
        let extracted = propagator.extract(&carrier).unwrap();
        assert_eq!(ctx.trace_id, extracted.trace_id);
        assert_eq!(extracted.baggage("env"), Some("prod"));
    }

    #[test]
    fn registry_clear_removes_context() {
        let reg = TraceRegistry::new();
        reg.set(TraceContext::new_root());
        reg.clear();
        let ctx = reg.get();
        assert!(!ctx.baggage.contains_key("env"));
    }

    #[test]
    fn invalid_traceparent_returns_none() {
        assert!(TraceContext::from_traceparent("invalid").is_none());
        assert!(TraceContext::from_traceparent("00-abc").is_none());
    }
}
