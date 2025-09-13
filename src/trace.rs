use crate::Result;
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{Layer, fmt};

static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);
static PROCESS_START: OnceCell<Instant> = OnceCell::new();
static SPAN_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Initialize the tracing subscriber
pub fn init_tracing(json_output: bool) -> Result<()> {
    use tracing_subscriber::prelude::*;

    TRACE_ENABLED.store(true, Ordering::Relaxed);
    PROCESS_START.set(Instant::now()).ok();

    // Install LogTracer to forward log events to tracing if possible
    // This allows existing log macros to show up as trace events
    if let Err(_) = tracing_log::LogTracer::init() {
        // LogTracer couldn't be installed, probably because a logger is already set
        // This is fine - we just won't capture log events in traces
    }

    // Try to set our subscriber, but handle the case where one is already set
    let result = if json_output {
        // JSON Lines output to stdout
        let json_layer = JsonLayer::new();
        tracing_subscriber::registry().with(json_layer).try_init()
    } else {
        // Pretty console output to stderr with hierarchical spans
        let fmt_layer = fmt::layer()
            .with_target(false)
            .with_writer(std::io::stderr)
            .with_timer(fmt::time::uptime())
            .with_ansi(console::Term::stderr().features().colors_supported())
            .with_thread_ids(false)
            .with_thread_names(false)
            .compact();

        tracing_subscriber::registry().with(fmt_layer).try_init()
    };

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            // A subscriber is already set - this might be from clx or elsewhere
            // Let's check if we can work with the existing subscriber
            let err_str = e.to_string();
            if err_str.contains(
                "attempted to set a logger after the logging system was already initialized",
            ) {
                // This is the common case - another part of the system has set up tracing
                // We can still use tracing, we just can't install our own subscriber
                // For now, just continue - our spans will go to the existing subscriber
                eprintln!(
                    "Note: Tracing subscriber already initialized, using existing subscriber"
                );
                Ok(())
            } else {
                Err(eyre::eyre!(
                    "Failed to initialize tracing subscriber: {}",
                    e
                ))
            }
        }
    }
}

/// JSON Lines layer for tracing output
struct JsonLayer {
    writer: Mutex<std::io::Stdout>,
}

impl JsonLayer {
    fn new() -> Self {
        // Write metadata line
        let mut stdout = std::io::stdout();
        let meta = JsonMeta {
            r#type: "meta",
            span_schema_version: 1,
            hk_version: env!("CARGO_PKG_VERSION"),
            pid: std::process::id(),
        };
        if let Ok(json) = serde_json::to_string(&meta) {
            let _ = writeln!(stdout, "{}", json);
        }

        Self {
            writer: Mutex::new(stdout),
        }
    }

    fn timestamp_ns() -> u64 {
        PROCESS_START
            .get()
            .map(|start| start.elapsed().as_nanos() as u64)
            .unwrap_or(0)
    }

    fn next_span_id() -> String {
        format!("span_{}", SPAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn write_json<T: Serialize>(&self, value: &T) {
        if let Ok(json) = serde_json::to_string(value) {
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writeln!(writer, "{}", json);
                let _ = writer.flush();
            }
        }
    }
}

impl<S> Layer<S> for JsonLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let metadata = attrs.metadata();
        if let Some(span) = ctx.span(id) {
            let span_id = Self::next_span_id();
            let parent_id = attrs.parent().and_then(|pid| {
                ctx.span(pid).and_then(|parent_span| {
                    let ext = parent_span.extensions();
                    ext.get::<SpanData>().map(|d| d.id.clone())
                })
            });

            span.extensions_mut().insert(SpanData {
                id: span_id.clone(),
                parent_id: parent_id.clone(),
                start_ns: Self::timestamp_ns(),
            });

            let mut visitor = JsonVisitor::default();
            attrs.record(&mut visitor);

            let event = JsonSpanStart {
                r#type: "span_start",
                ts_ns: Self::timestamp_ns(),
                id: span_id,
                name: metadata.name(),
                attrs: visitor.fields,
                parent_id,
            };
            self.write_json(&event);
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(&id) {
            if let Some(data) = span.extensions().get::<SpanData>() {
                let event = JsonSpanEnd {
                    r#type: "span_end",
                    ts_ns: Self::timestamp_ns(),
                    id: data.id.clone(),
                };
                self.write_json(&event);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = JsonVisitor::default();
        event.record(&mut visitor);

        let parent_id = ctx.current_span().id().and_then(|id| {
            ctx.span(id).and_then(|s| {
                let ext = s.extensions();
                ext.get::<SpanData>().map(|d| d.id.clone())
            })
        });

        let json_event = JsonInstant {
            r#type: "instant",
            ts_ns: Self::timestamp_ns(),
            name: metadata.name(),
            attrs: visitor.fields,
            parent_id,
        };
        self.write_json(&json_event);
    }
}

// Data structures for JSON output
#[derive(Serialize)]
struct JsonMeta {
    r#type: &'static str,
    span_schema_version: u32,
    hk_version: &'static str,
    pid: u32,
}

#[derive(Serialize)]
struct JsonSpanStart {
    r#type: &'static str,
    ts_ns: u64,
    id: String,
    name: &'static str,
    attrs: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
}

#[derive(Serialize)]
struct JsonSpanEnd {
    r#type: &'static str,
    ts_ns: u64,
    id: String,
}

#[derive(Serialize)]
struct JsonInstant {
    r#type: &'static str,
    ts_ns: u64,
    name: &'static str,
    attrs: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
}

// Internal span data stored in extensions
struct SpanData {
    id: String,
    parent_id: Option<String>,
    start_ns: u64,
}

// Visitor to collect fields from spans/events
#[derive(Default)]
struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl tracing::field::Visit for JsonVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if let Some(num) = serde_json::Number::from_f64(value as f64) {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::Number(num));
        }
    }
}
