//! Privacy-preserving, bounded diagnostic event log.
//!
//! Entries are assembled from fixed event variants; callers cannot attach free-form
//! device names, addresses, tokens, paths, or backend error strings.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 500;
static VERBOSITY: AtomicU8 = AtomicU8::new(1);
static ENTRIES: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error = 0,
    Info = 1,
    Debug = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticEvent {
    BackendRefreshStarted,
    SessionBusUnavailable,
    BackendUnavailable,
    DiscoveryCompleted { device_count: usize },
    BackendDiscoveryFailed,
}

impl DiagnosticEvent {
    fn fields(self) -> String {
        match self {
            Self::BackendRefreshStarted => "\"event\":\"backend.refresh_started\"".into(),
            Self::SessionBusUnavailable => "\"event\":\"session_bus.unavailable\"".into(),
            Self::BackendUnavailable => "\"event\":\"backend.unavailable\"".into(),
            Self::DiscoveryCompleted { device_count } => format!(
                "\"event\":\"backend.discovery_completed\",\"device_count\":{device_count}"
            ),
            Self::BackendDiscoveryFailed => "\"event\":\"backend.discovery_failed\"".into(),
        }
    }

    fn level(self) -> DiagnosticLevel {
        match self {
            Self::SessionBusUnavailable | Self::BackendUnavailable | Self::BackendDiscoveryFailed => DiagnosticLevel::Error,
            Self::BackendRefreshStarted | Self::DiscoveryCompleted { .. } => DiagnosticLevel::Info,
        }
    }
}

fn entries() -> &'static Mutex<VecDeque<String>> {
    ENTRIES.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_ENTRIES)))
}

pub fn set_verbosity(index: u32) {
    VERBOSITY.store(index.min(3) as u8, Ordering::Relaxed);
}

pub fn verbosity_index() -> u32 {
    VERBOSITY.load(Ordering::Relaxed) as u32
}

pub fn record(level: DiagnosticLevel, event: DiagnosticEvent) {
    let verbosity = VERBOSITY.load(Ordering::Relaxed);
    if verbosity == 3 || level as u8 > verbosity || level != event.level() {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let line = format!(
        "{{\"timestamp_unix\":{timestamp},\"level\":\"{}\",{}}}",
        match level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Info => "info",
            DiagnosticLevel::Debug => "debug",
        },
        event.fields()
    );

    if let Ok(mut entries) = entries().lock() {
        if entries.len() == MAX_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(line);
    }
}

pub fn export() -> String {
    entries()
        .lock()
        .map(|entries| entries.iter().cloned().collect::<Vec<_>>().join("\n"))
        .unwrap_or_else(|_| String::from("Diagnostic log is temporarily unavailable."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_events_are_structured_and_exclude_free_form_identifiers() {
        let fields = DiagnosticEvent::DiscoveryCompleted { device_count: 2 }.fields();
        assert!(fields.contains("backend.discovery_completed"));
        assert!(fields.contains("device_count"));
        assert!(!fields.contains("device name"));
        assert!(!fields.contains("/home/"));
        assert!(!fields.contains("@"));
    }

    #[test]
    fn verbosity_levels_are_bounded() {
        set_verbosity(99);
        assert_eq!(verbosity_index(), 3);
        set_verbosity(1);
        assert_eq!(verbosity_index(), 1);
        set_verbosity(0);
    }

    #[test]
    fn log_export_is_json_lines() {
        let previous = verbosity_index();
        set_verbosity(1);
        record(DiagnosticLevel::Info, DiagnosticEvent::BackendRefreshStarted);
        let exported = export();
        assert!(exported.lines().all(|line| line.starts_with("{\"timestamp_unix\"")));
        set_verbosity(previous);
    }
}
