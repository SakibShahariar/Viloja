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

    use std::collections::HashMap;
    use std::sync::mpsc::{self, Receiver};

    struct ContractFakeBackend {
        state: BackendState,
        events: Option<Sender<BackendEvent>>,
        device: Device,
        transfers: HashMap<TransferId, TransferSnapshot>,
        next_transfer_id: u64,
        available: bool,
    }

    impl ContractFakeBackend {
        fn new() -> Self {
            Self {
                state: BackendState::Stopped,
                events: None,
                device: Device {
                    id: DeviceId("device-1".into()),
                    backend: BackendId("fake".into()),
                    name: "Test phone".into(),
                    device_type: DeviceType::Phone,
                    paired: true,
                    reachable: true,
                },
                transfers: HashMap::new(),
                next_transfer_id: 1,
                available: true,
            }
        }

        fn publish(&self, event: BackendEvent) {
            if let Some(events) = &self.events {
                let _ = events.send(event);
            }
        }

        fn request_incoming(&mut self) -> TransferId {
            let id = TransferId(format!("incoming-{}", self.next_transfer_id));
            self.next_transfer_id += 1;
            let incoming = IncomingTransfer {
                id: id.clone(),
                device: self.device.clone(),
                file_names: vec!["photo.jpg".into()],
                total_bytes: Some(100),
            };
            self.transfers.insert(id.clone(), TransferSnapshot {
                id: id.clone(),
                backend: self.id(),
                device: self.device.clone(),
                direction: TransferDirection::Incoming,
                state: TransferState::AwaitingApproval,
                bytes_transferred: 0,
                total_bytes: Some(100),
            });
            self.publish(BackendEvent::IncomingTransfer(incoming));
            id
        }

        fn complete_transfer(&mut self, id: &TransferId) {
            let updated = self.transfers.get_mut(id).map(|transfer| {
                transfer.state = TransferState::Completed;
                transfer.bytes_transferred = transfer.total_bytes.unwrap_or(0);
                transfer.clone()
            });
            if let Some(transfer) = updated {
                self.publish(BackendEvent::TransferUpdated(transfer));
            }
        }

        fn disconnect_device(&mut self) {
            self.device.reachable = false;
            self.publish(BackendEvent::DeviceLost(self.device.id.clone()));
        }

        fn start_with_receiver(&mut self) -> Receiver<BackendEvent> {
            let (sender, receiver) = mpsc::channel();
            self.start(sender).expect("fake backend should start");
            receiver
        }
    }

    impl SharingBackend for ContractFakeBackend {
        fn id(&self) -> BackendId {
            BackendId("fake".into())
        }

        fn snapshot(&self) -> BackendSnapshot {
            BackendSnapshot {
                id: self.id(),
                display_name: "Contract fake".into(),
                state: self.state.clone(),
                capabilities: BackendCapabilities {
                    discovery: true,
                    send_files: true,
                    receive_files: true,
                    cancel_outgoing: true,
                    cancel_incoming: false,
                    pairing: true,
                    router_free_transfer: false,
                },
            }
        }

        fn start(&mut self, events: Sender<BackendEvent>) -> Result<(), BackendError> {
            self.events = Some(events);
            if !self.available {
                self.state = BackendState::Unavailable {
                    reason: "test service unavailable".into(),
                };
                self.publish(BackendEvent::StateChanged(self.state.clone()));
                return Err(BackendError {
                    backend: self.id(),
                    message: "test service unavailable".into(),
                });
            }
            self.state = BackendState::Running;
            self.publish(BackendEvent::StateChanged(self.state.clone()));
            self.publish(BackendEvent::DeviceDiscovered(self.device.clone()));
            Ok(())
        }

        fn stop(&mut self) -> Result<(), BackendError> {
            self.state = BackendState::Stopped;
            self.publish(BackendEvent::StateChanged(self.state.clone()));
            self.events = None;
            Ok(())
        }

        fn send_files(&mut self, request: SendRequest) -> Result<TransferId, BackendError> {
            if self.state != BackendState::Running {
                return Err(BackendError {
                    backend: self.id(),
                    message: "backend is not running".into(),
                });
            }
            if request.device_id != self.device.id || request.files.is_empty() {
                return Err(BackendError {
                    backend: self.id(),
                    message: "invalid destination or empty file selection".into(),
                });
            }
            let id = TransferId(format!("outgoing-{}", self.next_transfer_id));
            self.next_transfer_id += 1;
            let transfer = TransferSnapshot {
                id: id.clone(),
                backend: self.id(),
                device: self.device.clone(),
                direction: TransferDirection::Outgoing,
                state: TransferState::Queued,
                bytes_transferred: 0,
                total_bytes: Some(100),
            };
            self.transfers.insert(id.clone(), transfer.clone());
            self.publish(BackendEvent::TransferUpdated(transfer));
            Ok(id)
        }

        fn decide_incoming(
            &mut self,
            transfer_id: &TransferId,
            decision: IncomingDecision,
        ) -> Result<(), BackendError> {
            let backend_id = self.id();
            let updated = {
                let transfer = self.transfers.get_mut(transfer_id).ok_or_else(|| BackendError {
                    backend: backend_id.clone(),
                    message: "unknown transfer".into(),
                })?;
                if transfer.direction != TransferDirection::Incoming
                    || transfer.state != TransferState::AwaitingApproval
                {
                    return Err(BackendError {
                        backend: backend_id,
                        message: "incoming transfer is not awaiting approval".into(),
                    });
                }
                transfer.state = match decision {
                    IncomingDecision::Accept => TransferState::InProgress,
                    IncomingDecision::Reject => TransferState::Rejected,
                };
                transfer.clone()
            };
            self.publish(BackendEvent::TransferUpdated(updated));
            Ok(())
        }

        fn cancel_transfer(&mut self, transfer_id: &TransferId) -> Result<(), BackendError> {
            let backend_id = self.id();
            let updated = {
                let transfer = self.transfers.get_mut(transfer_id).ok_or_else(|| BackendError {
                    backend: backend_id.clone(),
                    message: "unknown transfer".into(),
                })?;
                if !matches!(transfer.state, TransferState::Queued | TransferState::InProgress) {
                    return Err(BackendError {
                        backend: backend_id,
                        message: "transfer is not cancellable".into(),
                    });
                }
                transfer.state = TransferState::Cancelled;
                transfer.clone()
            };
            self.publish(BackendEvent::TransferUpdated(updated));
            Ok(())
        }
    }

    fn receive_transfer(receiver: &Receiver<BackendEvent>) -> TransferSnapshot {
        match receiver.recv().expect("expected transfer event") {
            BackendEvent::TransferUpdated(transfer) => transfer,
            other => panic!("expected transfer update, received {other:?}"),
        }
    }

    #[test]
    fn backend_contract_covers_discovery_outgoing_progress_and_completion() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::StateChanged(BackendState::Running)));
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::DeviceDiscovered(_)));

        let id = backend.send_files(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).unwrap();
        let queued = receive_transfer(&receiver);
        assert_eq!(queued.id, id);
        assert_eq!(queued.state, TransferState::Queued);

        let mut progress = queued;
        progress.state = TransferState::InProgress;
        progress.bytes_transferred = 45;
        assert_eq!(progress.progress_percent(), Some(45));

        backend.complete_transfer(&id);
        let completed = receive_transfer(&receiver);
        assert_eq!(completed.state, TransferState::Completed);
        assert_eq!(completed.progress_percent(), Some(100));
    }

    #[test]
    fn backend_contract_requires_explicit_incoming_acceptance_or_rejection() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let accepted_id = backend.request_incoming();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::IncomingTransfer(_)));
        backend.decide_incoming(&accepted_id, IncomingDecision::Accept).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::InProgress);

        let rejected_id = backend.request_incoming();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::IncomingTransfer(_)));
        backend.decide_incoming(&rejected_id, IncomingDecision::Reject).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::Rejected);
    }

    #[test]
    fn backend_contract_covers_cancellation_disconnect_and_restart() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let id = backend.send_files(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).unwrap();
        let _ = receive_transfer(&receiver);
        backend.cancel_transfer(&id).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::Cancelled);

        backend.disconnect_device();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::DeviceLost(_)));
        backend.stop().unwrap();

        let restarted = backend.start_with_receiver();
        assert!(matches!(restarted.recv().unwrap(), BackendEvent::StateChanged(BackendState::Running)));
        assert!(matches!(restarted.recv().unwrap(), BackendEvent::DeviceDiscovered(_)));
    }

    #[test]
    fn backend_contract_reports_unavailable_service_and_can_recover() {
        let mut backend = ContractFakeBackend::new();
        backend.available = false;
        let (sender, receiver) = mpsc::channel();
        assert!(backend.start(sender).is_err());
        assert!(matches!(
            receiver.recv().unwrap(),
            BackendEvent::StateChanged(BackendState::Unavailable { .. })
        ));

        backend.available = true;
        let recovered = backend.start_with_receiver();
        assert!(matches!(
            recovered.recv().unwrap(),
            BackendEvent::StateChanged(BackendState::Running)
        ));
        assert!(matches!(recovered.recv().unwrap(), BackendEvent::DeviceDiscovered(_)));
    }
}
