mod backend;
mod diagnostics;
mod valent;

use adw::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;
use diagnostics::{DiagnosticEvent, DiagnosticLevel};

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

fn clear_device_list(device_list: &gtk::Box) {
    while let Some(child) = device_list.first_child() {
        device_list.remove(&child);
    }
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
    device_list: &gtk::Box,
    known_devices: &HashMap<backend::DeviceId, backend::Device>,
    selected_device: &Rc<RefCell<Option<String>>>,
    selection_status: &gtk::Label,
) {
    clear_device_list(device_list);
    let mut devices = known_devices.values().cloned().collect::<Vec<_>>();
    devices.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    match prune_selected_device(selected_device, &devices) {
        Some(name) => selection_status.set_text(&format!("Selected destination: {name}")),
        None => selection_status.set_text("No device selected."),
    }

    for device in devices {
        let card = make_card(&device.name, connection_label(&device));
        let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let icon_name = match device.device_type {
            backend::DeviceType::Phone => "phone-symbolic",
            backend::DeviceType::Tablet => "tablet-symbolic",
            backend::DeviceType::Computer | backend::DeviceType::Unknown => "computer-symbolic",
        };
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(32);
        row_content.append(&icon);
        row_content.append(&card);

        let row = gtk::Button::new();
        row.add_css_class("flat");
        row.set_hexpand(true);
        row.set_child(Some(&row_content));
        let selected = selected_device.clone();
        let selection_status = selection_status.clone();
        let device_id = device.id.0.clone();
        let device_name = device.name.clone();
        row.connect_clicked(move |_| {
            *selected.borrow_mut() = Some(device_id.clone());
            selection_status.set_text(&format!("Selected destination: {device_name}"));
        });
        device_list.append(&row);
    }
}

fn start_backend(
    backend: Rc<RefCell<Box<dyn backend::SharingBackend>>>,
    events: std::sync::mpsc::Sender<backend::BackendEvent>,
    refresh_button: gtk::Button,
) {
    diagnostics::record(DiagnosticLevel::Info, DiagnosticEvent::BackendRefreshStarted);
    glib::MainContext::default().spawn_local(async move {
        let mut backend = backend.borrow_mut();
        let result = backend.start(events).await;
        drop(backend);
        refresh_button.set_sensitive(true);
        if result.is_err() {
            diagnostics::record(DiagnosticLevel::Error, DiagnosticEvent::BackendDiscoveryFailed);
        }
    });
}

fn make_label(text: &str, css_class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    if !css_class.is_empty() {
        label.add_css_class(css_class);
    }
    label
}

fn make_page(title: &str, description: &str) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.set_margin_top(28);
    page.set_margin_bottom(28);
    page.set_margin_start(28);
    page.set_margin_end(28);
    page.append(&make_label(title, "title-1"));
    page.append(&make_label(description, "dim-label"));
    page
}

fn make_card(title: &str, body: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("card");
    card.set_margin_top(6);
    card.set_margin_bottom(6);
    card.set_margin_start(12);
    card.set_margin_end(12);
    card.set_hexpand(true);
    card.append(&make_label(title, "heading"));
    card.append(&make_label(body, ""));
    card
}

const VALENT_LIMITATIONS: &str = "Device discovery: experimental session-D-Bus query; not verified against an installed Valent/GNOME session. Pairing: handled in Valent, not managed by Viloja. Sending and receiving files: not implemented. Router-free, hotspot, Wi-Fi Direct, and Bluetooth transfers: not verified; no support is claimed.";

fn backend_limitations_card() -> gtk::Box {
    make_card("Current backend: Valent (experimental)", VALENT_LIMITATIONS)
}

fn backend_capabilities_text(snapshot: &backend::BackendSnapshot) -> String {
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
    let sending = if capabilities.send_files { "available" } else { "unavailable" };
    let receiving = if capabilities.receive_files { "available" } else { "unavailable" };
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
        "Discovery: {discovery}. Pairing: {pairing}. Send files: {sending}. Receive files: {receiving}. Transfer cancellation: {cancellation}. Router-free transfer: {router_free}."
    )
}

fn backend_capabilities_card(snapshot: &backend::BackendSnapshot) -> gtk::Box {
    let body = backend_capabilities_text(snapshot);
    make_card(
        &format!("{} backend capabilities", snapshot.display_name),
        &body,
    )
}

fn make_scrolled_page(content: &gtk::Box) -> gtk::ScrolledWindow {
    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_child(Some(content));
    scrolled
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Viloja")
        .default_width(1000)
        .default_height(700)
        .build();

    let backend: Rc<RefCell<Box<dyn backend::SharingBackend>>> =
        Rc::new(RefCell::new(valent::create_backend()));
    let backend_snapshot = backend.borrow().snapshot();
    let (event_sender, event_receiver) = mpsc::channel::<backend::BackendEvent>();
    let known_devices: Rc<RefCell<HashMap<backend::DeviceId, backend::Device>>> =
        Rc::new(RefCell::new(HashMap::new()));

    let header = adw::HeaderBar::new();
    let app_title = gtk::Label::new(Some("Viloja"));
    app_title.add_css_class("title");
    header.set_title_widget(Some(&app_title));

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(150);
    stack.set_vexpand(true);
    stack.set_hexpand(true);

    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_width_request(190);

    let nearby = make_page(
        "Nearby",
        "Find devices known to the active sharing backend.",
    );
    let discovery_status = make_label("Starting device discovery…", "");
    let refresh_button = gtk::Button::with_label("Refresh devices");
    refresh_button.add_css_class("suggested-action");
    refresh_button.set_halign(gtk::Align::Start);
    refresh_button.set_sensitive(false);
    let device_list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let selected_device: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selection_status = make_label("No device selected.", "dim-label");
    nearby.append(&refresh_button);
    nearby.append(&discovery_status);
    nearby.append(&device_list);
    nearby.append(&selection_status);
    nearby.append(&backend_limitations_card());
    nearby.append(&backend_capabilities_card(&backend_snapshot));

    let backend_for_refresh = backend.clone();
    let events_for_refresh = event_sender.clone();
    let button_for_refresh = refresh_button.clone();
    let devices_for_refresh = known_devices.clone();
    let list_for_refresh = device_list.clone();
    let selected_for_refresh = selected_device.clone();
    let selection_for_refresh = selection_status.clone();
    let status_for_refresh = discovery_status.clone();
    refresh_button.connect_clicked(move |_| {
        devices_for_refresh.borrow_mut().clear();
        clear_device_list(&list_for_refresh);
        clear_selected_device(&selected_for_refresh);
        selection_for_refresh.set_text("No device selected.");
        status_for_refresh.set_text("Refreshing devices…");
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
    let selected_for_events = selected_device.clone();
    let selection_for_events = selection_status.clone();
    let status_for_events = discovery_status.clone();
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
                        status_for_events.set_text("Connecting to the sharing backend…");
                    }
                    backend::BackendState::Running => {
                        let count = devices_for_events.borrow().len();
                        if count == 0 {
                            status_for_events.set_text(
                                "The backend is available, but it has no known devices yet.",
                            );
                        } else {
                            status_for_events.set_text(&format!(
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
                            &devices_for_events.borrow(),
                            &selected_for_events,
                            &selection_for_events,
                        );
                        status_for_events.set_text(
                            "Valent is unavailable. Start Valent to restore discovery; Viloja will retry when it appears.",
                        );
                        diagnostics::record(
                            DiagnosticLevel::Error,
                            DiagnosticEvent::BackendUnavailable,
                        );
                    }
                    backend::BackendState::Stopped => {
                        status_for_events.set_text("Device discovery is stopped.");
                    }
                    backend::BackendState::Stopping => {
                        status_for_events.set_text("Stopping device discovery…");
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
                        &devices,
                        &selected_for_events,
                        &selection_for_events,
                    );
                    status_for_events.set_text(&format!(
                        "Found {count} device{}.",
                        if count == 1 { "" } else { "s" }
                    ));
                    diagnostics::record(
                        DiagnosticLevel::Info,
                        DiagnosticEvent::DiscoveryCompleted { device_count: count },
                    );
                }
                backend::BackendEvent::DeviceUpdated(device) => {
                    devices_for_events
                        .borrow_mut()
                        .insert(device.id.clone(), device);
                    let devices = devices_for_events.borrow();
                    render_device_list(
                        &list_for_events,
                        &devices,
                        &selected_for_events,
                        &selection_for_events,
                    );
                }
                backend::BackendEvent::DeviceLost(device_id) => {
                    devices_for_events.borrow_mut().remove(&device_id);
                    render_device_list(
                        &list_for_events,
                        &devices_for_events.borrow(),
                        &selected_for_events,
                        &selection_for_events,
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
        let backend = backend_for_close.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Ok(mut backend) = backend.try_borrow_mut() {
                let _ = backend.stop().await;
            }
        });
        glib::Propagation::Proceed
    });

    stack.add_titled(&make_scrolled_page(&nearby), Some("nearby"), "Nearby");

    let send = make_page(
        "Send files",
        "Choose files to prepare a transfer. Viloja cannot send them yet.",
    );
    send.append(&backend_limitations_card());
    let send_selection_status = make_label("No files selected.", "");
    let choose_button = gtk::Button::with_label("Choose files…");
    choose_button.add_css_class("suggested-action");
    send.append(&make_card(
        "Select files",
        "You can select multiple files and review the selection here. Nothing is sent by this prototype.",
    ));
    send.append(&choose_button);
    send.append(&send_selection_status);
    send.append(&make_label(
        "Transfer will remain disabled until a real device action is connected.",
        "dim-label",
    ));
    stack.add_titled(&make_scrolled_page(&send), Some("send"), "Send files");

    let transfers = make_page(
        "Transfers",
        "Transfer monitoring is unavailable until a real backend transfer flow is integrated.",
    );
    transfers.append(&backend_limitations_card());
    transfers.append(&make_card(
        "No transfers yet",
        "Transfer progress, cancellation, completion, and error states will appear once transfer events are integrated.",
    ));
    stack.add_titled(&make_scrolled_page(&transfers), Some("transfers"), "Transfers");

    let history = make_page(
        "History",
        "Viloja does not yet store transfer history because transfers are not integrated.",
    );
    history.append(&backend_limitations_card());
    history.append(&make_card(
        "History is empty",
        "Viloja does not record transfer history in this prototype.",
    ));
    stack.add_titled(&make_scrolled_page(&history), Some("history"), "History");

    let settings = make_page(
        "Settings",
        "Backend availability and transfer preferences will be configurable when those integrations exist.",
    );
    settings.append(&backend_limitations_card());
    settings.append(&backend_capabilities_card(&backend_snapshot));
    settings.append(&make_card(
        "Receiving",
        "Receiving preferences will be available when the backend exposes its supported options.",
    ));
    settings.append(&make_card(
        "Downloads",
        "The download destination has not been configured yet.",
    ));
    settings.append(&make_card(
        "Privacy",
        "Device visibility and trust controls will be based on the active backend's real capabilities.",
    ));

    settings.append(&make_label("Diagnostics", "title-2"));
    settings.append(&make_label(
        "Diagnostic entries use fixed event names and numeric counts only. Device names, addresses, tokens, paths, and raw backend error strings are never added to this log.",
        "dim-label",
    ));
    let verbosity = gtk::DropDown::from_strings(&[
        "Errors only",
        "Normal",
        "Verbose",
        "Off",
    ]);
    verbosity.set_selected(diagnostics::verbosity_index());
    verbosity.connect_selected_notify(|dropdown| {
        diagnostics::set_verbosity(dropdown.selected());
    });
    settings.append(&make_label("Log verbosity", "heading"));
    settings.append(&verbosity);

    let diagnostic_view = gtk::TextView::new();
    diagnostic_view.set_editable(false);
    diagnostic_view.set_cursor_visible(false);
    diagnostic_view.set_monospace(true);
    diagnostic_view.set_wrap_mode(gtk::WrapMode::WordChar);
    diagnostic_view.buffer().set_text(&diagnostics::export());
    let diagnostic_scroll = gtk::ScrolledWindow::new();
    diagnostic_scroll.set_min_content_height(180);
    diagnostic_scroll.set_child(Some(&diagnostic_view));
    settings.append(&diagnostic_scroll);

    let refresh_diagnostics = gtk::Button::with_label("Refresh diagnostics");
    refresh_diagnostics.set_halign(gtk::Align::Start);
    let refresh_buffer = diagnostic_view.buffer();
    refresh_diagnostics.connect_clicked(move |_| {
        refresh_buffer.set_text(&diagnostics::export());
    });
    settings.append(&refresh_diagnostics);

    let copy_diagnostics = gtk::Button::with_label("Copy diagnostics");
    copy_diagnostics.set_halign(gtk::Align::Start);
    let diagnostic_buffer = diagnostic_view.buffer();
    let diagnostic_display = gtk::prelude::WidgetExt::display(&window);
    copy_diagnostics.connect_clicked(move |_| {
        let exported = diagnostics::export();
        diagnostic_buffer.set_text(&exported);
        diagnostic_display.clipboard().set_text(&exported);
    });
    settings.append(&copy_diagnostics);

    let clear_diagnostics = gtk::Button::with_label("Clear diagnostics");
    clear_diagnostics.set_halign(gtk::Align::Start);
    let clear_buffer = diagnostic_view.buffer();
    clear_diagnostics.connect_clicked(move |_| {
        diagnostics::clear();
        clear_buffer.set_text("");
    });
    settings.append(&clear_diagnostics);
    stack.add_titled(&make_scrolled_page(&settings), Some("settings"), "Settings");

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.append(&sidebar);
    body.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    body.append(&stack);

    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    layout.append(&header);
    layout.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    layout.append(&body);
    window.set_content(Some(&layout));

    let chooser_window = window.clone();
    let status_label = send_selection_status.clone();
    choose_button.connect_clicked(move |_| {
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
                    status.set_text("1 file selected. Ready for a backend integration.");
                } else {
                    status.set_text(&format!(
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
    use crate::valent::{parse_valent_devices, ValentDeviceInfo};

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

        assert_eq!(prune_selected_device(&selected, &[backend_device("Laptop")]), None);
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
