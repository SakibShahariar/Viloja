mod backend;
mod diagnostics;
mod valent;

use adw::prelude::*;
use diagnostics::{DiagnosticEvent, DiagnosticLevel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// Shared slot for the active backend adapter.
///
/// Lifecycle operations take the adapter out of this slot before awaiting, so a
/// `RefCell` borrow is never held across an `.await` point and a refresh cannot
/// panic while a previous discovery is still pending.
type BackendSlot = Rc<RefCell<Option<Box<dyn backend::SharingBackend>>>>;

fn prune_selected_device(
    selected_device: &Rc<RefCell<Option<String>>>,
    devices: &[backend::Device],
) -> Option<String> {
    let mut selected = selected_device.borrow_mut();
    let selected_name = selected.as_ref().and_then(|id| {
        devices
            .iter()
            .find(|device| &device.id.0 == id)
            .map(|device| device.name.clone())
    });

    if selected_name.is_none() {
        *selected = None;
    }

    selected_name
}

fn clear_selected_device(selected_device: &Rc<RefCell<Option<String>>>) -> bool {
    selected_device.borrow_mut().take().is_some()
}

fn make_action_row(title: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build()
}

/// Rows added to a `PreferencesGroup` through `add`, tracked so the group can be
/// cleared without touching its internal header/list widgets.
type GroupRows = Rc<RefCell<Vec<gtk::Widget>>>;

fn clear_group(group: &adw::PreferencesGroup, rows: &GroupRows) {
    for row in rows.borrow_mut().drain(..) {
        group.remove(&row);
    }
}

fn add_row(group: &adw::PreferencesGroup, rows: &GroupRows, row: &impl IsA<gtk::Widget>) {
    group.add(row);
    rows.borrow_mut().push(row.clone().upcast());
}

fn connection_label(device: &backend::Device) -> &'static str {
    match device.pairing_state {
        backend::PairingState::IncomingRequest => "Incoming pairing request",
        backend::PairingState::OutgoingRequest => "Outgoing pairing request",
        backend::PairingState::Paired if device.reachable => "Connected",
        backend::PairingState::Paired => "Paired · offline",
        backend::PairingState::Unpaired => "Not connected",
    }
}

fn render_device_list(
    device_list: &adw::PreferencesGroup,
    device_rows: &GroupRows,
    known_devices: &HashMap<backend::DeviceId, backend::Device>,
    selected_device: &Rc<RefCell<Option<String>>>,
    selection_status: &adw::ActionRow,
) {
    clear_group(device_list, device_rows);
    let mut devices = known_devices.values().cloned().collect::<Vec<_>>();
    devices.sort_by_key(|device| device.name.to_lowercase());

    match prune_selected_device(selected_device, &devices) {
        Some(name) => selection_status.set_subtitle(&format!("Selected destination: {name}")),
        None => selection_status.set_subtitle("No device selected."),
    }

    for device in devices {
        let row = adw::ActionRow::builder()
            .title(&device.name)
            .subtitle(connection_label(&device))
            .activatable(true)
            .build();
        let icon_name = match device.device_type {
            backend::DeviceType::Phone => "phone-symbolic",
            backend::DeviceType::Tablet => "tablet-symbolic",
            backend::DeviceType::Computer | backend::DeviceType::Unknown => "computer-symbolic",
        };
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(32);
        row.add_prefix(&icon);

        let selected = selected_device.clone();
        let selection_status = selection_status.clone();
        let device_id = device.id.0.clone();
        let device_name = device.name.clone();
        row.connect_activated(move |_| {
            *selected.borrow_mut() = Some(device_id.clone());
            selection_status.set_subtitle(&format!("Selected destination: {device_name}"));
        });
        add_row(device_list, device_rows, &row);
    }
}

fn start_backend(
    backend: BackendSlot,
    events: std::sync::mpsc::Sender<backend::BackendEvent>,
    refresh_button: adw::ActionRow,
) {
    diagnostics::record(
        DiagnosticLevel::Info,
        DiagnosticEvent::BackendRefreshStarted,
    );
    glib::MainContext::default().spawn_local(async move {
        let Some(mut adapter) = backend.borrow_mut().take() else {
            refresh_button.set_sensitive(true);
            return;
        };
        let result = adapter.start(events).await;
        *backend.borrow_mut() = Some(adapter);
        refresh_button.set_sensitive(true);
        if result.is_err() {
            diagnostics::record(
                DiagnosticLevel::Error,
                DiagnosticEvent::BackendDiscoveryFailed,
            );
        }
    });
}

fn make_page(title: &str, description: &str) -> adw::PreferencesPage {
    adw::PreferencesPage::builder()
        .title(title)
        .description(description)
        .build()
}

fn make_card(title: &str, body: &str) -> adw::PreferencesGroup {
    adw::PreferencesGroup::builder()
        .title(title)
        .description(body)
        .build()
}

const VALENT_LIMITATIONS: &str = "Device discovery: experimental session-D-Bus query; not verified against an installed Valent/GNOME session. Pairing: handled in Valent, not managed by Viloja. Sending and receiving files: not implemented. Router-free, hotspot, Wi-Fi Direct, and Bluetooth transfers: not verified; no support is claimed.";

fn backend_limitations_card() -> adw::PreferencesGroup {
    make_card("Current backend: Valent (experimental)", VALENT_LIMITATIONS)
}

fn backend_capabilities_text(snapshot: &backend::BackendSnapshot) -> String {
    let availability = match &snapshot.state {
        backend::BackendState::Unavailable { .. } => "unavailable",
        backend::BackendState::Stopped => "stopped",
        backend::BackendState::Starting => "starting",
        backend::BackendState::Running => "available",
        backend::BackendState::Stopping => "stopping",
        backend::BackendState::Failed { .. } => "failed",
    };
    let capabilities = snapshot.capabilities;
    let discovery = if capabilities.discovery {
        "experimental"
    } else {
        "unavailable"
    };
    let pairing = if capabilities.pairing {
        "available"
    } else {
        "unavailable in Viloja (managed externally by Valent)"
    };
    let sending = if capabilities.send_files {
        "available"
    } else {
        "unavailable"
    };
    let receiving = if capabilities.receive_files {
        "available"
    } else {
        "unavailable"
    };
    let cancellation = if capabilities.cancel_outgoing || capabilities.cancel_incoming {
        "available for supported transfer directions"
    } else {
        "unavailable"
    };
    let router_free = if capabilities.router_free_transfer {
        "available"
    } else {
        "unknown / not provided by this adapter"
    };

    format!(
        "Availability: {availability}. Discovery: {discovery}. Pairing: {pairing}. Send files: {sending}. Receive files: {receiving}. Transfer cancellation: {cancellation}. Router-free transfer: {router_free}."
    )
}

fn backend_capabilities_row(snapshot: &backend::BackendSnapshot) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(format!("{} backend capabilities", snapshot.display_name))
        .subtitle(backend_capabilities_text(snapshot))
        .subtitle_lines(4)
        .build()
}

fn render_backend_capabilities(
    container: &adw::PreferencesGroup,
    rows: &GroupRows,
    registry: &backend::BackendRegistry,
) {
    clear_group(container, rows);
    for snapshot in registry.snapshots() {
        add_row(container, rows, &backend_capabilities_row(&snapshot));
    }
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Viloja")
        .default_width(1000)
        .default_height(700)
        .build();

    let backend: BackendSlot = Rc::new(RefCell::new(Some(valent::create_backend())));
    let backend_snapshot = backend
        .borrow()
        .as_ref()
        .expect("the backend is initialized before the UI is built")
        .snapshot();
    let mut initial_registry = backend::BackendRegistry::default();
    initial_registry.register(backend_snapshot);
    let backend_registry = Rc::new(RefCell::new(initial_registry));
    let (event_sender, event_receiver) = mpsc::channel::<backend::BackendEvent>();
    let known_devices: Rc<RefCell<HashMap<backend::DeviceId, backend::Device>>> =
        Rc::new(RefCell::new(HashMap::new()));

    let header = adw::HeaderBar::new();
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);
    stack.set_hexpand(true);
    let switcher = adw::ViewSwitcher::builder().stack(&stack).build();
    header.set_title_widget(Some(&switcher));

    // Nearby
    let nearby = make_page(
        "Nearby",
        "Find devices known to the active sharing backend.",
    );
    let discovery_group = adw::PreferencesGroup::builder().title("Discovery").build();
    let discovery_status = adw::ActionRow::builder()
        .title("Status")
        .subtitle("Starting device discovery…")
        .build();
    let refresh_button = make_action_row("Refresh devices");
    refresh_button.add_css_class("suggested-action");
    refresh_button.set_sensitive(false);
    discovery_group.add(&discovery_status);
    discovery_group.add(&refresh_button);
    nearby.add(&discovery_group);

    let device_list = adw::PreferencesGroup::builder()
        .title("Devices")
        .description("Devices reported by the active backend appear here.")
        .build();
    let device_rows: GroupRows = Rc::new(RefCell::new(Vec::new()));
    let selected_device: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selection_status = adw::ActionRow::builder()
        .title("Selected destination")
        .subtitle("No device selected.")
        .build();
    let selection_group = adw::PreferencesGroup::builder().title("Selection").build();
    selection_group.add(&selection_status);
    nearby.add(&device_list);
    nearby.add(&selection_group);
    nearby.add(&backend_limitations_card());
    let nearby_backend_cards = adw::PreferencesGroup::builder()
        .title("Backend capabilities")
        .build();
    let nearby_backend_rows: GroupRows = Rc::new(RefCell::new(Vec::new()));
    render_backend_capabilities(
        &nearby_backend_cards,
        &nearby_backend_rows,
        &backend_registry.borrow(),
    );
    nearby.add(&nearby_backend_cards);
    let settings_backend_cards = adw::PreferencesGroup::builder()
        .title("Backend capabilities")
        .build();
    let settings_backend_rows: GroupRows = Rc::new(RefCell::new(Vec::new()));

    let backend_for_refresh = backend.clone();
    let events_for_refresh = event_sender.clone();
    let button_for_refresh = refresh_button.clone();
    let devices_for_refresh = known_devices.clone();
    let list_for_refresh = device_list.clone();
    let rows_for_refresh = device_rows.clone();
    let selected_for_refresh = selected_device.clone();
    let selection_for_refresh = selection_status.clone();
    let status_for_refresh = discovery_status.clone();
    refresh_button.connect_activated(move |_| {
        devices_for_refresh.borrow_mut().clear();
        clear_group(&list_for_refresh, &rows_for_refresh);
        clear_selected_device(&selected_for_refresh);
        selection_for_refresh.set_subtitle("No device selected.");
        status_for_refresh.set_subtitle("Refreshing devices…");
        button_for_refresh.set_sensitive(false);
        start_backend(
            backend_for_refresh.clone(),
            events_for_refresh.clone(),
            button_for_refresh.clone(),
        );
    });

    let receiver = event_receiver;
    let devices_for_events = known_devices.clone();
    let list_for_events = device_list.clone();
    let rows_for_events = device_rows.clone();
    let selected_for_events = selected_device.clone();
    let selection_for_events = selection_status.clone();
    let status_for_events = discovery_status.clone();
    let registry_for_events = backend_registry.clone();
    let nearby_cards_for_events = nearby_backend_cards.clone();
    let nearby_rows_for_events = nearby_backend_rows.clone();
    let settings_cards_for_events = settings_backend_cards.clone();
    let settings_rows_for_events = settings_backend_rows.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        loop {
            let event = match receiver.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return glib::ControlFlow::Break,
            };

            match event {
                backend::BackendEvent::StateChanged(state) => match state {
                    backend::BackendState::Starting => {
                        status_for_events.set_subtitle("Connecting to the sharing backend…");
                    }
                    backend::BackendState::Running => {
                        let count = devices_for_events.borrow().len();
                        if count == 0 {
                            status_for_events.set_subtitle(
                                "The backend is available, but it has no known devices yet.",
                            );
                        } else {
                            status_for_events.set_subtitle(&format!(
                                "Found {count} device{}.",
                                if count == 1 { "" } else { "s" }
                            ));
                        }
                    }
                    backend::BackendState::Unavailable { .. }
                    | backend::BackendState::Failed { .. } => {
                        devices_for_events.borrow_mut().clear();
                        clear_selected_device(&selected_for_events);
                        render_device_list(
                            &list_for_events,
                            &rows_for_events,
                            &devices_for_events.borrow(),
                            &selected_for_events,
                            &selection_for_events,
                        );
                        status_for_events.set_subtitle(
                            "Valent is unavailable. Start Valent to restore discovery; Viloja will retry when it appears.",
                        );
                        diagnostics::record(
                            DiagnosticLevel::Error,
                            DiagnosticEvent::BackendUnavailable,
                        );
                    }
                    backend::BackendState::Stopped => {
                        status_for_events.set_subtitle("Device discovery is stopped.");
                    }
                    backend::BackendState::Stopping => {
                        status_for_events.set_subtitle("Stopping device discovery…");
                    }
                },
                backend::BackendEvent::DeviceDiscovered(device) => {
                    devices_for_events
                        .borrow_mut()
                        .insert(device.id.clone(), device);
                    let devices = devices_for_events.borrow();
                    let count = devices.len();
                    render_device_list(
                        &list_for_events,
                        &rows_for_events,
                        &devices,
                        &selected_for_events,
                        &selection_for_events,
                    );
                    status_for_events.set_subtitle(&format!(
                        "Found {count} device{}.",
                        if count == 1 { "" } else { "s" }
                    ));
                    diagnostics::record(
                        DiagnosticLevel::Info,
                        DiagnosticEvent::DiscoveryCompleted {
                            device_count: count,
                        },
                    );
                }
                backend::BackendEvent::DeviceUpdated(device) => {
                    devices_for_events
                        .borrow_mut()
                        .insert(device.id.clone(), device);
                    let devices = devices_for_events.borrow();
                    render_device_list(
                        &list_for_events,
                        &rows_for_events,
                        &devices,
                        &selected_for_events,
                        &selection_for_events,
                    );
                }
                backend::BackendEvent::DeviceLost(device_id) => {
                    devices_for_events.borrow_mut().remove(&device_id);
                    render_device_list(
                        &list_for_events,
                        &rows_for_events,
                        &devices_for_events.borrow(),
                        &selected_for_events,
                        &selection_for_events,
                    );
                }
                backend::BackendEvent::BackendSnapshotChanged(snapshot) => {
                    registry_for_events.borrow_mut().register(snapshot);
                    render_backend_capabilities(
                        &nearby_cards_for_events,
                        &nearby_rows_for_events,
                        &registry_for_events.borrow(),
                    );
                    render_backend_capabilities(
                        &settings_cards_for_events,
                        &settings_rows_for_events,
                        &registry_for_events.borrow(),
                    );
                }
                backend::BackendEvent::IncomingTransfer(_)
                | backend::BackendEvent::TransferUpdated(_) => {}
            }
        }
        glib::ControlFlow::Continue
    });

    let backend_for_close = backend.clone();
    window.connect_close_request(move |_| {
        if let Some(mut adapter) = backend_for_close.borrow_mut().take() {
            glib::MainContext::default().spawn_local(async move {
                let _ = adapter.stop().await;
            });
        }
        glib::Propagation::Proceed
    });

    stack.add_titled(&nearby, Some("nearby"), "Nearby");

    // Send files
    let send = make_page(
        "Send files",
        "Choose files to prepare a transfer. Viloja cannot send them yet.",
    );
    send.add(&backend_limitations_card());
    let choose_group = adw::PreferencesGroup::builder()
        .title("Select files")
        .description(
            "You can select multiple files and review the selection here. Nothing is sent by this prototype.",
        )
        .build();
    let choose_button = make_action_row("Choose files…");
    choose_button.add_css_class("suggested-action");
    choose_group.add(&choose_button);
    send.add(&choose_group);
    let send_selection_status = adw::ActionRow::builder()
        .title("Selected files")
        .subtitle("No files selected.")
        .build();
    let send_selection_group = adw::PreferencesGroup::builder().title("Selection").build();
    send_selection_group.add(&send_selection_status);
    send.add(&send_selection_group);
    let transfer_status = adw::PreferencesGroup::builder()
        .title("Transfer status")
        .description("Transfer will remain disabled until a real device action is connected.")
        .build();
    send.add(&transfer_status);
    stack.add_titled(&send, Some("send"), "Send files");

    // Transfers
    let transfers = make_page(
        "Transfers",
        "Transfer monitoring is unavailable until a real backend transfer flow is integrated.",
    );
    transfers.add(&backend_limitations_card());
    transfers.add(
        &adw::PreferencesGroup::builder()
            .title("No transfers yet")
            .description(
                "Transfer progress, cancellation, completion, and error states will appear once transfer events are integrated.",
            )
            .build(),
    );
    stack.add_titled(&transfers, Some("transfers"), "Transfers");

    // History
    let history = make_page(
        "History",
        "Viloja does not yet store transfer history because transfers are not integrated.",
    );
    history.add(&backend_limitations_card());
    history.add(
        &adw::PreferencesGroup::builder()
            .title("History is empty")
            .description("Viloja does not record transfer history in this prototype.")
            .build(),
    );
    stack.add_titled(&history, Some("history"), "History");

    // Settings
    let settings = make_page(
        "Settings",
        "Backend availability and transfer preferences will be configurable when those integrations exist.",
    );
    settings.add(&backend_limitations_card());
    settings.add(&settings_backend_cards);
    settings.add(&make_card(
        "Receiving",
        "Receiving preferences will be available when the backend exposes its supported options.",
    ));
    settings.add(&make_card(
        "Downloads",
        "The download destination has not been configured yet.",
    ));
    settings.add(&make_card(
        "Privacy",
        "Device visibility and trust controls will be based on the active backend's real capabilities.",
    ));

    let diagnostics_group = adw::PreferencesGroup::builder()
        .title("Diagnostics")
        .description(
            "Diagnostic entries use fixed event names and numeric counts only. Device names, addresses, tokens, paths, and raw backend error strings are never added to this log.",
        )
        .build();
    let verbosity = adw::ComboRow::builder()
        .title("Log verbosity")
        .model(&gtk::StringList::new(&[
            "Errors only",
            "Normal",
            "Verbose",
            "Off",
        ]))
        .build();
    verbosity.set_selected(diagnostics::verbosity_index());
    verbosity.connect_selected_notify(|row| diagnostics::set_verbosity(row.selected()));
    diagnostics_group.add(&verbosity);

    let diagnostic_view = gtk::TextView::new();
    diagnostic_view.set_editable(false);
    diagnostic_view.set_cursor_visible(false);
    diagnostic_view.set_monospace(true);
    diagnostic_view.set_wrap_mode(gtk::WrapMode::WordChar);
    diagnostic_view.buffer().set_text(&diagnostics::export());
    let diagnostic_scroll = gtk::ScrolledWindow::new();
    diagnostic_scroll.set_min_content_height(180);
    diagnostic_scroll.set_child(Some(&diagnostic_view));
    diagnostics_group.add(&diagnostic_scroll);

    let refresh_diagnostics = make_action_row("Refresh diagnostics");
    let refresh_buffer = diagnostic_view.buffer();
    refresh_diagnostics.connect_activated(move |_| refresh_buffer.set_text(&diagnostics::export()));
    diagnostics_group.add(&refresh_diagnostics);

    let copy_diagnostics = make_action_row("Copy diagnostics");
    let diagnostic_buffer = diagnostic_view.buffer();
    let diagnostic_display = gtk::prelude::WidgetExt::display(&window);
    copy_diagnostics.connect_activated(move |_| {
        let exported = diagnostics::export();
        diagnostic_buffer.set_text(&exported);
        diagnostic_display.clipboard().set_text(&exported);
    });
    diagnostics_group.add(&copy_diagnostics);

    let clear_diagnostics = make_action_row("Clear diagnostics");
    let clear_buffer = diagnostic_view.buffer();
    clear_diagnostics.connect_activated(move |_| {
        diagnostics::clear();
        clear_buffer.set_text("");
    });
    diagnostics_group.add(&clear_diagnostics);
    settings.add(&diagnostics_group);
    stack.add_titled(&settings, Some("settings"), "Settings");

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&stack));
    window.set_content(Some(&toolbar));

    let chooser_window = window.clone();
    let status_label = send_selection_status.clone();
    choose_button.connect_activated(move |_| {
        let chooser = gtk::FileChooserNative::new(
            Some("Choose files to prepare"),
            Some(&chooser_window),
            gtk::FileChooserAction::Open,
            Some("Select"),
            Some("Cancel"),
        );
        chooser.set_select_multiple(true);

        let status = status_label.clone();
        chooser.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let count = dialog.files().n_items();
                if count == 1 {
                    status.set_subtitle("1 file selected. Ready for a backend integration.");
                } else {
                    status.set_subtitle(&format!(
                        "{count} files selected. Ready for a backend integration."
                    ));
                }
            }
            dialog.destroy();
        });
        chooser.show();
    });

    start_backend(backend.clone(), event_sender, refresh_button.clone());
    window.present();
}

fn main() {
    let app = adw::Application::builder()
        .application_id("io.github.SakibShahariar.Viloja")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::valent::parse_valent_devices;

    fn backend_device(name: &str) -> backend::Device {
        backend::Device {
            id: backend::DeviceId(format!("device-{name}")),
            backend: backend::BackendId("valent".into()),
            name: name.into(),
            device_type: backend::DeviceType::Unknown,
            pairing_state: backend::PairingState::Paired,
            paired: true,
            reachable: true,
        }
    }

    #[test]
    fn selected_device_name_tracks_refreshes_and_selection_is_pruned_when_missing() {
        let selected = Rc::new(RefCell::new(Some("device-Phone".to_string())));

        assert_eq!(
            prune_selected_device(&selected, &[backend_device("Phone")]).as_deref(),
            Some("Phone")
        );
        assert_eq!(selected.borrow().as_deref(), Some("device-Phone"));

        let renamed = backend::Device {
            id: backend::DeviceId("device-Phone".into()),
            name: "Renamed phone".into(),
            ..backend_device("Phone")
        };
        assert_eq!(
            prune_selected_device(&selected, &[renamed]).as_deref(),
            Some("Renamed phone")
        );
        assert_eq!(selected.borrow().as_deref(), Some("device-Phone"));

        assert_eq!(
            prune_selected_device(&selected, &[backend_device("Laptop")]),
            None
        );
        assert_eq!(selected.borrow().as_deref(), None);
    }

    #[test]
    fn selected_device_is_cleared_when_discovery_becomes_unavailable() {
        let selected = Rc::new(RefCell::new(Some("device-Phone".to_string())));
        assert!(clear_selected_device(&selected));
        assert_eq!(selected.borrow().as_deref(), None);
        assert!(!clear_selected_device(&selected));
    }

    #[test]
    fn backend_limitations_do_not_overstate_unverified_features() {
        assert!(VALENT_LIMITATIONS.contains("not verified"));
        assert!(VALENT_LIMITATIONS.contains("not implemented"));
        assert!(VALENT_LIMITATIONS.contains("no support is claimed"));
    }

    #[test]
    fn capability_report_comes_from_the_shared_backend_snapshot() {
        let snapshot = valent::backend_snapshot(backend::BackendState::Stopped);
        let report = backend_capabilities_text(&snapshot);

        assert!(report.contains("Availability: stopped"));
        assert!(report.contains("Discovery: experimental"));
        assert!(report.contains("Pairing: unavailable in Viloja"));
        assert!(report.contains("Send files: unavailable"));
        assert!(report.contains("Receive files: unavailable"));
        assert!(report.contains("Transfer cancellation: unavailable"));
        assert!(report.contains("Router-free transfer: unknown"));
    }

    #[test]
    fn valent_device_parser_extracts_devices_and_ignores_other_interfaces() {
        let object_manager_type = glib::VariantTy::new("(a{oa{sa{sv}}})")
            .expect("ObjectManager reply type should be valid");
        let reply = glib::Variant::parse(
            Some(object_manager_type),
            "({
                objectpath '/ca/andyholmes/Valent/devices/zeta': {
                    'ca.andyholmes.Valent.Device': {
                        'Id': <'zeta-id'>,
                        'Name': <'Zeta phone'>,
                        'IconName': <'phone-symbolic'>,
                        'State': <uint32 2>
                    },
                    'org.example.Unrelated': {
                        'Name': <'Must be ignored'>
                    }
                },
                objectpath '/ca/andyholmes/Valent/devices/alpha': {
                    'ca.andyholmes.Valent.Device': {
                        'Id': <'alpha-id'>,
                        'Name': <'Alpha phone'>,
                        'State': <uint32 1>
                    }
                }
            },)",
        )
        .expect("fixture should match the ObjectManager reply signature");

        let devices = parse_valent_devices(&reply);

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "alpha-id");
        assert_eq!(devices[0].name, "Alpha phone");
        assert_eq!(devices[0].state, 1);
        assert_eq!(devices[1].id, "zeta-id");
        assert_eq!(devices[1].name, "Zeta phone");
        assert_eq!(devices[1].icon_name, "phone-symbolic");
        assert_eq!(devices[1].state, 2);
    }

    #[test]
    fn valent_device_parser_falls_back_from_blank_or_mistyped_metadata() {
        let object_manager_type = glib::VariantTy::new("(a{oa{sa{sv}}})")
            .expect("ObjectManager reply type should be valid");
        let reply = glib::Variant::parse(
            Some(object_manager_type),
            "({
                objectpath '/ca/andyholmes/Valent/devices/fallback': {
                    'ca.andyholmes.Valent.Device': {
                        'Id': <''>,
                        'Name': <'   '>,
                        'State': <uint32 0>
                    }
                },
                objectpath '/ca/andyholmes/Valent/devices/valid': {
                    'ca.andyholmes.Valent.Device': {
                        'Id': <' valid-id '>,
                        'Name': <'  Android phone  '>,
                        'State': <uint32 1>
                    }
                }
            },)",
        )
        .expect("fixture should match the ObjectManager reply signature");

        let devices = parse_valent_devices(&reply);
        let fallback = devices
            .iter()
            .find(|device| device.id.ends_with("/fallback"))
            .expect("fallback device should be retained");
        assert_eq!(fallback.id, "/ca/andyholmes/Valent/devices/fallback");
        assert_eq!(fallback.name, "Unnamed device");

        let valid = devices
            .iter()
            .find(|device| device.id == "valid-id")
            .expect("valid device should be retained");
        assert_eq!(valid.name, "Android phone");
    }

    #[test]
    fn connection_labels_preserve_each_shared_pairing_state() {
        let mut device = backend_device("Phone");

        device.pairing_state = backend::PairingState::Unpaired;
        device.paired = false;
        assert_eq!(connection_label(&device), "Not connected");

        device.pairing_state = backend::PairingState::IncomingRequest;
        assert_eq!(connection_label(&device), "Incoming pairing request");

        device.pairing_state = backend::PairingState::OutgoingRequest;
        assert_eq!(connection_label(&device), "Outgoing pairing request");

        device.pairing_state = backend::PairingState::Paired;
        device.reachable = true;
        assert_eq!(connection_label(&device), "Connected");

        device.reachable = false;
        assert_eq!(connection_label(&device), "Paired · offline");
    }
}
