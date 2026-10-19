//! Protocol-independent contracts shared by Viloja's sharing backends.
//!
//! This module deliberately contains no simulated backend. A concrete adapter must
//! report real devices and real transfer events before the UI can present them.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Stable identifier for a backend implementation, not a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BackendId(pub String);

/// Stable identifier scoped to the backend that discovered the device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

/// Identifier for a transfer, scoped to the backend that created it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    pub backend: BackendId,
    pub name: String,
    pub device_type: DeviceType,
    pub paired: bool,
    pub reachable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Phone,
    Computer,
    Tablet,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackendCapabilities {
    pub discovery: bool,
    pub send_files: bool,
    pub receive_files: bool,
    pub cancel_outgoing: bool,
    pub cancel_incoming: bool,
    pub pairing: bool,
    /// True only if the backend can substantiate this for its current transport.
    pub router_free_transfer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendState {
    Unavailable { reason: String },
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSnapshot {
    pub id: BackendId,
    pub display_name: String,
    pub state: BackendState,
    pub capabilities: BackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendRequest {
    pub device_id: DeviceId,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingTransfer {
    pub id: TransferId,
    pub device: Device,
    pub file_names: Vec<String>,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferState {
    AwaitingApproval,
    Queued,
    InProgress,
    Completed,
    Rejected,
    Cancelled,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSnapshot {
    pub id: TransferId,
    pub backend: BackendId,
    pub device: Device,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
}

impl TransferSnapshot {
    /// Returns a bounded percentage when a meaningful, non-zero total is known.
    /// Unknown totals and zero-byte transfers have no percentage.
    pub fn progress_percent(&self) -> Option<u8> {
        let total = self.total_bytes.filter(|total| *total > 0)?;
        let ratio = (self.bytes_transferred.min(total) as f64 / total as f64) * 100.0;
        Some(ratio.round().clamp(0.0, 100.0) as u8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    StateChanged(BackendState),
    DeviceDiscovered(Device),
    DeviceUpdated(Device),
    DeviceLost(DeviceId),
    IncomingTransfer(IncomingTransfer),
    TransferUpdated(TransferSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub backend: BackendId,
    pub message: String,
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.backend.0, self.message)
    }
}

impl std::error::Error for BackendError {}

/// Contract implemented by each real sharing integration.
///
/// Adapters own protocol details and may run their networking on background
/// tasks. They report lifecycle, device and transfer changes through the event
/// sender. The UI must never infer success merely because a request was accepted.
pub trait SharingBackend {
    fn id(&self) -> BackendId;
    fn snapshot(&self) -> BackendSnapshot;
    fn start(&mut self, events: Sender<BackendEvent>) -> Result<(), BackendError>;
    fn stop(&mut self) -> Result<(), BackendError>;
    fn send_files(&mut self, request: SendRequest) -> Result<TransferId, BackendError>;
    fn decide_incoming(
        &mut self,
        transfer_id: &TransferId,
        decision: IncomingDecision,
    ) -> Result<(), BackendError>;
    fn cancel_transfer(&mut self, transfer_id: &TransferId) -> Result<(), BackendError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transfer(bytes_transferred: u64, total_bytes: Option<u64>) -> TransferSnapshot {
        TransferSnapshot {
            id: TransferId("test-transfer".into()),
            backend: BackendId("test-backend".into()),
            device: Device {
                id: DeviceId("test-device".into()),
                backend: BackendId("test-backend".into()),
                name: "Test device".into(),
                device_type: DeviceType::Phone,
                paired: true,
                reachable: true,
            },
            direction: TransferDirection::Outgoing,
            state: TransferState::InProgress,
            bytes_transferred,
            total_bytes,
        }
    }

    #[test]
    fn progress_is_reported_for_known_nonzero_totals() {
        assert_eq!(transfer(0, Some(100)).progress_percent(), Some(0));
        assert_eq!(transfer(37, Some(100)).progress_percent(), Some(37));
        assert_eq!(transfer(100, Some(100)).progress_percent(), Some(100));
    }

    #[test]
    fn progress_never_exceeds_one_hundred_percent() {
        assert_eq!(transfer(120, Some(100)).progress_percent(), Some(100));
    }

    #[test]
    fn progress_is_unknown_for_missing_or_zero_total() {
        assert_eq!(transfer(10, None).progress_percent(), None);
        assert_eq!(transfer(0, Some(0)).progress_percent(), None);
    }
}
