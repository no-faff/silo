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
    was_redirected: bool,
) -> adw::ApplicationWindow {
    let url = url.to_string();
    let domain_str = domain.unwrap_or("").to_string();

    // -- header bar with settings gear --

    let header = adw::HeaderBar::new();

    let settings_btn = gtk::Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Settings")
        .css_classes(["flat"])
        .build();

    let app_for_settings = app.clone();
    settings_btn.connect_clicked(move |_| {
        let config = config::load();
        let browsers = silo_core::browser::discover_with_config(&config);
        crate::settings::show(&app_for_settings, &config, &browsers);
    });
    header.pack_end(&settings_btn);

    // -- domain hero display --

    let domain_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Center)
        .margin_top(12)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();

    if !domain_str.is_empty() {
        let domain_label = gtk::Label::builder()
            .label(&domain_str)
            .css_classes(["title-1"])
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(40)
            .build();
        domain_box.append(&domain_label);
    }

    if was_redirected {
        let redirect_label = gtk::Label::builder()
            .label("Unwrapped from a redirect")
            .css_classes(["dim-label", "caption"])
            .margin_top(4)
            .build();
        domain_box.append(&redirect_label);
    }

    // -- browser list --

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

    // -- remember toggle (at bottom) --

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
        .margin_bottom(4)
        .build();
    remember_list.append(&remember_row);
    remember_list.set_cursor_from_name(Some("pointer"));

    // -- bottom bar --

    let heart_btn = gtk::Button::builder()
        .icon_name("emblem-favorite-symbolic")
        .tooltip_text("Buy me a cuppa")
        .css_classes(["flat"])
        .build();
    heart_btn.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://nofaff.netlify.app")
            .spawn();
    });

    let star_btn = gtk::Button::builder()
        .icon_name("starred-symbolic")
        .tooltip_text("Leave a star on GitHub")
        .css_classes(["flat"])
        .build();
    star_btn.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/no-faff/silo")
            .spawn();
    });

    let bottom_bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .spacing(4)
        .margin_top(4)
        .margin_bottom(8)
        .build();
    bottom_bar.append(&star_btn);
    bottom_bar.append(&heart_btn);

    // -- assemble layout --

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    content_box.append(&domain_box);
    content_box.append(&scrolled);
    content_box.append(&remember_list);
    content_box.append(&bottom_bar);

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

    // -- row activation handler --

    let browsers_clone = browsers.to_vec();
    let url_clone = url.clone();
    let domain_clone = domain_str.clone();
    let remember_ref = remember_row.clone();
    let window_ref = window.clone();

    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(entry) = browsers_clone.get(index) {
            save_choice(&remember_ref, &domain_clone, entry, &window_ref);

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

    // -- keyboard shortcuts: 1-9, 0, Escape --

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
                save_choice(&remember_for_keys, &domain_for_keys, entry, &window_for_keys);

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

    // Auto-close when picker loses focus (user clicked elsewhere)
    window.connect_notify_local(Some("is-active"), move |win, _| {
        if !win.is_active() {
            win.close();
        }
    });

    // modal so it takes focus even when settings is open behind
    window.set_modal(true);
    window.present();

    window
}

fn save_choice(
    remember_row: &adw::SwitchRow,
    domain: &str,
    entry: &BrowserEntry,
    window: &adw::ApplicationWindow,
) {
    let mut config = config::load();
    config.remember_choice = remember_row.is_active();

    let (w, h) = window.default_size();
    config.picker_size = Some(silo_core::config::WindowSize {
        width: w,
        height: h,
    });

    if remember_row.is_active() && !domain.is_empty() {
        let new_rule = Rule {
            pattern: domain.to_string(),
            browser: Some(BrowserRef {
                desktop_file: entry.desktop_file.clone(),
                args: entry.profile_args.clone(),
            }),
        };
        config.rules.retain(|r| r.pattern != domain);
        config.rules.push(new_rule);
    }

    if let Err(e) = config::save(&config) {
        eprintln!("silo: failed to save config: {e}");
    }
}
