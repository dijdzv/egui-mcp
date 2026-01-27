//! Error types for the MCP server
//!
//! This module provides structured error types for better error handling
//! and more informative error messages.

use atspi::zbus;
use thiserror::Error;

/// Errors that can occur during AT-SPI operations
#[derive(Debug, Error)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum AtspiError {
    /// Failed to connect to AT-SPI
    #[error("Failed to connect to AT-SPI: {0}")]
    Connection(#[source] atspi::AtspiError),

    /// Application not found
    #[error("Application '{app_name}' not found via AT-SPI")]
    AppNotFound { app_name: String },

    /// Element not found
    #[error("Element with id {id} not found in application '{app_name}'")]
    ElementNotFound { id: u64, app_name: String },

    /// AT-SPI interface not available on the element
    #[error("AT-SPI {interface} interface not available on element {id}")]
    InterfaceNotAvailable { interface: &'static str, id: u64 },

    /// D-Bus communication error
    #[error("D-Bus error: {0}")]
    DBus(#[from] zbus::Error),

    /// AT-SPI library error
    #[error("AT-SPI error: {0}")]
    Atspi(#[from] atspi::AtspiError),
}

impl AtspiError {
    /// Create an ElementNotFound error
    pub fn element_not_found(id: u64, app_name: impl Into<String>) -> Self {
        Self::ElementNotFound {
            id,
            app_name: app_name.into(),
        }
    }

    /// Create an InterfaceNotAvailable error (for future use)
    #[allow(dead_code)]
    pub fn interface_not_available(interface: &'static str, id: u64) -> Self {
        Self::InterfaceNotAvailable { interface, id }
    }
}
