mod backend;

use adw::prelude::*;
use gio::prelude::*;
use glib::variant::ToVariant;

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

fn read_valent_devices() -> Result<Vec<ValentDeviceInfo>, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
        .map_err(|error| format!("Could not connect to the session bus: {error}"))?;

    let reply_type = glib::VariantTy::new("(a{oa{sa{sv}}})")
        .expect("the ObjectManager reply signature is valid");
    let reply = connection
        .call_sync(
            Some(VALENT_BUS_NAME),
            VALENT_OBJECT_PATH,
            OBJECT_MANAGER_INTERFACE,
            "GetManagedObjects",
            None,
            Some(reply_type),
            gio::DBusCallFlags::NONE,
            3000,
            gio::Cancellable::NONE,
        )
        .map_err(|error| {
            format!(
                "Valent is unavailable or does not expose its device manager: {error}"
            )
        })?;

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
                    "Id" => id = value.get::<String>().unwrap_or(id),
                    "Name" => name = value.get::<String>().unwrap_or(name),
                    "State" => state = value.get::<u32>().unwrap_or(state),
                    _ => {}
                }
            }

            devices.push(ValentDeviceInfo { id, name, state });
        }
    }

    devices.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(devices)
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

fn make_scrolled_page(content: &gtk::Box) -> gtk::ScrolledWindow {
    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_child(Some(content));
    scrolled
}

fn refresh_valent_devices(status: &gtk::Label, device_list: &gtk::Box) {
    while let Some(child) = device_list.first_child() {
        device_list.remove(&child);
    }

    match read_valent_devices() {
        Ok(devices) if devices.is_empty() => {
            status.set_text(
                "Valent is available, but it has no known devices yet. Open Valent and make sure device discovery is enabled.",
            );
        }
        Ok(devices) => {
            status.set_text(&format!(
                "Found {} device{} through Valent.",
                devices.len(),
                if devices.len() == 1 { "" } else { "s" }
            ));
            for device in devices {
                let card = make_card(&device.name, device.connection_label());
                card.set_tooltip_text(Some(&format!("Valent device ID: {}", device.id)));
                device_list.append(&card);
            }
        }
        Err(error) => status.set_text(&error),
    }
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
    nearby.append(&refresh_button);
    nearby.append(&discovery_status);
    nearby.append(&device_list);
    nearby.append(&make_card(
        "File sharing",
        "Device discovery is connected. Sending and receiving will be enabled after Viloja validates the exported device actions and transfer flow.",
    ));

    let status_for_refresh = discovery_status.clone();
    let list_for_refresh = device_list.clone();
    refresh_button.connect_clicked(move |_| {
        refresh_valent_devices(&status_for_refresh, &list_for_refresh);
    });
    stack.add_titled(&make_scrolled_page(&nearby), Some("nearby"), "Nearby");

    let send = make_page(
        "Send files",
        "Choose files to prepare a transfer. Sending is not available until the real transfer actions are integrated.",
    );
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
        "Incoming and outgoing transfers will be listed here.",
    );
    transfers.append(&make_card(
        "No transfers yet",
        "Transfer progress, cancellation, completion, and error states will appear once transfer events are integrated.",
    ));
    stack.add_titled(&make_scrolled_page(&transfers), Some("transfers"), "Transfers");

    let history = make_page(
        "History",
        "A local overview of completed and failed transfers.",
    );
    history.append(&make_card(
        "History is empty",
        "Viloja does not record transfer history in this prototype.",
    ));
    stack.add_titled(&make_scrolled_page(&history), Some("history"), "History");

    let settings = make_page(
        "Settings",
        "Preferences for device discovery, receiving, and privacy.",
    );
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
