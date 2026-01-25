//! MCP Log Layer for capturing application logs
//!
//! This module provides a tracing-subscriber Layer that captures log entries
//! into a buffer for retrieval via MCP tools.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use egui_mcp_client::McpLogLayer;
//! use tracing_subscriber::prelude::*;
//!
//! fn main() {
//!     let (mcp_layer, log_buffer) = McpLogLayer::new(1000);
//!
//!     tracing_subscriber::registry()
//!         .with(mcp_layer)
//!         .with(tracing_subscriber::fmt::layer())
//!         .init();
//!
//!     let mcp_client = McpClient::new().with_log_buffer(log_buffer);
//!     // ... run egui app
//! }
//! ```

use egui_mcp_protocol::LogEntry;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Shared log buffer type
pub type LogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;

/// A tracing Layer that captures log entries for MCP access
pub struct McpLogLayer {
    buffer: LogBuffer,
    max_entries: usize,
}

impl McpLogLayer {
    /// Create a new MCP log layer with the specified maximum number of entries
    ///
    /// Returns the layer and a shared reference to the log buffer that can be
    /// passed to `McpClient::with_log_buffer()`.
    pub fn new(max_entries: usize) -> (Self, LogBuffer) {
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(max_entries)));
        let layer = Self {
            buffer: buffer.clone(),
            max_entries,
        };
        (layer, buffer)
    }

    /// Get a reference to the log buffer
    pub fn buffer(&self) -> LogBuffer {
        self.buffer.clone()
    }
}

/// Visitor to extract the message field from a tracing event
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove surrounding quotes if present
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

impl<S> Layer<S> for McpLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        // If no message field, try to get any field content
        if visitor.message.is_empty() {
            let mut any_visitor = AnyFieldVisitor::default();
            event.record(&mut any_visitor);
            visitor.message = any_visitor.fields.join(", ");
        }

        let entry = LogEntry {
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_string(),
            message: visitor.message,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };

        let mut buf = self.buffer.lock();
        buf.push_back(entry);
        while buf.len() > self.max_entries {
            buf.pop_front();
        }
    }
}

/// Visitor to capture any field content when no message field is present
#[derive(Default)]
struct AnyFieldVisitor {
    fields: Vec<String>,
}

impl Visit for AnyFieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={:?}", field.name(), value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.push(format!("{}={}", field.name(), value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.push(format!("{}={}", field.name(), value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.push(format!("{}={}", field.name(), value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.push(format!("{}={}", field.name(), value));
    }
}

/// Helper functions for filtering logs by level
pub fn level_to_priority(level: &str) -> u8 {
    match level.to_uppercase().as_str() {
        "ERROR" => 5,
        "WARN" => 4,
        "INFO" => 3,
        "DEBUG" => 2,
        "TRACE" => 1,
        _ => 0,
    }
}
