use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to control whether output should be printed
static SILENT: AtomicBool = AtomicBool::new(false);

/// Set the silent mode globally
pub fn set_silent(silent: bool) {
    SILENT.store(silent, Ordering::Relaxed);
}

/// Check if silent mode is enabled
pub fn is_silent() -> bool {
    SILENT.load(Ordering::Relaxed)
}

/// Print to stdout only if not in silent mode
#[macro_export]
macro_rules! oprintln {
    () => {
        if !$crate::output::is_silent() {
            println!();
        }
    };
    ($($arg:tt)*) => {
        if !$crate::output::is_silent() {
            println!($($arg)*);
        }
    };
}

/// Print to stdout only if not in silent mode (no newline)
#[macro_export]
macro_rules! oprint {
    ($($arg:tt)*) => {
        if !$crate::output::is_silent() {
            print!($($arg)*);
        }
    };
}
