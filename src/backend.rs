//! Protocol-independent contracts shared by Viloja's sharing backends.
//!
//! This module deliberately contains no simulated backend. A concrete adapter must
//! report real devices and real transfer events before the UI can present them.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
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
    pub pairing_state: PairingState,
    pub paired: bool,
    pub reachable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    Unpaired,
    Paired,
    IncomingRequest,
    OutgoingRequest,
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
        let transferred = self.bytes_transferred.min(total) as u128;
        let total = total as u128;
        let rounded_percent = (transferred * 100 + total / 2) / total;
        Some(rounded_percent as u8)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendErrorKind {
    DeviceUnavailable,
    Network,
    PermissionDenied,
    ServiceUnavailable,
    Unsupported,
    InvalidRequest,
    Unknown,
}

impl BackendErrorKind {
    pub const fn user_message(self) -> &'static str {
        match self {
            Self::DeviceUnavailable => "That device is no longer available. Refresh nearby devices and try again.",
            Self::Network => "The connection was interrupted. Check both devices and try again.",
            Self::PermissionDenied => "Viloja does not have permission to access the selected file or location.",
            Self::ServiceUnavailable => "The sharing service is unavailable. Reconnect the backend and try again.",
            Self::Unsupported => "This action is not supported by the selected backend.",
            Self::InvalidRequest => "The request is invalid or no longer applies. Review it and try again.",
            Self::Unknown => "The operation could not be completed. Try again or inspect diagnostics.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub backend: BackendId,
    pub kind: BackendErrorKind,
    /// Raw backend detail for diagnostics only; never display this directly to users.
    pub message: String,
    pub retryable: bool,
}

impl BackendError {
    pub fn new(
        backend: BackendId,
        kind: BackendErrorKind,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            backend,
            kind,
            message: message.into(),
            retryable,
        }
    }

    pub const fn user_message(&self) -> &'static str {
        self.kind.user_message()
    }

    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.user_message())
    }
}

impl std::error::Error for BackendError {}

/// Registry of backend snapshots presented by the application.
///
/// The registry is protocol-agnostic: adapters publish their current snapshots,
/// and UI code can render every registered backend without knowing its type.
#[derive(Debug, Clone, Default)]
pub struct BackendRegistry {
    snapshots: std::collections::HashMap<BackendId, BackendSnapshot>,
}

impl BackendRegistry {
    pub fn register(&mut self, snapshot: BackendSnapshot) {
        self.snapshots.insert(snapshot.id.clone(), snapshot);
    }

    pub fn update_state(&mut self, id: &BackendId, state: BackendState) -> bool {
        let Some(snapshot) = self.snapshots.get_mut(id) else {
            return false;
        };
        snapshot.state = state;
        true
    }

    pub fn snapshots(&self) -> Vec<BackendSnapshot> {
        let mut snapshots = self.snapshots.values().cloned().collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.display_name.to_lowercase().cmp(&right.display_name.to_lowercase()));
        snapshots
    }
}

/// A backend operation future that may complete on the GTK main context.
///
/// The boxed future keeps `SharingBackend` object-safe while allowing adapters
/// to await D-Bus, filesystem, and network operations without blocking the UI.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Contract implemented by each real sharing integration.
///
/// Adapters own protocol details and may run their networking on background
/// tasks. They report lifecycle, device and transfer changes through the event
/// sender. The UI must never infer success merely because a request was accepted.
pub trait SharingBackend {
    fn id(&self) -> BackendId;
    fn snapshot(&self) -> BackendSnapshot;
    fn pair_device<'a>(
        &'a mut self,
        device_id: &'a DeviceId,
    ) -> BackendFuture<'a, Result<(), BackendError>>;
    fn unpair_device<'a>(
        &'a mut self,
        device_id: &'a DeviceId,
    ) -> BackendFuture<'a, Result<(), BackendError>>;
    fn start(
        &mut self,
        events: Sender<BackendEvent>,
    ) -> BackendFuture<'_, Result<(), BackendError>>;
    fn stop(&mut self) -> BackendFuture<'_, Result<(), BackendError>>;
    fn send_files(
        &mut self,
        request: SendRequest,
    ) -> BackendFuture<'_, Result<TransferId, BackendError>>;
    fn decide_incoming<'a>(
        &'a mut self,
        transfer_id: &'a TransferId,
        decision: IncomingDecision,
    ) -> BackendFuture<'a, Result<(), BackendError>>;
    fn cancel_transfer<'a>(
        &'a mut self,
        transfer_id: &'a TransferId,
    ) -> BackendFuture<'a, Result<(), BackendError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_ready<F: Future>(future: F) -> F::Output {
        struct NoopWake;

        impl Wake for NoopWake {
            fn wake(self: Arc<Self>) {}
        }

        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("contract fake futures should be immediately ready"),
        }
    }

    fn transfer(bytes_transferred: u64, total_bytes: Option<u64>) -> TransferSnapshot {
        TransferSnapshot {
            id: TransferId("test-transfer".into()),
            backend: BackendId("test-backend".into()),
            device: Device {
                id: DeviceId("test-device".into()),
                backend: BackendId("test-backend".into()),
                name: "Test device".into(),
                device_type: DeviceType::Phone,
                pairing_state: PairingState::Paired,
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
    fn backend_errors_provide_safe_messages_and_retry_guidance() {
        let cases = [
            (
                BackendErrorKind::DeviceUnavailable,
                "That device is no longer available. Refresh nearby devices and try again.",
                false,
            ),
            (
                BackendErrorKind::Network,
                "The connection was interrupted. Check both devices and try again.",
                true,
            ),
            (
                BackendErrorKind::PermissionDenied,
                "Viloja does not have permission to access the selected file or location.",
                false,
            ),
            (
                BackendErrorKind::ServiceUnavailable,
                "The sharing service is unavailable. Reconnect the backend and try again.",
                true,
            ),
            (
                BackendErrorKind::Unsupported,
                "This action is not supported by the selected backend.",
                false,
            ),
            (
                BackendErrorKind::InvalidRequest,
                "The request is invalid or no longer applies. Review it and try again.",
                false,
            ),
            (
                BackendErrorKind::Unknown,
                "The operation could not be completed. Try again or inspect diagnostics.",
                false,
            ),
        ];
        let raw_detail = "peer=192.0.2.15 token=private-value";

        for (kind, expected_message, retryable) in cases {
            let error = BackendError::new(
                BackendId("test".into()),
                kind,
                raw_detail,
                retryable,
            );

            assert_eq!(error.user_message(), expected_message);
            assert_eq!(error.is_retryable(), retryable);
            assert!(!error.to_string().contains("192.0.2.15"));
            assert!(!error.to_string().contains("private-value"));
            assert_eq!(error.message, raw_detail);
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
    fn progress_rounds_fractional_percentages_without_float_precision_loss() {
        assert_eq!(transfer(1, Some(3)).progress_percent(), Some(33));
        assert_eq!(transfer(2, Some(3)).progress_percent(), Some(67));
        assert_eq!(
            transfer(u64::MAX / 2, Some(u64::MAX)).progress_percent(),
            Some(50)
        );
    }

    #[test]
    fn progress_is_unknown_for_missing_or_zero_total() {
        assert_eq!(transfer(10, None).progress_percent(), None);
        assert_eq!(transfer(0, Some(0)).progress_percent(), None);
    }

    use std::collections::HashMap;
    use std::sync::mpsc::{self, Receiver};
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct ContractFakeBackend {
        state: BackendState,
        events: Option<Sender<BackendEvent>>,
        device: Device,
        transfers: HashMap<TransferId, TransferSnapshot>,
        next_transfer_id: u64,
        available: bool,
        pairing_supported: bool,
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
                    pairing_state: PairingState::Paired,
                    paired: true,
                    reachable: true,
                },
                transfers: HashMap::new(),
                next_transfer_id: 1,
                available: true,
                pairing_supported: true,
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

        fn start_now(&mut self, events: Sender<BackendEvent>) -> Result<(), BackendError> {
            run_ready(self.start(events))
        }

        fn stop_now(&mut self) -> Result<(), BackendError> {
            run_ready(self.stop())
        }

        fn pair_device_now(&mut self, id: &DeviceId) -> Result<(), BackendError> {
            run_ready(self.pair_device(id))
        }

        fn unpair_device_now(&mut self, id: &DeviceId) -> Result<(), BackendError> {
            run_ready(self.unpair_device(id))
        }

        fn send_files_now(&mut self, request: SendRequest) -> Result<TransferId, BackendError> {
            run_ready(self.send_files(request))
        }

        fn decide_incoming_now(
            &mut self,
            id: &TransferId,
            decision: IncomingDecision,
        ) -> Result<(), BackendError> {
            run_ready(self.decide_incoming(id, decision))
        }

        fn cancel_transfer_now(&mut self, id: &TransferId) -> Result<(), BackendError> {
            run_ready(self.cancel_transfer(id))
        }

        fn start_with_receiver(&mut self) -> Receiver<BackendEvent> {
            let (sender, receiver) = mpsc::channel();
            self.start_now(sender).expect("fake backend should start");
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
                    pairing: self.pairing_supported,
                    router_free_transfer: false,
                },
            }
        }

        fn pair_device<'a>(
            &'a mut self,
            device_id: &'a DeviceId,
        ) -> BackendFuture<'a, Result<(), BackendError>> {
            Box::pin(async move {
                if !self.snapshot().capabilities.pairing {
                    return Err(BackendError::new(self.id(), BackendErrorKind::Unsupported, "backend does not support pairing", false));
                }
                if &self.device.id != device_id {
                    return Err(BackendError::new(self.id(), BackendErrorKind::DeviceUnavailable, "unknown device", false));
                }

                self.device.paired = true;
                self.publish(BackendEvent::DeviceUpdated(self.device.clone()));
                Ok(())
            })
        }

        fn unpair_device<'a>(
            &'a mut self,
            device_id: &'a DeviceId,
        ) -> BackendFuture<'a, Result<(), BackendError>> {
            Box::pin(async move {
                if !self.snapshot().capabilities.pairing {
                    return Err(BackendError::new(self.id(), BackendErrorKind::Unsupported, "backend does not support unpairing", false));
                }
                if &self.device.id != device_id {
                    return Err(BackendError::new(
                        self.id(),
                        BackendErrorKind::DeviceUnavailable,
                        "unknown device",
                        false,
                    ));
                }

                self.device.paired = false;
                self.publish(BackendEvent::DeviceUpdated(self.device.clone()));
                Ok(())
            })
        }

        fn start(
            &mut self,
            events: Sender<BackendEvent>,
        ) -> BackendFuture<'_, Result<(), BackendError>> {
            Box::pin(async move {
                self.events = Some(events);
                if !self.available {
                    self.state = BackendState::Unavailable {
                        reason: "test service unavailable".into(),
                    };
                    self.publish(BackendEvent::StateChanged(self.state.clone()));
                    return Err(BackendError::new(self.id(), BackendErrorKind::ServiceUnavailable, "test service unavailable", true));
                }

                self.state = BackendState::Running;
                self.publish(BackendEvent::StateChanged(self.state.clone()));
                self.publish(BackendEvent::DeviceDiscovered(self.device.clone()));
                Ok(())
            })
        }

        fn stop(&mut self) -> BackendFuture<'_, Result<(), BackendError>> {
            Box::pin(async move {
                self.state = BackendState::Stopped;
                self.publish(BackendEvent::StateChanged(self.state.clone()));
                self.events = None;
                Ok(())
            })
        }

        fn send_files(
            &mut self,
            request: SendRequest,
        ) -> BackendFuture<'_, Result<TransferId, BackendError>> {
            Box::pin(async move {
                if self.state != BackendState::Running {
                    return Err(BackendError::new(self.id(), BackendErrorKind::ServiceUnavailable, "backend is not running", true));
                }

                if request.device_id != self.device.id || request.files.is_empty() {
                    return Err(BackendError::new(self.id(), BackendErrorKind::InvalidRequest, "invalid destination or empty file selection", false));
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
            })
        }

        fn decide_incoming<'a>(
            &'a mut self,
            transfer_id: &'a TransferId,
            decision: IncomingDecision,
        ) -> BackendFuture<'a, Result<(), BackendError>> {
            Box::pin(async move {
                let backend_id = self.id();
                let updated = {
                    let transfer = self
                        .transfers
                        .get_mut(transfer_id)
                        .ok_or_else(|| BackendError::new(backend_id.clone(), BackendErrorKind::InvalidRequest, "unknown transfer", false))?;

                    if transfer.direction != TransferDirection::Incoming
                        || transfer.state != TransferState::AwaitingApproval
                    {
                        return Err(BackendError::new(backend_id, BackendErrorKind::InvalidRequest, "incoming transfer is not awaiting approval", false));
                    }

                    transfer.state = match decision {
                        IncomingDecision::Accept => TransferState::InProgress,
                        IncomingDecision::Reject => TransferState::Rejected,
                    };
                    transfer.clone()
                };

                self.publish(BackendEvent::TransferUpdated(updated));
                Ok(())
            })
        }

        fn cancel_transfer<'a>(
            &'a mut self,
            transfer_id: &'a TransferId,
        ) -> BackendFuture<'a, Result<(), BackendError>> {
            Box::pin(async move {
                let backend_id = self.id();
                let capabilities = self.snapshot().capabilities;
                let updated = {
                    let transfer = self
                        .transfers
                        .get_mut(transfer_id)
                        .ok_or_else(|| BackendError::new(
                            backend_id.clone(),
                            BackendErrorKind::InvalidRequest,
                            "unknown transfer",
                            false,
                        ))?;

                    let supported = match transfer.direction {
                        TransferDirection::Incoming => capabilities.cancel_incoming,
                        TransferDirection::Outgoing => capabilities.cancel_outgoing,
                    };
                    if !supported {
                        return Err(BackendError::new(backend_id, BackendErrorKind::Unsupported, "backend does not support cancelling this transfer direction", false));
                    }

                    if !matches!(
                        transfer.state,
                        TransferState::Queued | TransferState::InProgress
                    ) {
                        return Err(BackendError::new(backend_id, BackendErrorKind::InvalidRequest, "transfer is not cancellable", false));
                    }

                    transfer.state = TransferState::Cancelled;
                    transfer.clone()
                };

                self.publish(BackendEvent::TransferUpdated(updated));
                Ok(())
            })
        }
    }


    fn receive_transfer(receiver: &Receiver<BackendEvent>) -> TransferSnapshot {
        match receiver.recv().expect("expected transfer event") {
            BackendEvent::TransferUpdated(transfer) => transfer,
            other => panic!("expected transfer update, received {other:?}"),
        }
    }

    #[test]
    fn dropping_an_unpolled_backend_operation_has_no_side_effects() {
        let mut backend = ContractFakeBackend::new();
        let (sender, receiver) = mpsc::channel();

        let start = backend.start(sender);
        drop(start);

        assert_eq!(backend.state, BackendState::Stopped);
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn backend_contract_rejects_pairing_when_capability_is_disabled() {
        let mut backend = ContractFakeBackend::new();
        backend.pairing_supported = false;
        let device_id = backend.device.id.clone();

        assert!(backend.pair_device_now(&device_id).is_err());
        assert!(backend.unpair_device_now(&device_id).is_err());
        assert!(backend.device.paired);
    }

    #[test]
    fn backend_contract_covers_pairing_and_unpairing() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();
        let device_id = backend.device.id.clone();

        backend.pair_device_now(&device_id).expect("pairing should succeed");
        match receiver.recv().unwrap() {
            BackendEvent::DeviceUpdated(device) => assert!(device.paired),
            other => panic!("expected paired device update, received {other:?}"),
        }

        backend.unpair_device_now(&device_id).expect("unpairing should succeed");
        match receiver.recv().unwrap() {
            BackendEvent::DeviceUpdated(device) => assert!(!device.paired),
            other => panic!("expected unpaired device update, received {other:?}"),
        }

        let unknown = DeviceId("unknown-device".into());
        assert!(backend.pair_device_now(&unknown).is_err());
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn backend_contract_is_awaitable_through_a_trait_object() {
        let mut backend: Box<dyn SharingBackend> = Box::new(ContractFakeBackend::new());
        let (sender, receiver) = mpsc::channel();
        run_ready(backend.start(sender)).expect("backend should start");
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::StateChanged(BackendState::Running)));
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::DeviceDiscovered(_)));
        let id = run_ready(backend.send_files(SendRequest {
            device_id: DeviceId("device-1".into()),
            files: vec![PathBuf::from("/tmp/object-safe.txt")],
        })).expect("trait-object send should succeed");
        assert_eq!(receive_transfer(&receiver).id, id);
        run_ready(backend.stop()).expect("backend should stop");
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::StateChanged(BackendState::Stopped)));
    }

    #[test]
    fn backend_contract_covers_discovery_outgoing_progress_and_completion() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::StateChanged(BackendState::Running)));
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::DeviceDiscovered(_)));

        let id = backend.send_files_now(SendRequest {
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
    fn backend_contract_rejects_invalid_outgoing_requests_without_events() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        assert!(backend.send_files_now(SendRequest {
            device_id: backend.device.id.clone(),
            files: Vec::new(),
        }).is_err());
        assert!(backend.send_files_now(SendRequest {
            device_id: DeviceId("unknown-device".into()),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).is_err());

        assert!(backend.transfers.is_empty());
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn backend_contract_rejects_invalid_incoming_decisions_without_events() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let unknown_id = TransferId("missing-transfer".into());
        assert!(backend.decide_incoming_now(&unknown_id, IncomingDecision::Accept).is_err());

        let outgoing_id = backend.send_files_now(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).unwrap();
        let _ = receive_transfer(&receiver);
        assert!(backend.decide_incoming_now(&outgoing_id, IncomingDecision::Accept).is_err());

        let incoming_id = backend.request_incoming();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::IncomingTransfer(_)));
        backend.decide_incoming_now(&incoming_id, IncomingDecision::Accept).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::InProgress);

        assert!(backend.decide_incoming_now(&incoming_id, IncomingDecision::Reject).is_err());
        assert_eq!(
            backend.transfers.get(&incoming_id).unwrap().state,
            TransferState::InProgress
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn backend_contract_requires_explicit_incoming_acceptance_or_rejection() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let accepted_id = backend.request_incoming();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::IncomingTransfer(_)));
        backend.decide_incoming_now(&accepted_id, IncomingDecision::Accept).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::InProgress);

        let rejected_id = backend.request_incoming();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::IncomingTransfer(_)));
        backend.decide_incoming_now(&rejected_id, IncomingDecision::Reject).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::Rejected);
    }

    #[test]
    fn backend_contract_rejects_incoming_cancellation_when_capability_is_disabled() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let incoming_id = backend.request_incoming();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::IncomingTransfer(_)));
        backend.decide_incoming_now(&incoming_id, IncomingDecision::Accept).unwrap();
        assert!(matches!(
            receive_transfer(&receiver).state,
            TransferState::InProgress
        ));

        assert!(backend.cancel_transfer_now(&incoming_id).is_err());
        assert!(matches!(
            backend.transfers.get(&incoming_id).unwrap().state,
            TransferState::InProgress
        ));
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn backend_contract_rejects_unknown_and_completed_transfer_cancellation() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let unknown_id = TransferId("missing-transfer".into());
        assert!(backend.cancel_transfer_now(&unknown_id).is_err());
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        let completed_id = backend.send_files_now(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/completed.txt")],
        }).unwrap();
        let _ = receive_transfer(&receiver);
        backend.complete_transfer(&completed_id);
        assert_eq!(receive_transfer(&receiver).state, TransferState::Completed);

        assert!(backend.cancel_transfer_now(&completed_id).is_err());
        assert_eq!(
            backend.transfers.get(&completed_id).unwrap().state,
            TransferState::Completed
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn backend_contract_covers_cancellation_disconnect_and_restart() {
        let mut backend = ContractFakeBackend::new();
        let receiver = backend.start_with_receiver();
        let _ = receiver.recv();
        let _ = receiver.recv();

        let id = backend.send_files_now(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).unwrap();
        let _ = receive_transfer(&receiver);
        backend.cancel_transfer_now(&id).unwrap();
        assert_eq!(receive_transfer(&receiver).state, TransferState::Cancelled);

        backend.disconnect_device();
        assert!(matches!(receiver.recv().unwrap(), BackendEvent::DeviceLost(_)));
        backend.stop_now().unwrap();

        let restarted = backend.start_with_receiver();
        assert!(matches!(restarted.recv().unwrap(), BackendEvent::StateChanged(BackendState::Running)));
        match restarted.recv().unwrap() {
            BackendEvent::DeviceDiscovered(device) => {
                // Restarting the backend does not prove the remote device reconnected.
                assert!(!device.reachable);
            }
            other => panic!("expected device discovery after restart, received {other:?}"),
        }
    }

    #[test]
    fn backend_contract_rejects_sends_while_unavailable_or_stopped() {
        let mut backend = ContractFakeBackend::new();
        backend.available = false;
        let (sender, receiver) = mpsc::channel();

        assert!(backend.start_now(sender).is_err());
        assert!(matches!(
            receiver.recv().unwrap(),
            BackendEvent::StateChanged(BackendState::Unavailable { .. })
        ));
        assert!(backend.send_files_now(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).is_err());
        assert!(backend.transfers.is_empty());
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        backend.available = true;
        let events = backend.start_with_receiver();
        let _ = events.recv();
        let _ = events.recv();
        backend.stop_now().unwrap();
        assert!(matches!(
            events.recv().unwrap(),
            BackendEvent::StateChanged(BackendState::Stopped)
        ));

        assert!(backend.send_files_now(SendRequest {
            device_id: backend.device.id.clone(),
            files: vec![PathBuf::from("/tmp/example.txt")],
        }).is_err());
        assert!(backend.transfers.is_empty());
        assert!(matches!(
            events.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn backend_contract_reports_unavailable_service_and_can_recover() {
        let mut backend = ContractFakeBackend::new();
        backend.available = false;
        let (sender, receiver) = mpsc::channel();
        assert!(backend.start_now(sender).is_err());
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

    #[test]
    fn backend_registry_lists_all_registered_snapshots_in_name_order() {
        let mut registry = BackendRegistry::default();
        let capabilities = BackendCapabilities {
            discovery: true,
            send_files: false,
            receive_files: false,
            cancel_outgoing: false,
            cancel_incoming: false,
            pairing: false,
            router_free_transfer: false,
        };
        registry.register(BackendSnapshot {
            id: BackendId("zeta".into()),
            display_name: "Zeta".into(),
            state: BackendState::Unavailable {
                reason: "not installed".into(),
            },
            capabilities,
        });
        registry.register(BackendSnapshot {
            id: BackendId("alpha".into()),
            display_name: "Alpha".into(),
            state: BackendState::Running,
            capabilities,
        });

        let snapshots = registry.snapshots();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, BackendId("alpha".into()));
        assert_eq!(snapshots[1].id, BackendId("zeta".into()));
    }

    #[test]
    fn backend_registry_updates_only_the_matching_backend_state() {
        let mut registry = BackendRegistry::default();
        let id = BackendId("valent".into());
        registry.register(BackendSnapshot {
            id: id.clone(),
            display_name: "Valent".into(),
            state: BackendState::Stopped,
            capabilities: BackendCapabilities::default(),
        });

        assert!(registry.update_state(
            &id,
            BackendState::Unavailable {
                reason: "service not found".into(),
            }
        ));
        assert!(matches!(
            registry.snapshots()[0].state,
            BackendState::Unavailable { .. }
        ));
        assert!(!registry.update_state(
            &BackendId("missing".into()),
            BackendState::Running
        ));
    }
}
