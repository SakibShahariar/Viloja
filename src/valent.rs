//! Valent-specific D-Bus discovery data and decoding.
//!
//! This module owns the protocol's object/interface identifiers and converts
//! ObjectManager replies into displayable device records. GTK widgets stay in
//! the application layer; this is an extraction step toward a full adapter.

use glib::Variant;

const VALENT_BUS_NAME: &str = "ca.andyholmes.Valent";
const VALENT_OBJECT_PATH: &str = "/ca/andyholmes/Valent";
const VALENT_DEVICE_INTERFACE: &str = "ca.andyholmes.Valent.Device";
const OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";

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

