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
    RefreshSuperseded,
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
            Self::RefreshSuperseded => "\"event\":\"backend.refresh_superseded\"".into(),
        }
    }

    fn level(self) -> DiagnosticLevel {
        match self {
            Self::SessionBusUnavailable | Self::BackendUnavailable | Self::BackendDiscoveryFailed => DiagnosticLevel::Error,
            Self::BackendRefreshStarted | Self::DiscoveryCompleted { .. } => DiagnosticLevel::Info,
            Self::RefreshSuperseded => DiagnosticLevel::Debug,
        }
    }
}

fn entries() -> &'static Mutex<VecDeque<String>> {
    ENTRIES.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_ENTRIES)))
}

fn should_record(verbosity: u8, level: DiagnosticLevel, event: DiagnosticEvent) -> bool {
    verbosity != 3 && level as u8 <= verbosity && level == event.level()
}

fn format_entry(timestamp: u64, level: DiagnosticLevel, event: DiagnosticEvent) -> String {
    format!(
        "{{\"timestamp_unix\":{timestamp},\"level\":\"{}\",{}}}",
        match level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Info => "info",
            DiagnosticLevel::Debug => "debug",
        },
        event.fields()
    )
}

fn append_entry(entries: &mut VecDeque<String>, line: String) {
    while entries.len() >= MAX_ENTRIES {
        entries.pop_front();
    }
    entries.push_back(line);
}

fn clear_entries(entries: &mut VecDeque<String>) {
    entries.clear();
}

fn normalize_verbosity(index: u32) -> u8 {
    index.min(3) as u8
}

pub fn set_verbosity(index: u32) {
    VERBOSITY.store(normalize_verbosity(index), Ordering::Relaxed);
}

pub fn verbosity_index() -> u32 {
    VERBOSITY.load(Ordering::Relaxed) as u32
}

pub fn record(level: DiagnosticLevel, event: DiagnosticEvent) {
    let verbosity = VERBOSITY.load(Ordering::Relaxed);
    if !should_record(verbosity, level, event) {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let line = format_entry(timestamp, level, event);

    if let Ok(mut entries) = entries().lock() {
        append_entry(&mut entries, line);
    }
}

pub fn clear() {
    if let Ok(mut entries) = entries().lock() {
        clear_entries(&mut entries);
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
        assert_eq!(normalize_verbosity(99), 3);
        assert_eq!(normalize_verbosity(1), 1);
        assert_eq!(normalize_verbosity(0), 0);
    }

    #[test]
    fn refresh_superseded_is_a_debug_only_event() {
        let event = DiagnosticEvent::RefreshSuperseded;
        assert_eq!(event.level(), DiagnosticLevel::Debug);
        assert!(event.fields().contains("backend.refresh_superseded"));
        assert!(!should_record(1, DiagnosticLevel::Debug, event));
        assert!(should_record(2, DiagnosticLevel::Debug, event));
        assert!(!should_record(3, DiagnosticLevel::Debug, event));
    }

    #[test]
    fn log_export_is_json_lines() {
        let first = format_entry(1, DiagnosticLevel::Info, DiagnosticEvent::BackendRefreshStarted);
        let second = format_entry(2, DiagnosticLevel::Error, DiagnosticEvent::BackendUnavailable);
        let exported = [first, second].join("\n");
        assert_eq!(exported.lines().count(), 2);
        assert!(exported.lines().all(|line| line.starts_with("{\"timestamp_unix\"")));
        assert!(exported.lines().all(|line| line.ends_with('}')));
    }

    #[test]
    fn bounded_log_discards_the_oldest_entry() {
        let mut log = (0..MAX_ENTRIES).map(|n| n.to_string()).collect::<VecDeque<_>>();
        append_entry(&mut log, "newest".into());
        assert_eq!(log.len(), MAX_ENTRIES);
        assert_eq!(log.front().map(String::as_str), Some("1"));
        assert_eq!(log.back().map(String::as_str), Some("newest"));
    }

    #[test]
    fn append_restores_the_bound_even_if_log_starts_oversized() {
        let mut log = (0..(MAX_ENTRIES + 2))
            .map(|n| n.to_string())
            .collect::<VecDeque<_>>();
        append_entry(&mut log, "newest".into());
        assert_eq!(log.len(), MAX_ENTRIES);
        assert_eq!(log.front().map(String::as_str), Some("3"));
        assert_eq!(log.back().map(String::as_str), Some("newest"));
    }

    #[test]
    fn clearing_diagnostics_removes_all_entries() {
        let mut log = VecDeque::from(["first".to_string(), "second".to_string()]);
        clear_entries(&mut log);
        assert!(log.is_empty());
    }
}
