use adw::prelude::*;
use silo_core::browser::BrowserEntry;
use silo_core::config::{self, BrowserRef, Config, Rule};

/// Returns the picker window so callers can overlay dialogs on it.
pub fn show(
    app: &adw::Application,
    url: &str,
    domain: Option<&str>,
    browsers: &[BrowserEntry],
    config: &Config,
) -> adw::ApplicationWindow {
    let url = url.to_string();
    let domain_str = domain.unwrap_or("").to_string();

    let header = adw::HeaderBar::builder()
        .show_title(false)
        .build();

    let remember_title = if domain_str.is_empty() {
        "Always use for this link".to_string()
    } else {
        format!("Always use for {}", domain_str)
    };
    let remember_row = adw::SwitchRow::builder()
        .title(&remember_title)
        .active(config.remember_choice)
        .build();

    let remember_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(12)
        .build();
    remember_list.append(&remember_row);

    remember_list.set_cursor_from_name(Some("pointer"));

    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    for (i, entry) in browsers.iter().enumerate() {
        let shortcut = match i {
            0..=8 => format!("{}", i + 1),
            9 => "0".to_string(),
            _ => String::new(),
        };

        let row = adw::ActionRow::builder()
            .title(&entry.display_name)
            .activatable(true)
            .build();

        let icon = gtk::Image::from_icon_name(&entry.icon);
        icon.set_pixel_size(32);
        row.add_prefix(&icon);

        if !shortcut.is_empty() {
            let shortcut_label = gtk::Label::builder()
                .label(&shortcut)
                .css_classes(["dim-label"])
                .build();
            row.add_suffix(&shortcut_label);
        }

        list_box.append(&row);
    }

    let scrolled = gtk::ScrolledWindow::builder()
        .child(&list_box)
        .vexpand(true)
        .build();

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    content_box.append(&remember_list);
    content_box.append(&scrolled);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content_box));

    let (default_w, default_h) = config
        .picker_size
        .as_ref()
        .map(|s| (s.width, s.height))
        .unwrap_or((450, 500));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .content(&toolbar)
        .title("Silo")
        .default_width(default_w)
        .default_height(default_h)
        .build();

    let browsers_clone = browsers.to_vec();
    let url_clone = url.clone();
    let domain_clone = domain_str.clone();
    let remember_ref = remember_row.clone();
    let window_ref = window.clone();

    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(entry) = browsers_clone.get(index) {
            save_choice(&remember_ref, &domain_clone, entry);

            if let Err(e) = silo_core::launcher::launch(entry, &url_clone) {
                let dialog = adw::AlertDialog::builder()
                    .heading("Failed to open browser")
                    .body(&e)
                    .build();
                dialog.add_responses(&[("close", "Dismiss")]);
                dialog.present(Some(&window_ref));
                return;
            }

            window_ref.close();
        }
    });

    // number keys 1-9, 0, Escape
    let key_controller = gtk::EventControllerKey::new();
    let browsers_for_keys = browsers.to_vec();
    let url_for_keys = url.clone();
    let domain_for_keys = domain_str.clone();
    let remember_for_keys = remember_row.clone();
    let window_for_keys = window.clone();

    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gtk::gdk::Key::Escape {
            window_for_keys.close();
            return gtk::glib::Propagation::Stop;
        }

        let index = match keyval {
            gtk::gdk::Key::_1 => Some(0),
            gtk::gdk::Key::_2 => Some(1),
            gtk::gdk::Key::_3 => Some(2),
            gtk::gdk::Key::_4 => Some(3),
            gtk::gdk::Key::_5 => Some(4),
            gtk::gdk::Key::_6 => Some(5),
            gtk::gdk::Key::_7 => Some(6),
            gtk::gdk::Key::_8 => Some(7),
            gtk::gdk::Key::_9 => Some(8),
            gtk::gdk::Key::_0 => Some(9),
            _ => None,
        };

        if let Some(i) = index
            && let Some(entry) = browsers_for_keys.get(i) {
                save_choice(&remember_for_keys, &domain_for_keys, entry);

                if let Err(e) = silo_core::launcher::launch(entry, &url_for_keys) {
                    let dialog = adw::AlertDialog::builder()
                        .heading("Failed to open browser")
                        .body(&e)
                        .build();
                    dialog.add_responses(&[("close", "Dismiss")]);
                    dialog.present(Some(&window_for_keys));
                    return gtk::glib::Propagation::Stop;
                }

                window_for_keys.close();
                return gtk::glib::Propagation::Stop;
            }

        gtk::glib::Propagation::Proceed
    });

    window.add_controller(key_controller);

    window.connect_close_request(|win| {
        let mut config = config::load();
        let (w, h) = win.default_size();
        config.picker_size = Some(silo_core::config::WindowSize {
            width: w,
            height: h,
        });
        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }
        gtk::glib::Propagation::Proceed
    });

    window.present();

    window
}

fn save_choice(remember_row: &adw::SwitchRow, domain: &str, entry: &BrowserEntry) {
    let mut config = config::load();
    config.remember_choice = remember_row.is_active();

    if remember_row.is_active() && !domain.is_empty() {
        let new_rule = Rule {
            domain: domain.to_string(),
            browser: BrowserRef {
                desktop_file: entry.desktop_file.clone(),
                args: entry.profile_args.clone(),
            },
        };
        config.rules.retain(|r| r.domain != domain);
        config.rules.push(new_rule);
    }

    if let Err(e) = config::save(&config) {
        eprintln!("silo: failed to save config: {e}");
    }
}
