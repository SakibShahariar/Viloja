use adw::prelude::*;
use gtk::prelude::*;

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
    card.set_margin_top(4);
    card.set_margin_bottom(4);
    card.set_margin_start(2);
    card.set_margin_end(2);
    card.set_hexpand(true);
    card.set_margin_top(6);
    card.set_margin_bottom(6);
    card.set_spacing(8);
    card.set_margin_start(12);
    card.set_margin_end(12);
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
    sidebar.set_stack(Some(&stack));
    sidebar.set_width_request(190);

    let nearby = make_page(
        "Nearby",
        "Find devices and share files over supported local connections.",
    );
    nearby.append(&make_card(
        "Device discovery is not connected yet",
        "This first frontend build intentionally does not show fake nearby devices. Real discovery will appear here after a backend is integrated and tested.",
    ));
    nearby.append(&make_card(
        "Backend status",
        "No sharing backend is active. Quick Share and KDE Connect integration are still being evaluated.",
    ));
    stack.add_titled(&make_scrolled_page(&nearby), Some("nearby"), "Nearby");

    let send = make_page(
        "Send files",
        "Choose files to prepare a transfer. Sending is not available until a real backend is connected.",
    );
    let selection_status = make_label("No files selected.", "");
    let choose_button = gtk::Button::with_label("Choose files…");
    choose_button.add_css_class("suggested-action");
    send.append(&make_card(
        "Select files",
        "You can select multiple files and review the selection here. Nothing is uploaded or sent by this prototype.",
    ));
    send.append(&choose_button);
    send.append(&selection_status);
    let send_note = make_label(
        "Transfer will remain disabled until a supported backend can discover a real device.",
        "dim-label",
    );
    send.append(&send_note);
    stack.add_titled(&make_scrolled_page(&send), Some("send"), "Send files");

    let transfers = make_page(
        "Transfers",
        "Incoming and outgoing transfers will be listed here.",
    );
    transfers.append(&make_card(
        "No transfers yet",
        "Transfer progress, cancellation, completion, and error states will be shown here once a backend is integrated.",
    ));
    stack.add_titled(&make_scrolled_page(&transfers), Some("transfers"), "Transfers");

    let history = make_page(
        "History",
        "A local overview of completed and failed transfers.",
    );
    history.append(&make_card(
        "History is empty",
        "Viloja does not record transfer history in this prototype. A local history store will be added with the transfer model.",
    ));
    stack.add_titled(&make_scrolled_page(&history), Some("history"), "History");

    let settings = make_page(
        "Settings",
        "Preferences for device discovery, receiving, and privacy.",
    );
    settings.append(&make_card(
        "Receiving",
        "Receiving preferences will be available when a backend exposes its supported options.",
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
    choose_button.connect_clicked(move |_| {
        let chooser = gtk::FileChooserNative::new(
            Some("Choose files to prepare"),
            Some(&chooser_window),
            gtk::FileChooserAction::Open,
            Some("Select"),
            Some("Cancel"),
        );
        chooser.set_select_multiple(true);

        let status = selection_status.clone();
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
