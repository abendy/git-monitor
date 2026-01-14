//! Transient notification toasts.

use std::time::{Duration, Instant};

/// How long toasts are displayed before auto-dismissing
const TOAST_DURATION: Duration = Duration::from_secs(3);

/// Toast severity/type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    /// Informational (blue)
    Info,
    /// Success (green)
    Success,
    /// Warning (yellow)
    Warning,
    /// Error (red)
    Error,
}

/// A transient notification that auto-dismisses
#[derive(Debug, Clone)]
pub struct Toast {
    /// Message to display
    pub message: String,
    /// Severity/type
    pub level: ToastLevel,
    /// When the toast was created
    created: Instant,
}

impl Toast {
    /// Create a new toast
    #[must_use]
    pub fn new(message: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: message.into(),
            level,
            created: Instant::now(),
        }
    }

    /// Create an info toast
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Info)
    }

    /// Create a success toast
    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Success)
    }

    /// Create a warning toast
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Warning)
    }

    /// Create an error toast
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Error)
    }

    /// Check if the toast has expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.created.elapsed() >= TOAST_DURATION
    }

    /// Get remaining time as a fraction (0.0 to 1.0)
    #[must_use]
    pub fn remaining_fraction(&self) -> f32 {
        let elapsed = self.created.elapsed();
        if elapsed >= TOAST_DURATION {
            0.0
        } else {
            1.0 - (elapsed.as_secs_f32() / TOAST_DURATION.as_secs_f32())
        }
    }
}
