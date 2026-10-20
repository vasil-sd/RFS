//! Minimal stderr logging shared by the new CLI commands (Phase 5 task 3).
//!
//! `--log-level off|error|warn|info|debug` selects the highest level that is
//! printed to STDERR. Command FATAL diagnostics (parse/validation/codegen
//! errors that drive exit code 1) are NOT log lines: they bypass this logger
//! and always go to stderr, exactly like the pre-Phase-5 commands do. The
//! logger exists for informational progress output of the new external-tool
//! commands (`rfs test`, `rfs diagram`, ...).

/// Log level filter (higher = more verbose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

impl LogLevel {
    /// Parse the `--log-level` value (case-sensitive lowercase keywords).
    pub fn parse(s: &str) -> Option<LogLevel> {
        match s {
            "off" => Some(LogLevel::Off),
            "error" => Some(LogLevel::Error),
            "warn" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            _ => None,
        }
    }
}

/// A stderr logger with a level filter.
#[derive(Debug, Clone)]
pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }

    /// The configured level (tests / diagnostics).
    pub fn level(&self) -> LogLevel {
        self.level
    }

    fn log(&self, min: LogLevel, prefix: &str, msg: &str) {
        if self.level >= min {
            eprintln!("{prefix}{msg}");
        }
    }

    /// `error: ...` (printed when level >= Error).
    pub fn error(&self, msg: &str) {
        self.log(LogLevel::Error, "error: ", msg);
    }

    /// `warning: ...` (printed when level >= Warn).
    pub fn warn(&self, msg: &str) {
        self.log(LogLevel::Warn, "warning: ", msg);
    }

    /// `info: ...` (printed when level >= Info).
    pub fn info(&self, msg: &str) {
        self.log(LogLevel::Info, "info: ", msg);
    }

    /// `debug: ...` (printed when level >= Debug).
    pub fn debug(&self, msg: &str) {
        self.log(LogLevel::Debug, "debug: ", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parsing() {
        assert_eq!(LogLevel::parse("off"), Some(LogLevel::Off));
        assert_eq!(LogLevel::parse("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse("warn"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::parse("verbose"), None);
        assert_eq!(LogLevel::parse("OFF"), None);
    }

    #[test]
    fn level_ordering() {
        assert!(LogLevel::Off < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
    }
}
