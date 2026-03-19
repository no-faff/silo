use adw::prelude::*;
use silo_core::browser::BrowserEntry;
use silo_core::config::{self, BrowserRef, Config, Rule};

pub fn show(
    app: &adw::Application,
    url: &str,
    domain: Option<&str>,
    browsers: &[BrowserEntry],
    config: &Config,
) {
    let url = url.to_string();
    let domain_str = domain.unwrap_or("").to_string();

    let header = adw::HeaderBar::builder().show_title(false).build();

    let header_label = gtk::Label::builder()
        .label(if domain_str.is_empty() {
            "Open in".to_string()
        } else {
            format!("Open {} in", domain_str)
        })
        .css_classes(["title"])
        .build();

    let remember_check = gtk::CheckButton::builder()
        .label(if domain_str.is_empty() {
            "Always use for this link".to_string()
        } else {
            format!("Always use for {}", domain_str)
        })
        .active(config.remember_choice)
        .build();

    let header_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    header_box.append(&header_label);
    header_box.append(&remember_check);

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
    content_box.append(&header_box);
    content_box.append(&scrolled);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content_box));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .content(&toolbar)
        .default_width(400)
        .default_height(500)
        .build();

    let browsers_clone = browsers.to_vec();
    let url_clone = url.clone();
    let domain_clone = domain_str.clone();
    let remember_ref = remember_check.clone();
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
    let remember_for_keys = remember_check.clone();
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

        if let Some(i) = index {
            if let Some(entry) = browsers_for_keys.get(i) {
                save_choice(&remember_for_keys, &domain_for_keys, entry);

                if let Err(e) = silo_core::launcher::launch(entry, &url_for_keys) {
                    eprintln!("silo: {e}");
                }

                window_for_keys.close();
                return gtk::glib::Propagation::Stop;
            }
        }

        gtk::glib::Propagation::Proceed
    });

    window.add_controller(key_controller);

    // close on focus loss
    let window_for_focus = window.clone();
    window.connect_is_active_notify(move |win| {
        if !win.is_active() {
            window_for_focus.close();
        }
    });

    window.present();
}

fn save_choice(remember_check: &gtk::CheckButton, domain: &str, entry: &BrowserEntry) {
    let mut config = config::load();
    config.remember_choice = remember_check.is_active();

    if remember_check.is_active() && !domain.is_empty() {
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
