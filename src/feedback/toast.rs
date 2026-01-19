//! Transient notification toasts.

use std::time::{Duration, Instant};

/// How long toasts are displayed before auto-dismissing
const TOAST_DURATION: Duration = Duration::from_secs(3);

/// Toast severity/type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants for toast styling
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
    #[allow(dead_code)] // Convenience constructor
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Info)
    }

    /// Create a success toast
    #[must_use]
    #[allow(dead_code)] // Convenience constructor
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Success)
    }

    /// Create a warning toast
    #[must_use]
    #[allow(dead_code)] // Convenience constructor
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, ToastLevel::Warning)
    }

    /// Create an error toast
    #[must_use]
    #[allow(dead_code)] // Convenience constructor
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
    #[allow(dead_code)] // For animated progress bar
    pub fn remaining_fraction(&self) -> f32 {
        let elapsed = self.created.elapsed();
        if elapsed >= TOAST_DURATION {
            0.0
        } else {
            1.0 - (elapsed.as_secs_f32() / TOAST_DURATION.as_secs_f32())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod toast {
        use super::*;

        #[test]
        fn new_creates_toast_with_level() {
            let toast = Toast::new("Test message", ToastLevel::Info);

            assert_eq!(toast.message, "Test message");
            assert_eq!(toast.level, ToastLevel::Info);
        }

        #[test]
        fn info_creates_info_toast() {
            let toast = Toast::info("Info message");

            assert_eq!(toast.message, "Info message");
            assert_eq!(toast.level, ToastLevel::Info);
        }

        #[test]
        fn success_creates_success_toast() {
            let toast = Toast::success("Success message");

            assert_eq!(toast.message, "Success message");
            assert_eq!(toast.level, ToastLevel::Success);
        }

        #[test]
        fn warning_creates_warning_toast() {
            let toast = Toast::warning("Warning message");

            assert_eq!(toast.message, "Warning message");
            assert_eq!(toast.level, ToastLevel::Warning);
        }

        #[test]
        fn error_creates_error_toast() {
            let toast = Toast::error("Error message");

            assert_eq!(toast.message, "Error message");
            assert_eq!(toast.level, ToastLevel::Error);
        }

        #[test]
        fn new_toast_is_not_expired() {
            let toast = Toast::new("Test", ToastLevel::Info);

            assert!(!toast.is_expired());
        }

        #[test]
        fn new_toast_has_high_remaining_fraction() {
            let toast = Toast::new("Test", ToastLevel::Info);

            // Should be close to 1.0 (just created)
            assert!(toast.remaining_fraction() > 0.9);
        }
    }

    mod toast_level {
        use super::*;

        #[test]
        fn levels_are_distinct() {
            assert_ne!(ToastLevel::Info, ToastLevel::Success);
            assert_ne!(ToastLevel::Success, ToastLevel::Warning);
            assert_ne!(ToastLevel::Warning, ToastLevel::Error);
            assert_ne!(ToastLevel::Error, ToastLevel::Info);
        }
    }
}
