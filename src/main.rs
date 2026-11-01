mod backend;
mod diagnostics;

use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use diagnostics::{DiagnosticLevel, DiagnosticEvent};

const VALENT_BUS_NAME: &str = "ca.andyholmes.Valent";
const VALENT_OBJECT_PATH: &str = "/ca/andyholmes/Valent";
const VALENT_DEVICE_INTERFACE: &str = "ca.andyholmes.Valent.Device";
const OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";

#[derive(Debug, Clone)]
struct ValentDeviceInfo {
    id: String,
    name: String,
    state: u32,
}

impl ValentDeviceInfo {
    fn connection_label(&self) -> &'static str {
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

#[derive(Default)]
struct RefreshState {
    request_id: u64,
    cancellable: Option<gio::Cancellable>,
    closed: bool,
}

fn begin_refresh(state: &Rc<RefCell<RefreshState>>) -> (u64, gio::Cancellable) {
    let mut state = state.borrow_mut();
    if let Some(previous) = state.cancellable.take() {
        diagnostics::record(
            DiagnosticLevel::Debug,
            DiagnosticEvent::RefreshSuperseded,
        );
        previous.cancel();
    }

    state.request_id = state.request_id.wrapping_add(1);
    let request_id = state.request_id;
    let cancellable = gio::Cancellable::new();
    state.cancellable = Some(cancellable.clone());
    (request_id, cancellable)
}

fn is_current_refresh(state: &Rc<RefCell<RefreshState>>, request_id: u64) -> bool {
    let state = state.borrow();
    !state.closed && state.request_id == request_id
}

fn finish_refresh(state: &Rc<RefCell<RefreshState>>, request_id: u64) -> bool {
    let mut state = state.borrow_mut();
    if state.closed || state.request_id != request_id {
        return false;
    }

    state.cancellable = None;
    true
}

fn invalidate_refresh(state: &Rc<RefCell<RefreshState>>) {
    let mut state = state.borrow_mut();
    state.request_id = state.request_id.wrapping_add(1);
    if let Some(cancellable) = state.cancellable.take() {
        cancellable.cancel();
    }
}

fn close_refresh(state: &Rc<RefCell<RefreshState>>) {
    let mut state = state.borrow_mut();
    state.closed = true;
    state.request_id = state.request_id.wrapping_add(1);
    if let Some(cancellable) = state.cancellable.take() {
        cancellable.cancel();
    }
}

fn parse_valent_devices(reply: &glib::Variant) -> Vec<ValentDeviceInfo> {
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
                    "State" => state = value.get::<u32>().unwrap_or(state),
                    _ => {}
                }
            }

            devices.push(ValentDeviceInfo { id, name, state });
        }
    }

    devices.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    devices
}

fn device_result_status(result: &Result<Vec<ValentDeviceInfo>, String>) -> String {
    match result {
        Ok(devices) if devices.is_empty() => {
            "Valent is available, but it has no known devices yet. Open Valent and make sure device discovery is enabled.".into()
        }
        Ok(devices) => format!(
            "Found {} device{} through Valent.",
            devices.len(),
            if devices.len() == 1 { "" } else { "s" }
        ),
        Err(error) => format!(
            "Valent is unavailable or its device manager cannot be reached. Install Valent if needed, start it, enable device discovery, then refresh. Details: {error}"
        ),
    }
}

fn prune_selected_device(
    selected_device: &Rc<RefCell<Option<String>>>,
    devices: &[ValentDeviceInfo],
) -> bool {
    let mut selected = selected_device.borrow_mut();
    if selected.as_ref().is_some_and(|id| devices.iter().any(|device| &device.id == id)) {
        true
    } else {
        *selected = None;
        false
    }
}

fn clear_selected_device(selected_device: &Rc<RefCell<Option<String>>>) -> bool {
    selected_device.borrow_mut().take().is_some()
}

fn clear_device_list(device_list: &gtk::Box) {
    while let Some(child) = device_list.first_child() {
        device_list.remove(&child);
    }
}

type DeviceSignalSubscriptions = Rc<RefCell<Option<(gio::DBusConnection, Vec<gio::SignalSubscriptionId>)>>>;

fn clear_device_signal_subscriptions(subscriptions: &DeviceSignalSubscriptions) {
    if let Some((connection, ids)) = subscriptions.borrow_mut().take() {
        for id in ids {
            connection.signal_unsubscribe(id);
        }
    }
}

fn install_device_signal_subscriptions(
    connection: &gio::DBusConnection,
    window: &adw::ApplicationWindow,
    status: &gtk::Label,
    device_list: &gtk::Box,
    refresh_button: &gtk::Button,
    state: &Rc<RefCell<RefreshState>>,
    subscriptions: &DeviceSignalSubscriptions,
    selected_device: &Rc<RefCell<Option<String>>>,
    selection_status: &gtk::Label,
) {
    clear_device_signal_subscriptions(subscriptions);

    let window_weak = window.downgrade();
    let status = status.clone();
    let device_list = device_list.clone();
    let refresh_button = refresh_button.clone();
    let state = state.clone();
    let subscriptions_for_refresh = subscriptions.clone();
    let selected_device_for_refresh = selected_device.clone();
    let selection_status_for_refresh = selection_status.clone();
    let refresh = Rc::new(move || {
        if let Some(window) = window_weak.upgrade() {
            refresh_valent_devices(
                &window,
                &status,
                &device_list,
                &refresh_button,
                &state,
                &subscriptions_for_refresh,
                &selected_device_for_refresh,
                &selection_status_for_refresh,
            );
        }
    });

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

    *subscriptions.borrow_mut() = Some((connection.clone(), vec![
        properties_id,
        added_id,
        removed_id,
    ]));
}

fn refresh_valent_devices(
    window: &adw::ApplicationWindow,
    status: &gtk::Label,
    device_list: &gtk::Box,
    refresh_button: &gtk::Button,
    state: &Rc<RefCell<RefreshState>>,
    subscriptions: &DeviceSignalSubscriptions,
    selected_device: &Rc<RefCell<Option<String>>>,
    selection_status: &gtk::Label,
) {
    let (request_id, cancellable) = begin_refresh(state);
    diagnostics::record(DiagnosticLevel::Info, DiagnosticEvent::BackendRefreshStarted);
    status.set_text("Refreshing devices…");
    refresh_button.set_sensitive(false);
    clear_device_list(device_list);

    let window_weak = window.downgrade();
    let status_for_bus = status.clone();
    let list_for_bus = device_list.clone();
    let button_for_bus = refresh_button.clone();
    let state_for_bus = state.clone();
    let subscriptions = subscriptions.clone();
    let selected_device = selected_device.clone();
    let selection_status = selection_status.clone();

    let bus_cancellable = cancellable.clone();
    gio::bus_get(
        gio::BusType::Session,
        Some(&bus_cancellable),
        move |connection_result| {
            if window_weak.upgrade().is_none()
                || !is_current_refresh(&state_for_bus, request_id)
            {
                return;
            }

            let connection = match connection_result {
                Ok(connection) => connection,
                Err(error) => {
                    diagnostics::record(
                        DiagnosticLevel::Error,
                        DiagnosticEvent::SessionBusUnavailable,
                    );
                    if finish_refresh(&state_for_bus, request_id) {
                        status_for_bus.set_text(&format!(
                            "Could not connect to the session bus: {error}"
                        ));
                        button_for_bus.set_sensitive(true);
                    }
                    return;
                }
            };

            install_device_signal_subscriptions(
                &connection,
                &window_weak.upgrade().expect("window was checked above"),
                &status_for_bus,
                &list_for_bus,
                &button_for_bus,
                &state_for_bus,
                &subscriptions,
                &selected_device,
                &selection_status,
            );

            let reply_type = glib::VariantTy::new("(a{oa{sa{sv}}})")
                .expect("the ObjectManager reply signature is valid");
            let state_for_call = state_for_bus.clone();
            let window_for_call = window_weak.clone();
            let status_for_call = status_for_bus.clone();
            let list_for_call = list_for_bus.clone();
            let button_for_call = button_for_bus.clone();
            let selected_device_for_call = selected_device.clone();
            let selection_status_for_call = selection_status.clone();

            connection.call(
                Some(VALENT_BUS_NAME),
                VALENT_OBJECT_PATH,
                OBJECT_MANAGER_INTERFACE,
                "GetManagedObjects",
                None,
                Some(reply_type),
                gio::DBusCallFlags::NONE,
                3000,
                Some(&cancellable),
                move |reply_result| {
                    if window_for_call.upgrade().is_none()
                        || !finish_refresh(&state_for_call, request_id)
                    {
                        return;
                    }

                    button_for_call.set_sensitive(true);
                    let result = reply_result
                        .map(|reply| parse_valent_devices(&reply))
                        .map_err(|error| {
                            format!(
                                "Valent is unavailable or does not expose its device manager: {error}"
                            )
                        });
                    status_for_call.set_text(&device_result_status(&result));

                    match &result {
                        Ok(devices) => diagnostics::record(DiagnosticLevel::Info, DiagnosticEvent::DiscoveryCompleted { device_count: devices.len() }),
                        Err(_) => diagnostics::record(DiagnosticLevel::Error, DiagnosticEvent::BackendDiscoveryFailed),
                    }

                    if let Ok(devices) = result {
                        if !prune_selected_device(&selected_device_for_call, &devices) {
                            selection_status_for_call.set_text("No device selected.");
                        }
                        for device in devices {
                            let card = make_card(&device.name, device.connection_label());
                            card.set_tooltip_text(Some(&format!(
                                "Valent device ID: {}",
                                device.id
                            )));
                            let row = gtk::Button::new();
                            row.add_css_class("flat");
                            row.set_hexpand(true);
                            row.set_child(Some(&card));
                            let selected = selected_device_for_call.clone();
                            let selection_status = selection_status_for_call.clone();
                            let device_id = device.id.clone();
                            let device_name = device.name.clone();
                            row.connect_clicked(move |_| {
                                *selected.borrow_mut() = Some(device_id.clone());
                                selection_status.set_text(&format!("Selected destination: {device_name}"));
                            });
                            list_for_call.append(&row);
                        }
                    } else {
                        clear_selected_device(&selected_device_for_call);
                        selection_status_for_call.set_text("No device selected.");
                    }
                },
            );
        },
    );
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

const VALENT_CAPABILITIES: &str = "Discovery: experimental session-D-Bus query (availability is shown on Nearby after refresh). Pairing: delegated to Valent; Viloja does not start or manage pairing. Send files: unavailable in Viloja. Receive files: unavailable in Viloja. Transfer cancellation: unavailable. Router-free transfer: unknown and not supported as a Viloja feature.";

fn backend_capabilities_card() -> gtk::Box {
    make_card("Valent backend capabilities", VALENT_CAPABILITIES)
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
        "Find devices known to Valent, the GNOME implementation of KDE Connect.",
    );
    let discovery_status = make_label(
        "Use Refresh to query Valent over the session D-Bus.",
        "",
    );
    let refresh_button = gtk::Button::with_label("Refresh devices");
    refresh_button.add_css_class("suggested-action");
    refresh_button.set_halign(gtk::Align::Start);
    let device_list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let selected_device: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selection_status = make_label("No device selected.", "dim-label");
    nearby.append(&refresh_button);
    nearby.append(&discovery_status);
    nearby.append(&device_list);
    nearby.append(&selection_status);
    nearby.append(&backend_limitations_card());
    nearby.append(&backend_capabilities_card());

    let refresh_state = Rc::new(RefCell::new(RefreshState::default()));
    let device_signal_subscriptions: DeviceSignalSubscriptions = Rc::new(RefCell::new(None));
    let window_for_refresh = window.clone();
    let status_for_refresh = discovery_status.clone();
    let list_for_refresh = device_list.clone();
    let button_for_refresh = refresh_button.clone();
    let state_for_refresh = refresh_state.clone();
    let subscriptions_for_refresh = device_signal_subscriptions.clone();
    let selected_device_for_refresh = selected_device.clone();
    let selection_status_for_refresh = selection_status.clone();
    refresh_button.connect_clicked(move |_| {
        refresh_valent_devices(
            &window_for_refresh,
            &status_for_refresh,
            &list_for_refresh,
            &button_for_refresh,
            &state_for_refresh,
            &subscriptions_for_refresh,
            &selected_device_for_refresh,
            &selection_status_for_refresh,
        );
    });

    let state_for_close = refresh_state.clone();
    let subscriptions_for_close = device_signal_subscriptions.clone();
    let watcher_for_close = Rc::new(RefCell::new(None));
    let watcher_for_close_request = watcher_for_close.clone();
    window.connect_close_request(move |_| {
        close_refresh(&state_for_close);
        clear_device_signal_subscriptions(&subscriptions_for_close);
        if let Some(watcher_id) = watcher_for_close_request.borrow_mut().take() {
            gio::bus_unwatch_name(watcher_id);
        }
        glib::Propagation::Proceed
    });

    let window_for_watcher = window.downgrade();
    let window_for_appearance = window.clone();
    let status_for_appearance = discovery_status.clone();
    let list_for_appearance = device_list.clone();
    let button_for_appearance = refresh_button.clone();
    let state_for_appearance = refresh_state.clone();
    let subscriptions_for_appearance = device_signal_subscriptions.clone();
    let selected_device_for_appearance = selected_device.clone();
    let selection_status_for_appearance = selection_status.clone();
    let status_for_disappearance = discovery_status.clone();
    let list_for_disappearance = device_list.clone();
    let button_for_disappearance = refresh_button.clone();
    let state_for_disappearance = refresh_state.clone();
    let subscriptions_for_disappearance = device_signal_subscriptions.clone();
    let selected_device_for_disappearance = selected_device.clone();
    let selection_status_for_disappearance = selection_status.clone();
    let watcher_id = gio::bus_watch_name(
        gio::BusType::Session,
        VALENT_BUS_NAME,
        gio::BusNameWatcherFlags::NONE,
        move |_, _, _| {
            refresh_valent_devices(
                &window_for_appearance,
                &status_for_appearance,
                &list_for_appearance,
                &button_for_appearance,
                &state_for_appearance,
                &subscriptions_for_appearance,
                &selected_device_for_appearance,
                &selection_status_for_appearance,
            );
        },
        move |_, _| {
            if window_for_watcher.upgrade().is_none() {
                return;
            }
            invalidate_refresh(&state_for_disappearance);
            clear_device_signal_subscriptions(&subscriptions_for_disappearance);
            clear_selected_device(&selected_device_for_disappearance);
            selection_status_for_disappearance.set_text("No device selected.");
            diagnostics::record(DiagnosticLevel::Error, DiagnosticEvent::BackendUnavailable);
            clear_device_list(&list_for_disappearance);
            status_for_disappearance.set_text(
                "Valent is not running. Start Valent to restore device discovery; Viloja will retry automatically when it appears.",
            );
            button_for_disappearance.set_sensitive(true);
        },
    );
    *watcher_for_close.borrow_mut() = Some(watcher_id);

    stack.add_titled(&make_scrolled_page(&nearby), Some("nearby"), "Nearby");

    let send = make_page(
        "Send files",
        "Choose files to prepare a transfer. Viloja cannot send them yet.",
    );
    send.append(&backend_limitations_card());
    let selection_status = make_label("No files selected.", "");
    let choose_button = gtk::Button::with_label("Choose files…");
    choose_button.add_css_class("suggested-action");
    send.append(&make_card(
        "Select files",
        "You can select multiple files and review the selection here. Nothing is sent by this prototype.",
    ));
    send.append(&choose_button);
    send.append(&selection_status);
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
    settings.append(&backend_capabilities_card());
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
    let status_label = selection_status.clone();
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

    fn device(name: &str) -> ValentDeviceInfo {
        ValentDeviceInfo {
            id: format!("device-{name}"),
            name: name.into(),
            state: 1,
        }
    }

    #[test]
    fn selected_device_is_retained_only_while_it_remains_discovered() {
        let selected = Rc::new(RefCell::new(Some("device-Phone".to_string())));
        assert!(prune_selected_device(&selected, &[device("Phone")]));
        assert_eq!(selected.borrow().as_deref(), Some("device-Phone"));

        assert!(!prune_selected_device(&selected, &[device("Laptop")]));
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
    fn valent_connection_labels_cover_each_documented_state() {
        assert_eq!(device("Connected").connection_label(), "Connected");
        assert_eq!(
            ValentDeviceInfo { state: 2, ..device("Paired") }.connection_label(),
            "Paired · offline"
        );
        assert_eq!(
            ValentDeviceInfo { state: 4, ..device("Incoming") }.connection_label(),
            "Incoming pairing request"
        );
        assert_eq!(
            ValentDeviceInfo { state: 8, ..device("Outgoing") }.connection_label(),
            "Outgoing pairing request"
        );
        assert_eq!(
            ValentDeviceInfo { state: 0, ..device("Unknown") }.connection_label(),
            "Not connected"
        );
    }

    #[test]
    fn connected_state_takes_precedence_over_other_state_bits() {
        assert_eq!(
            ValentDeviceInfo { state: 1 | 2 | 4 | 8, ..device("Combined") }
                .connection_label(),
            "Connected"
        );
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
    fn backend_limitations_do_not_overstate_unverified_features() {
        assert!(VALENT_LIMITATIONS.contains("not verified"));
        assert!(VALENT_LIMITATIONS.contains("not implemented"));
        assert!(VALENT_LIMITATIONS.contains("no support is claimed"));
    }

    #[test]
    fn valent_capability_report_marks_unimplemented_actions_unavailable() {
        assert!(VALENT_CAPABILITIES.contains("Discovery: experimental"));
        assert!(VALENT_CAPABILITIES.contains("Pairing: delegated to Valent"));
        assert!(VALENT_CAPABILITIES.contains("Send files: unavailable"));
        assert!(VALENT_CAPABILITIES.contains("Receive files: unavailable"));
        assert!(VALENT_CAPABILITIES.contains("Transfer cancellation: unavailable"));
        assert!(VALENT_CAPABILITIES.contains("Router-free transfer: unknown"));
    }

    #[test]
    fn refresh_status_distinguishes_empty_and_successful_results() {
        assert!(device_result_status(&Ok(Vec::new())).contains("no known devices"));
        assert_eq!(
            device_result_status(&Ok(vec![device("Phone")])),
            "Found 1 device through Valent."
        );
        assert_eq!(
            device_result_status(&Ok(vec![device("Phone"), device("Laptop")])),
            "Found 2 devices through Valent."
        );
    }

    #[test]
    fn refresh_status_preserves_unavailable_and_timeout_errors() {
        let unavailable = device_result_status(&Err("service has no owner".into()));
        assert!(unavailable.contains("Valent is unavailable"));
        assert!(unavailable.contains("Install Valent"));
        assert!(unavailable.contains("enable device discovery"));
        assert!(unavailable.contains("service has no owner"));

        let timeout = device_result_status(&Err("D-Bus call timed out".into()));
        assert!(timeout.contains("Valent is unavailable"));
        assert!(timeout.contains("D-Bus call timed out"));
    }

    #[test]
    fn starting_a_new_refresh_cancels_and_invalidates_the_previous_one() {
        let state = Rc::new(RefCell::new(RefreshState::default()));
        let (first_id, first_cancellable) = begin_refresh(&state);
        let (second_id, _) = begin_refresh(&state);

        assert!(first_cancellable.is_cancelled());
        assert!(second_id > first_id);
        assert!(!is_current_refresh(&state, first_id));
        assert!(is_current_refresh(&state, second_id));
        assert!(!finish_refresh(&state, first_id));
        assert!(finish_refresh(&state, second_id));
    }

    #[test]
    fn backend_disappearance_invalidates_refresh_and_allows_recovery() {
        let state = Rc::new(RefCell::new(RefreshState::default()));
        let (stale_id, stale_cancellable) = begin_refresh(&state);

        invalidate_refresh(&state);

        assert!(stale_cancellable.is_cancelled());
        assert!(!is_current_refresh(&state, stale_id));
        assert!(!finish_refresh(&state, stale_id));

        let (recovered_id, _) = begin_refresh(&state);
        assert!(is_current_refresh(&state, recovered_id));
        assert!(finish_refresh(&state, recovered_id));
    }

    #[test]
    fn closing_the_window_cancels_and_invalidates_an_in_flight_refresh() {
        let state = Rc::new(RefCell::new(RefreshState::default()));
        let (request_id, cancellable) = begin_refresh(&state);

        close_refresh(&state);

        assert!(cancellable.is_cancelled());
        assert!(!is_current_refresh(&state, request_id));
        assert!(!finish_refresh(&state, request_id));
    }
}
