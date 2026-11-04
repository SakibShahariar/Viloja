//! Valent-specific D-Bus discovery data and decoding.
//!
//! This module owns the protocol's object/interface identifiers and converts
//! ObjectManager replies into displayable device records. GTK widgets stay in
//! the application layer; this is an extraction step toward a full adapter.

use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::{
    BackendCapabilities, BackendId, BackendSnapshot, BackendState, Device, DeviceId, DeviceType,
};

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
