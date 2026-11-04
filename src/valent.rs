//! Valent-specific D-Bus discovery data and decoding.
//!
//! This module owns the protocol's object/interface identifiers and converts
//! ObjectManager replies into displayable device records. GTK widgets stay in
//! the application layer; this is an extraction step toward a full adapter.

use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::{
    BackendCapabilities, BackendError, BackendErrorKind, BackendEvent, BackendFuture, BackendId,
    BackendSnapshot, BackendState, Device, DeviceId, DeviceType, IncomingDecision, SendRequest,
    SharingBackend, TransferId,
};
use std::collections::HashSet;
use std::sync::mpsc::Sender;

pub(crate) const VALENT_BUS_NAME: &str = "ca.andyholmes.Valent";
pub(crate) const VALENT_OBJECT_PATH: &str = "/ca/andyholmes/Valent";
pub(crate) const VALENT_DEVICE_INTERFACE: &str = "ca.andyholmes.Valent.Device";
pub(crate) const OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";

/// Stable backend identifier used by the shared contract.
pub(crate) const BACKEND_ID: &str = "valent";

/// Describe the capabilities Viloja currently implements for the Valent adapter.
pub(crate) fn backend_snapshot(state: BackendState) -> BackendSnapshot {
    BackendSnapshot {
        id: BackendId(BACKEND_ID.into()),
        display_name: "Valent".into(),
        state,
        capabilities: BackendCapabilities {
            discovery: true,
            send_files: false,
            receive_files: false,
            cancel_outgoing: false,
            cancel_incoming: false,
            pairing: false,
            router_free_transfer: false,
        },
    }
}

/// Convert Valent's discovery record into the protocol-independent device model.
pub(crate) fn to_backend_device(device: &ValentDeviceInfo) -> Device {
    let connected = device.state & 1 != 0;
    let paired = device.state & (1 | 2) != 0;

    Device {
        id: DeviceId(device.id.clone()),
        backend: BackendId(BACKEND_ID.into()),
        name: device.name.clone(),
        device_type: DeviceType::Unknown,
        paired,
        reachable: connected,
    }
}

#[derive(Debug, Clone)]
pub struct ValentDeviceInfo {
    pub id: String,
    pub name: String,
    pub icon_name: String,
    pub state: u32,
}

impl ValentDeviceInfo {
    pub fn connection_label(&self) -> &'static str {
        if self.state & 1 != 0 {
            "Connected"
        } else if self.state & 2 != 0 {
            "Paired · offline"
        } else if self.state & 4 != 0 {
            "Incoming pairing request"
        } else if self.state & 8 != 0 {
            "Outgoing pairing request"
        } else {
            "Not connected"
        }
    }
}

pub fn parse_valent_devices(reply: &glib::Variant) -> Vec<ValentDeviceInfo> {
    let managed_objects = reply.child_value(0);
    let mut devices = Vec::new();

    for object_entry in managed_objects.iter() {
        let object_path = object_entry
            .child_value(0)
            .get::<String>()
            .unwrap_or_default();
        let interfaces = object_entry.child_value(1);

        for interface_entry in interfaces.iter() {
            let interface_name = interface_entry
                .child_value(0)
                .get::<String>()
                .unwrap_or_default();
            if interface_name != VALENT_DEVICE_INTERFACE {
                continue;
            }

            let properties = interface_entry.child_value(1);
            let mut id = object_path.clone();
            let mut name = String::from("Unnamed device");
            let mut icon_name = String::new();
            let mut state = 0_u32;

            for property_entry in properties.iter() {
                let property_name = property_entry
                    .child_value(0)
                    .get::<String>()
                    .unwrap_or_default();
                let boxed_value = property_entry
                    .child_value(1)
                    .get::<glib::Variant>();

                let Some(value) = boxed_value else {
                    continue;
                };

                match property_name.as_str() {
                    "Id" => {
                        if let Some(candidate) = value.get::<String>() {
                            let candidate = candidate.trim();
                            if !candidate.is_empty() {
                                id = candidate.to_owned();
                            }
                        }
                    }
                    "Name" => {
                        if let Some(candidate) = value.get::<String>() {
                            let candidate = candidate.trim();
                            if !candidate.is_empty() {
                                name = candidate.to_owned();
                            }
                        }
                    }
                    "IconName" => {
                        if let Some(candidate) = value.get::<String>() {
                            let candidate = candidate.trim();
                            if !candidate.is_empty() {
                                icon_name = candidate.to_owned();
                            }
                        }
                    }
                    "State" => state = value.get::<u32>().unwrap_or(state),
                    _ => {}
                }
            }

            devices.push(ValentDeviceInfo {
                id,
                name,
                icon_name,
                state,
            });
        }
    }

    devices.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    devices
}

pub fn device_result_status(result: &Result<Vec<ValentDeviceInfo>, String>) -> String {
    match result {
        Ok(devices) if devices.is_empty() => {
            "Valent is available, but it has no known devices yet. Open Valent and make sure device discovery is enabled.".into()
        }
        Ok(devices) => format!(
            "Found {} device{} through Valent.",
            devices.len(),
            if devices.len() == 1 { "" } else { "s" }
        ),
        Err(_) => "Valent is unavailable or its device manager cannot be reached. Install Valent if needed, start it, enable device discovery, then refresh.".into(),
    }
}


/// Query Valent's ObjectManager and decode its current device objects.
pub fn query_devices(
    connection: &gio::DBusConnection,
    cancellable: &gio::Cancellable,
    callback: impl FnOnce(Result<Vec<ValentDeviceInfo>, String>) + 'static,
) {
    let reply_type = glib::VariantTy::new("(a{oa{sa{sv}}})")
        .expect("the ObjectManager reply signature is valid");

    connection.call(
        Some(VALENT_BUS_NAME),
        VALENT_OBJECT_PATH,
        OBJECT_MANAGER_INTERFACE,
        "GetManagedObjects",
        None,
        Some(reply_type),
        gio::DBusCallFlags::NONE,
        3000,
        Some(cancellable),
        move |reply_result| {
            let result = reply_result
                .map(|reply| parse_valent_devices(&reply))
                .map_err(|_| {
                    "Valent is unavailable or does not expose its device manager.".to_owned()
                });
            callback(result);
        },
    );
}


pub(crate) type DeviceSignalSubscriptions =
    Rc<RefCell<Option<(gio::DBusConnection, Vec<gio::SignalSubscriptionId>)>>>;

pub(crate) fn connect_session_bus(
    cancellable: &gio::Cancellable,
    callback: impl FnOnce(Result<gio::DBusConnection, glib::Error>) + 'static,
) {
    gio::bus_get(gio::BusType::Session, Some(cancellable), callback);
}

pub(crate) fn clear_device_signal_subscriptions(subscriptions: &DeviceSignalSubscriptions) {
    if let Some((connection, ids)) = subscriptions.borrow_mut().take() {
        for id in ids {
            connection.signal_unsubscribe(id);
        }
    }
}

pub(crate) fn subscribe_device_changes(
    connection: &gio::DBusConnection,
    refresh: Rc<dyn Fn()>,
    subscriptions: &DeviceSignalSubscriptions,
) {
    clear_device_signal_subscriptions(subscriptions);

    let properties_refresh = refresh.clone();
    let properties_id = connection.signal_subscribe(
        Some(VALENT_BUS_NAME),
        Some("org.freedesktop.DBus.Properties"),
        Some("PropertiesChanged"),
        None,
        Some(VALENT_DEVICE_INTERFACE),
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, _| properties_refresh(),
    );

    let added_refresh = refresh.clone();
    let added_id = connection.signal_subscribe(
        Some(VALENT_BUS_NAME),
        Some(OBJECT_MANAGER_INTERFACE),
        Some("InterfacesAdded"),
        Some(VALENT_OBJECT_PATH),
        None,
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, _| added_refresh(),
    );

    let removed_id = connection.signal_subscribe(
        Some(VALENT_BUS_NAME),
        Some(OBJECT_MANAGER_INTERFACE),
        Some("InterfacesRemoved"),
        Some(VALENT_OBJECT_PATH),
        None,
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, _| refresh(),
    );

    *subscriptions.borrow_mut() = Some((connection.clone(), vec![properties_id, added_id, removed_id]));
}

pub(crate) struct NameWatcher(Box<dyn FnOnce()>);

pub(crate) fn watch_name(
    appeared: impl Fn() + 'static,
    vanished: impl Fn() + 'static,
) -> NameWatcher {
    let watcher_id = gio::bus_watch_name(
        gio::BusType::Session,
        VALENT_BUS_NAME,
        gio::BusNameWatcherFlags::NONE,
        move |_, _, _| appeared(),
        move |_, _| vanished(),
    );

    NameWatcher(Box::new(move || gio::bus_unwatch_name(watcher_id)))
}

pub(crate) fn unwatch_name(watcher_id: NameWatcher) {
    (watcher_id.0)();
}


#[cfg(test)]
mod backend_contract_tests {
    use super::*;

    fn info(state: u32) -> ValentDeviceInfo {
        ValentDeviceInfo {
            id: "valent-device-1".into(),
            name: "Example phone".into(),
            icon_name: "phone-symbolic".into(),
            state,
        }
    }

    #[test]
    fn discovery_record_maps_to_shared_device_contract() {
        let device = to_backend_device(&info(1));

        assert_eq!(device.id, DeviceId("valent-device-1".into()));
        assert_eq!(device.backend, BackendId(BACKEND_ID.into()));
        assert_eq!(device.name, "Example phone");
        assert_eq!(device.device_type, DeviceType::Unknown);
        assert!(device.paired);
        assert!(device.reachable);
    }

    #[test]
    fn paired_offline_device_is_not_reported_as_reachable() {
        let device = to_backend_device(&info(2));

        assert!(device.paired);
        assert!(!device.reachable);
    }

    #[test]
    fn unpaired_device_remains_unpaired_and_unreachable() {
        let device = to_backend_device(&info(0));

        assert!(!device.paired);
        assert!(!device.reachable);
    }

    #[test]
    fn valent_snapshot_does_not_claim_unimplemented_transfer_features() {
        let snapshot = backend_snapshot(BackendState::Stopped);

        assert_eq!(snapshot.id, BackendId(BACKEND_ID.into()));
        assert_eq!(snapshot.display_name, "Valent");
        assert_eq!(snapshot.state, BackendState::Stopped);
        assert!(snapshot.capabilities.discovery);
        assert!(!snapshot.capabilities.send_files);
        assert!(!snapshot.capabilities.receive_files);
        assert!(!snapshot.capabilities.cancel_outgoing);
        assert!(!snapshot.capabilities.cancel_incoming);
        assert!(!snapshot.capabilities.pairing);
        assert!(!snapshot.capabilities.router_free_transfer);
    }
}


fn publish_device_list(
    devices: Vec<ValentDeviceInfo>,
    events: &Sender<BackendEvent>,
    known_devices: &Rc<RefCell<HashSet<DeviceId>>>,
) {
    let current_ids = devices
        .iter()
        .map(|device| DeviceId(device.id.clone()))
        .collect::<HashSet<_>>();
    let mut known = known_devices.borrow_mut();

    let removed = known
        .difference(&current_ids)
        .cloned()
        .collect::<Vec<_>>();
    for id in removed {
        known.remove(&id);
        let _ = events.send(BackendEvent::DeviceLost(id));
    }

    for device in devices {
        let device = to_backend_device(&device);
        let event = if known.insert(device.id.clone()) {
            BackendEvent::DeviceDiscovered(device)
        } else {
            BackendEvent::DeviceUpdated(device)
        };
        let _ = events.send(event);
    }
}

fn subscribe_backend_device_changes(
    connection: &gio::DBusConnection,
    events: Sender<BackendEvent>,
    known_devices: Rc<RefCell<HashSet<DeviceId>>>,
    subscriptions: &DeviceSignalSubscriptions,
) {
    let connection = connection.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        let events = events.clone();
        let known_devices = known_devices.clone();
        let cancellable = gio::Cancellable::new();
        query_devices(&connection, &cancellable, move |result| {
            if let Ok(devices) = result {
                publish_device_list(devices, &events, &known_devices);
            }
        });
    });
    subscribe_device_changes(connection.as_ref(), refresh, subscriptions);
}

/// Concrete discovery adapter for Valent.
///
/// Transfer and pairing operations intentionally return Unsupported until Viloja
/// implements those actions itself; the capability snapshot never advertises them.
pub(crate) struct ValentBackend {
    state: Rc<RefCell<BackendState>>,
    connection: Option<gio::DBusConnection>,
    subscriptions: DeviceSignalSubscriptions,
    watcher: Option<NameWatcher>,
    events: Option<Sender<BackendEvent>>,
    known_devices: Rc<RefCell<HashSet<DeviceId>>>,
}

impl ValentBackend {
    pub(crate) fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(BackendState::Stopped)),
            connection: None,
            subscriptions: Rc::new(RefCell::new(None)),
            watcher: None,
            events: None,
            known_devices: Rc::new(RefCell::new(HashSet::new())),
        }
    }

    fn publish_state(&self, state: BackendState) {
        *self.state.borrow_mut() = state.clone();
        if let Some(events) = &self.events {
            let _ = events.send(BackendEvent::StateChanged(state));
        }
    }

    fn unsupported_error(&self) -> BackendError {
        BackendError::new(
            self.id(),
            BackendErrorKind::Unsupported,
            "This operation is not implemented by the Valent adapter.",
            false,
        )
    }
}

async fn query_devices_future(
    connection: &gio::DBusConnection,
) -> Result<Vec<ValentDeviceInfo>, BackendError> {
    let reply_type = glib::VariantTy::new("(a{oa{sa{sv}}})")
        .expect("the ObjectManager reply signature is valid");
    let reply = connection
        .call_future(
            Some(VALENT_BUS_NAME),
            VALENT_OBJECT_PATH,
            OBJECT_MANAGER_INTERFACE,
            "GetManagedObjects",
            None,
            Some(reply_type),
            gio::DBusCallFlags::NONE,
            3000,
        )
        .await
        .map_err(|error| {
            BackendError::new(
                BackendId(BACKEND_ID.into()),
                BackendErrorKind::ServiceUnavailable,
                error.to_string(),
                true,
            )
        })?;
    Ok(parse_valent_devices(&reply))
}

impl SharingBackend for ValentBackend {
    fn id(&self) -> BackendId {
        BackendId(BACKEND_ID.into())
    }

    fn snapshot(&self) -> BackendSnapshot {
        backend_snapshot(self.state.borrow().clone())
    }

    fn pair_device<'a>(
        &'a mut self,
        _device_id: &'a DeviceId,
    ) -> BackendFuture<'a, Result<(), BackendError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn unpair_device<'a>(
        &'a mut self,
        _device_id: &'a DeviceId,
    ) -> BackendFuture<'a, Result<(), BackendError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn start(
        &mut self,
        events: Sender<BackendEvent>,
    ) -> BackendFuture<'_, Result<(), BackendError>> {
        Box::pin(async move {
            clear_device_signal_subscriptions(&self.subscriptions);
            if let Some(watcher) = self.watcher.take() {
                unwatch_name(watcher);
            }
            self.connection = None;
            self.known_devices.borrow_mut().clear();
            self.events = Some(events.clone());
            self.publish_state(BackendState::Starting);

            let connection = match gio::bus_get_future(gio::BusType::Session).await {
                Ok(connection) => connection,
                Err(error) => {
                    self.publish_state(BackendState::Unavailable {
                        reason: "The session bus is unavailable.".into(),
                    });
                    return Err(BackendError::new(
                        self.id(),
                        BackendErrorKind::ServiceUnavailable,
                        error.to_string(),
                        true,
                    ));
                }
            };
            self.connection = Some(connection.clone());

            subscribe_backend_device_changes(
                &connection,
                events.clone(),
                self.known_devices.clone(),
                &self.subscriptions,
            );

            let state_appeared = self.state.clone();
            let state_vanished = self.state.clone();
            let events_appeared = events.clone();
            let events_vanished = events.clone();
            let connection_appeared = connection.clone();
            let known_appeared = self.known_devices.clone();
            let known_vanished = self.known_devices.clone();
            let subscriptions_appeared = self.subscriptions.clone();
            let subscriptions_vanished = self.subscriptions.clone();

            self.watcher = Some(watch_name(
                move || {
                    *state_appeared.borrow_mut() = BackendState::Running;
                    let _ = events_appeared.send(BackendEvent::StateChanged(BackendState::Running));
                    subscribe_backend_device_changes(
                        &connection_appeared,
                        events_appeared.clone(),
                        known_appeared.clone(),
                        &subscriptions_appeared,
                    );
                    let events = events_appeared.clone();
                    let known = known_appeared.clone();
                    let cancellable = gio::Cancellable::new();
                    query_devices(&connection_appeared, &cancellable, move |result| {
                        if let Ok(devices) = result {
                            publish_device_list(devices, &events, &known);
                        }
                    });
                },
                move || {
                    clear_device_signal_subscriptions(&subscriptions_vanished);
                    let removed = known_vanished
                        .borrow_mut()
                        .drain()
                        .collect::<Vec<_>>();
                    for id in removed {
                        let _ = events_vanished.send(BackendEvent::DeviceLost(id));
                    }
                    let state = BackendState::Unavailable {
                        reason: "Valent is not running.".into(),
                    };
                    *state_vanished.borrow_mut() = state.clone();
                    let _ = events_vanished.send(BackendEvent::StateChanged(state));
                },
            ));

            match query_devices_future(&connection).await {
                Ok(devices) => {
                    publish_device_list(devices, &events, &self.known_devices);
                    self.publish_state(BackendState::Running);
                    Ok(())
                }
                Err(error) => {
                    self.publish_state(BackendState::Unavailable {
                        reason: "Valent is unavailable.".into(),
                    });
                    Err(error)
                }
            }
        })
    }

    fn stop(&mut self) -> BackendFuture<'_, Result<(), BackendError>> {
        Box::pin(async move {
            clear_device_signal_subscriptions(&self.subscriptions);
            if let Some(watcher) = self.watcher.take() {
                unwatch_name(watcher);
            }
            self.connection = None;
            self.known_devices.borrow_mut().clear();
            self.publish_state(BackendState::Stopped);
            self.events = None;
            Ok(())
        })
    }

    fn send_files(
        &mut self,
        _request: SendRequest,
    ) -> BackendFuture<'_, Result<TransferId, BackendError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn decide_incoming<'a>(
        &'a mut self,
        _transfer_id: &'a TransferId,
        _decision: IncomingDecision,
    ) -> BackendFuture<'a, Result<(), BackendError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn cancel_transfer<'a>(
        &'a mut self,
        _transfer_id: &'a TransferId,
    ) -> BackendFuture<'a, Result<(), BackendError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }
}

#[cfg(test)]
mod valent_backend_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn discovery_events_report_added_updated_and_removed_devices() {
        let (events, receiver) = mpsc::channel();
        let known = Rc::new(RefCell::new(HashSet::new()));
        let device = ValentDeviceInfo {
            id: "device-1".into(),
            name: "Example phone".into(),
            icon_name: String::new(),
            state: 1,
        };

        publish_device_list(vec![device.clone()], &events, &known);
        assert!(matches!(
            receiver.try_recv(),
            Ok(BackendEvent::DeviceDiscovered(device)) if device.name == "Example phone"
        ));

        publish_device_list(vec![device], &events, &known);
        assert!(matches!(
            receiver.try_recv(),
            Ok(BackendEvent::DeviceUpdated(device)) if device.name == "Example phone"
        ));

        publish_device_list(Vec::new(), &events, &known);
        assert!(matches!(
            receiver.try_recv(),
            Ok(BackendEvent::DeviceLost(DeviceId(id))) if id == "device-1"
        ));
    }

    #[test]
    fn valent_backend_implements_the_shared_object_safe_contract() {
        let backend = ValentBackend::new();
        let contract: &dyn SharingBackend = &backend;

        assert_eq!(contract.id(), BackendId(BACKEND_ID.into()));
        assert_eq!(contract.snapshot().state, BackendState::Stopped);
        assert!(contract.snapshot().capabilities.discovery);
        assert!(!contract.snapshot().capabilities.send_files);
    }

    #[test]
    fn unsupported_valent_operations_return_a_safe_typed_error() {
        let backend = ValentBackend::new();
        let error = backend.unsupported_error();

        assert_eq!(error.kind, BackendErrorKind::Unsupported);
        assert!(!error.is_retryable());
        assert!(!error.to_string().contains("not implemented by the Valent adapter"));
        assert!(error.user_message().contains("not supported"));
    }
}
