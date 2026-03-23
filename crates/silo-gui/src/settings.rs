use adw::prelude::*;
use silo_core::browser::BrowserEntry;
use silo_core::config::{self, BrowserRef, Config, Rule};
use std::cell::RefCell;
use std::rc::Rc;

fn build_rules_group(
    browsers: &[BrowserEntry],
    config: &Config,
    window: &adw::PreferencesWindow,
) -> adw::PreferencesGroup {
    let rules_group = adw::PreferencesGroup::builder()
        .title("Domain rules")
        .description("Links matching these rules skip the picker and open in the assigned browser.")
        .build();

    // "Add" button in group header
    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Add rule")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let browsers_for_add = browsers.to_vec();
    let window_for_add = window.clone();
    add_btn.connect_clicked(move |_| {
        show_add_rule_dialog(&window_for_add, &browsers_for_add, None);
    });
    rules_group.set_header_suffix(Some(&add_btn));

    let stale_rules = silo_core::rule::find_stale_rules(&config.rules, browsers);

    for rule in &config.rules {
        let is_stale = stale_rules.iter().any(|s| std::ptr::eq(*s, rule));

        let browser_name = match &rule.browser {
            Some(browser) => {
                let name = browsers
                    .iter()
                    .find(|b| {
                        b.desktop_file == browser.desktop_file
                            && b.profile_args.as_deref() == browser.args.as_deref()
                    })
                    .map(|b| b.display_name.as_str())
                    .unwrap_or(&browser.desktop_file);
                if is_stale {
                    format!("{name} (not found)")
                } else {
                    name.to_string()
                }
            }
            None => "Always show picker".to_string(),
        };

        let row = adw::ActionRow::builder()
            .title(&rule.pattern)
            .subtitle(&browser_name)
            .activatable(true)
            .build();

        if is_stale {
            let warning = gtk::Image::from_icon_name("dialog-warning-symbolic");
            warning.add_css_class("warning");
            row.add_suffix(&warning);
        }

        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let pattern_for_delete = rule.pattern.clone();
        let group_ref = rules_group.clone();
        delete_btn.connect_clicked(move |btn| {
            let mut config = config::load();
            config.rules.retain(|r| r.pattern != pattern_for_delete);
            if let Err(e) = config::save(&config) {
                eprintln!("silo: failed to save config: {e}");
            }
            // Remove the row visually
            if let Some(row) = btn.parent().and_then(|p| p.parent()) {
                group_ref.remove(&row);
            }
        });

        // Click row to edit
        let browsers_for_edit = browsers.to_vec();
        let window_for_edit = window.clone();
        let rule_for_edit = rule.clone();
        row.connect_activated(move |_| {
            show_add_rule_dialog(&window_for_edit, &browsers_for_edit, Some(&rule_for_edit));
        });

        row.add_suffix(&delete_btn);
        rules_group.add(&row);
    }

    if config.rules.is_empty() {
        let empty_row = adw::ActionRow::builder()
            .title("No rules yet")
            .subtitle("Add one above or use 'Always use for...' in the picker")
            .build();
        rules_group.add(&empty_row);
    }

    rules_group
}

fn show_add_rule_dialog(
    window: &adw::PreferencesWindow,
    browsers: &[BrowserEntry],
    existing: Option<&Rule>,
) {
    let heading = if existing.is_some() { "Edit rule" } else { "Add rule" };

    let pattern_row = adw::EntryRow::builder()
        .title("Pattern")
        .build();
    pattern_row.set_text(
        existing.map(|r| r.pattern.as_str()).unwrap_or("")
    );

    // Browser list: "Always show picker" + all detected browsers
    let browser_names: Vec<String> = std::iter::once("Always show picker".to_string())
        .chain(browsers.iter().map(|b| b.display_name.clone()))
        .collect();
    let model = gtk::StringList::new(
        &browser_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );
    let browser_row = adw::ComboRow::builder()
        .title("Browser")
        .model(&model)
        .build();

    // Pre-select browser if editing
    if let Some(rule) = existing
        && let Some(ref browser) = rule.browser
        && let Some(pos) = browsers.iter().position(|b| {
            b.desktop_file == browser.desktop_file
                && b.profile_args.as_deref() == browser.args.as_deref()
        }) {
            browser_row.set_selected(pos as u32 + 1);
        }

    let hint = gtk::Label::builder()
        .label("e.g. github.com, *.corp.com, github.com/gist")
        .css_classes(["dim-label", "caption"])
        .halign(gtk::Align::Start)
        .margin_top(8)
        .build();

    let content = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    content.append(&pattern_row);
    content.append(&browser_row);

    let outer = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();
    outer.append(&content);
    outer.append(&hint);

    let dialog = adw::AlertDialog::builder()
        .heading(heading)
        .extra_child(&outer)
        .build();

    let save_id = if existing.is_some() { "save" } else { "add" };
    let save_label = if existing.is_some() { "Save" } else { "Add" };
    dialog.add_responses(&[("cancel", "Cancel"), (save_id, save_label)]);
    dialog.set_response_appearance(save_id, adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some(save_id));

    let pattern_ref = pattern_row.clone();
    let browser_ref = browser_row.clone();
    let browsers_clone = browsers.to_vec();
    let old_pattern = existing.map(|r| r.pattern.clone());

    dialog.connect_response(None, move |_, response| {
        if response == "cancel" {
            return;
        }

        let raw_pattern = pattern_ref.text().trim().to_string();
        let pattern = raw_pattern
            .strip_prefix("https://")
            .or_else(|| raw_pattern.strip_prefix("http://"))
            .unwrap_or(&raw_pattern)
            .to_string();
        if pattern.is_empty() {
            return;
        }

        let selected = browser_ref.selected();
        let browser = if selected == 0 {
            None
        } else {
            browsers_clone.get(selected as usize - 1).map(|b| BrowserRef {
                desktop_file: b.desktop_file.clone(),
                args: b.profile_args.clone(),
            })
        };

        let mut config = config::load();

        // Remove old rule if editing
        if let Some(ref old) = old_pattern {
            config.rules.retain(|r| r.pattern != *old);
        }
        // Remove any existing rule for this pattern
        config.rules.retain(|r| r.pattern != pattern);

        config.rules.push(Rule { pattern, browser });

        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }
    });

    dialog.present(Some(window));
}

fn show_add_custom_browser_dialog(window: &adw::PreferencesWindow) {
    let name_row = adw::EntryRow::builder()
        .title("Name")
        .build();

    let command_row = adw::EntryRow::builder()
        .title("Command")
        .build();

    let args_row = adw::EntryRow::builder()
        .title("Arguments (optional)")
        .build();

    let hint = gtk::Label::builder()
        .label("e.g. /usr/bin/qutebrowser")
        .css_classes(["dim-label", "caption"])
        .halign(gtk::Align::Start)
        .margin_top(8)
        .build();

    let content = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    content.append(&name_row);
    content.append(&command_row);
    content.append(&args_row);

    let outer = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();
    outer.append(&content);
    outer.append(&hint);

    let dialog = adw::AlertDialog::builder()
        .heading("Add custom browser")
        .extra_child(&outer)
        .build();
    dialog.add_responses(&[("cancel", "Cancel"), ("add", "Add")]);
    dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("add"));

    let name_ref = name_row.clone();
    let command_ref = command_row.clone();
    let args_ref = args_row.clone();
    let window_ref = window.clone();

    dialog.connect_response(None, move |_, response| {
        if response == "cancel" {
            return;
        }

        let name = name_ref.text().trim().to_string();
        let command = command_ref.text().trim().to_string();
        if name.is_empty() || command.is_empty() {
            return;
        }

        let args_text = args_ref.text().trim().to_string();
        let args = if args_text.is_empty() { None } else { Some(args_text) };

        let mut config = config::load();
        config.custom_browsers.push(config::CustomBrowser {
            name,
            command,
            args,
        });

        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }

        if let Some(app) = window_ref.application().and_downcast::<adw::Application>() {
            window_ref.close();
            let config = config::load();
            let browsers = silo_core::browser::discover_with_config(&config);
            show_on_page(&app, &config, &browsers, Some("browsers"));
        }
    });

    dialog.present(Some(window));
}

pub fn show(app: &adw::Application, config: &Config, browsers: &[BrowserEntry]) -> adw::PreferencesWindow {
    show_on_page(app, config, browsers, None)
}

pub fn show_on_page(
    app: &adw::Application,
    config: &Config,
    browsers: &[BrowserEntry],
    page: Option<&str>,
) -> adw::PreferencesWindow {
    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("Silo")
        .default_width(550)
        .default_height(680)
        .build();

    // -- welcome --

    let welcome_page = adw::PreferencesPage::builder()
        .title("Welcome")
        .icon_name("go-home-symbolic")
        .name("welcome")
        .build();

    let welcome_group = adw::PreferencesGroup::builder()
        .description("Silo lets you choose which browser opens your links.\n\nWhen you click a link in an email, chat or any app, Silo pops up with a list of your browsers and their profiles. You choose one, Silo disappears and the link opens in your chosen browser.\n\nIf you toggle on the \u{201c}Always use for\u{2026}\u{201d} switch in the picker, links to that domain will always open silently in the selected browser.\n\nUse the <b>Browsers</b> tab to hide browsers or profiles you don\u{2019}t use. The <b>Rules</b> tab lets you set domains that should always open in a specific browser, and decide what happens when no rule matches. The <b>Open</b> tab lets you paste a URL and open it in any browser, or check it for safety.\n\nSilo also unwraps tracking redirects. If a link has been wrapped by Outlook SafeLinks or Google, Silo strips the wrapper and shows you where it really goes.")
        .build();

    welcome_page.add(&welcome_group);

    // -- setup --

    let current_default_desktop = std::process::Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let is_default = current_default_desktop == "com.nofaff.Silo.desktop";

    let setup_description = if is_default {
        "Silo is intercepting your links.".to_string()
    } else {
        // Look up the base browser name from the .desktop file, not from
        // the profile list (profiles are a Silo concept, not a system one)
        let current_name = silo_core::desktop::find_by_id(&current_default_desktop)
            .map(|entry| entry.name);

        match current_name {
            Some(name) => format!("You need to do this to allow Silo to work. Your current default, {name}, has been saved and will be restored if you uninstall Silo."),
            None => "You need to do this to allow Silo to work.".to_string(),
        }
    };

    let setup_title = if is_default { "Done" } else { "Silo needs to be your default browser" };

    let setup_group = adw::PreferencesGroup::builder()
        .title(setup_title)
        .description(&setup_description)
        .build();

    if is_default {
        let status_row = adw::ActionRow::builder()
            .title("Silo is your default browser")
            .subtitle("You're all set")
            .build();
        status_row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
        setup_group.add(&status_row);
    }

    welcome_page.add(&setup_group);

    let btn_group = adw::PreferencesGroup::builder()
        .visible(!is_default)
        .build();

    let register_btn = gtk::Button::builder()
        .label("Set as default browser")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .build();

    let setup_group_ref = setup_group.clone();
    let app_ref = app.clone();
    let window_for_setup = window.clone();
    register_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        btn.set_label("Setting up, please wait...");

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            tx.send(silo_core::register::set_default_browser()).ok();
        });

        let setup_group = setup_group_ref.clone();
        let btn_ref = btn.clone();
        let win = window_for_setup.clone();
        let app = app_ref.clone();
        let started = std::time::Instant::now();
        gtk::glib::idle_add_local(move || {
            if started.elapsed() > std::time::Duration::from_secs(30) {
                eprintln!("silo: xdg-settings timed out after 30 seconds");
                btn_ref.set_label("Set as default browser");
                btn_ref.set_sensitive(true);
                setup_group.set_description(Some(
                    "Registration timed out. xdg-settings did not respond within 30 seconds. Try again or set the default browser manually in your system settings.",
                ));
                return gtk::glib::ControlFlow::Break;
            }

            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(previous) => {
                            let mut config = config::load();
                            config.previous_default_browser = previous;
                            let _ = config::save(&config);
                            // Reopen settings - it'll show the "all set" state
                            win.close();
                            let config = config::load();
                            let browsers = silo_core::browser::discover_with_config(&config);
                            show_on_page(&app, &config, &browsers, Some("welcome"));
                        }
                        Err(e) => {
                            eprintln!("silo: {e}");
                            btn_ref.set_label("Set as default browser");
                            btn_ref.set_sensitive(true);
                            setup_group.set_description(Some(&format!("Registration failed: {e}. Try again.")));
                        }
                    }
                    gtk::glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                Err(_) => gtk::glib::ControlFlow::Break,
            }
        });
    });

    btn_group.add(&adw::PreferencesRow::builder().child(&register_btn).build());

    // Remove card styling from this group
    let provider = gtk::CssProvider::new();
    provider.load_from_string(".bare-group { margin-top: -12px; } .bare-group list { background: none; box-shadow: none; border: none; } .bare-group row { background: none; border: none; } .bare-group row:focus { outline: none; }");
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    btn_group.add_css_class("bare-group");

    welcome_page.add(&btn_group);

    // -- browsers --

    let browsers_page = adw::PreferencesPage::builder()
        .title("Browsers")
        .icon_name("view-app-grid-symbolic")
        .name("browsers")
        .build();

    let all_browsers = silo_core::browser::discover();

    let detected_group = adw::PreferencesGroup::builder()
        .title("Detected browsers")
        .description("Browsers and profiles found on your system. Hide any you do not use.")
        .build();

    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Re-detect browsers")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let window_for_refresh = window.clone();
    let app_for_refresh = app.clone();
    refresh_btn.connect_clicked(move |_| {
        window_for_refresh.close();
        let config = config::load();
        let browsers = silo_core::browser::discover_with_config(&config);
        show_on_page(&app_for_refresh, &config, &browsers, Some("browsers"));
    });
    detected_group.set_header_suffix(Some(&refresh_btn));

    for entry in &all_browsers {
        let is_hidden = config.browser_overrides.iter().any(|o| {
            o.desktop_file == entry.desktop_file
                && o.profile_args.as_deref() == entry.profile_args.as_deref()
                && o.hidden
        });

        let row = adw::SwitchRow::builder()
            .title(&entry.display_name)
            .subtitle(&entry.desktop_file)
            .active(!is_hidden)
            .build();

        let desktop_file = entry.desktop_file.clone();
        let profile_args = entry.profile_args.clone();
        row.connect_active_notify(move |row| {
            let mut config = config::load();
            let hidden = !row.is_active();

            if let Some(ov) = config.browser_overrides.iter_mut().find(|o| {
                o.desktop_file == desktop_file
                    && o.profile_args.as_deref() == profile_args.as_deref()
            }) {
                ov.hidden = hidden;
            } else if hidden {
                config.browser_overrides.push(config::BrowserOverride {
                    desktop_file: desktop_file.clone(),
                    profile_args: profile_args.clone(),
                    display_name: None,
                    hidden,
                });
            }

            // Clean up overrides that are back to default
            config.browser_overrides.retain(|o| {
                o.hidden || o.display_name.is_some()
            });

            if let Err(e) = config::save(&config) {
                eprintln!("silo: failed to save config: {e}");
            }
        });

        detected_group.add(&row);
    }

    browsers_page.add(&detected_group);

    // -- custom browsers --

    let custom_group = adw::PreferencesGroup::builder()
        .title("Custom browsers")
        .description("Add browsers that were not detected automatically.")
        .build();

    let add_custom_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Add custom browser")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let window_for_custom = window.clone();
    add_custom_btn.connect_clicked(move |_| {
        show_add_custom_browser_dialog(&window_for_custom);
    });
    custom_group.set_header_suffix(Some(&add_custom_btn));

    for (i, cb) in config.custom_browsers.iter().enumerate() {
        let subtitle = match &cb.args {
            Some(args) => format!("{} {}", cb.command, args),
            None => cb.command.clone(),
        };

        let row = adw::ActionRow::builder()
            .title(&cb.name)
            .subtitle(&subtitle)
            .build();

        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let group_for_del = custom_group.clone();
        let idx = i;
        delete_btn.connect_clicked(move |btn| {
            let mut config = config::load();
            if idx < config.custom_browsers.len() {
                config.custom_browsers.remove(idx);
                if let Err(e) = config::save(&config) {
                    eprintln!("silo: failed to save config: {e}");
                }
            }
            if let Some(row) = btn.parent().and_then(|p| p.parent()) {
                group_for_del.remove(&row);
            }
        });

        row.add_suffix(&delete_btn);
        custom_group.add(&row);
    }

    if config.custom_browsers.is_empty() {
        let empty_row = adw::ActionRow::builder()
            .title("No custom browsers")
            .build();
        custom_group.add(&empty_row);
    }

    browsers_page.add(&custom_group);

    // -- rules --

    let rules_page = adw::PreferencesPage::builder()
        .title("Rules")
        .icon_name("view-list-symbolic")
        .name("rules")
        .build();

    // Unmatched links behaviour
    let unmatched_group = adw::PreferencesGroup::builder()
        .description("Would you like unmatched links to go to a specific browser silently, or always see the picker?")
        .build();

    let unmatched_row = adw::ComboRow::builder()
        .title("When no rule matches")
        .build();

    let unmatched_names: Vec<String> = std::iter::once("Show the picker".to_string())
        .chain(browsers.iter().map(|b| b.display_name.clone()))
        .collect();
    let unmatched_model = gtk::StringList::new(
        &unmatched_names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
    );
    unmatched_row.set_model(Some(&unmatched_model));

    if !config.always_ask {
        if let Some(ref fb) = config.fallback_browser
            && let Some(pos) = browsers.iter().position(|b| {
                b.desktop_file == fb.desktop_file
                    && b.profile_args.as_deref() == fb.args.as_deref()
            }) {
                unmatched_row.set_selected(pos as u32 + 1);
            }
    }

    let browsers_for_unmatched = browsers.to_vec();
    unmatched_row.connect_selected_notify(move |row| {
        let mut config = config::load();
        let selected = row.selected();
        if selected == 0 {
            config.always_ask = true;
            config.fallback_browser = None;
        } else {
            config.always_ask = false;
            config.fallback_browser = browsers_for_unmatched.get(selected as usize - 1).map(|b| {
                silo_core::config::BrowserRef {
                    desktop_file: b.desktop_file.clone(),
                    args: b.profile_args.clone(),
                }
            });
        }
        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }
    });

    unmatched_group.add(&unmatched_row);
    rules_page.add(&unmatched_group);

    // Domain rules
    let rules_group = build_rules_group(browsers, config, &window);
    rules_page.add(&rules_group);

    let current_rules_group = Rc::new(RefCell::new(rules_group));

    // Suspend rules
    let suspend_group = adw::PreferencesGroup::new();

    let suspend_row = adw::SwitchRow::builder()
        .title("Suspend rules")
        .subtitle("Temporarily ignore all rules and show the picker")
        .active(config.rules_suspended)
        .build();

    let window_for_suspend = window.clone();
    suspend_row.connect_active_notify(move |row| {
        let mut config = config::load();
        config.rules_suspended = row.is_active();
        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }
        let _ = &window_for_suspend;
    });

    suspend_group.add(&suspend_row);
    rules_page.add(&suspend_group);

    // Export/import
    let config_group = adw::PreferencesGroup::builder()
        .title("Back up your settings")
        .build();

    let export_row = adw::ActionRow::builder()
        .title("Export config")
        .subtitle("Save your settings and rules to a file")
        .activatable(true)
        .build();
    export_row.add_prefix(&gtk::Image::from_icon_name("document-save-symbolic"));

    let window_for_export = window.clone();
    export_row.connect_activated(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Export Silo config")
            .initial_name("silo-config.json")
            .build();

        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.json");
        filter.set_name(Some("JSON files"));
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));

        let win_ref = window_for_export.clone();
        dialog.save(Some(&window_for_export), gtk::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let source = config::config_path();
                    match std::fs::copy(&source, &path) {
                        Ok(_) => {
                            let toast = adw::Toast::new("Config exported");
                            win_ref.add_toast(toast);
                        }
                        Err(e) => {
                            eprintln!("silo: export failed: {e}");
                            let toast = adw::Toast::new("Export failed");
                            win_ref.add_toast(toast);
                        }
                    }
                }
            }
        });
    });

    let import_row = adw::ActionRow::builder()
        .title("Import config")
        .subtitle("Load settings and rules from a file. This replaces your current config.")
        .activatable(true)
        .build();
    import_row.add_prefix(&gtk::Image::from_icon_name("document-open-symbolic"));

    let window_for_import = window.clone();
    import_row.connect_activated(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Import Silo config")
            .build();

        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.json");
        filter.set_name(Some("JSON files"));
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));

        let win_ref = window_for_import.clone();
        dialog.open(Some(&window_for_import), gtk::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    match config::try_load_from(&path) {
                        Ok(imported) => {
                            let confirm = adw::AlertDialog::builder()
                                .heading("Import config?")
                                .body("This will replace your current settings and rules.")
                                .build();
                            confirm.add_responses(&[("cancel", "Cancel"), ("import", "Import")]);
                            confirm.set_response_appearance("import", adw::ResponseAppearance::Destructive);
                            confirm.set_default_response(Some("cancel"));

                            let win = win_ref.clone();
                            confirm.connect_response(None, move |_, response| {
                                if response != "import" {
                                    return;
                                }
                                if let Err(e) = config::save(&imported) {
                                    eprintln!("silo: import save failed: {e}");
                                }
                                if let Some(app) = win.application().and_downcast::<adw::Application>() {
                                    win.close();
                                    let config = config::load();
                                    let browsers = silo_core::browser::discover_with_config(&config);
                                    show_on_page(&app, &config, &browsers, Some("rules"));
                                } else {
                                    win.close();
                                }
                            });
                            confirm.present(Some(&win_ref));
                        }
                        Err(e) => {
                            eprintln!("silo: invalid config file: {e}");
                            let err = adw::AlertDialog::builder()
                                .heading("Import failed")
                                .body("The file is not a valid Silo config.")
                                .build();
                            err.add_responses(&[("close", "Dismiss")]);
                            err.present(Some(&win_ref));
                        }
                    }
                }
            }
        });
    });

    config_group.add(&export_row);
    config_group.add(&import_row);
    rules_page.add(&config_group);

    // -- open --

    let open_page = adw::PreferencesPage::builder()
        .title("Open")
        .icon_name("globe-symbolic")
        .build();

    let open_group = adw::PreferencesGroup::builder()
        .title("Open a link")
        .description("Paste a URL and choose which browser to open it in.")
        .build();

    let url_row = adw::EntryRow::builder()
        .title("URL")
        .build();

    let open_browser_row = adw::ComboRow::builder()
        .title("Browser")
        .build();

    let open_browser_names: Vec<String> = browsers.iter().map(|b| b.display_name.clone()).collect();
    let open_model = gtk::StringList::new(
        &open_browser_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );
    open_browser_row.set_model(Some(&open_model));

    let no_browsers = browsers.is_empty();

    let open_btn = gtk::Button::builder()
        .label("Open")
        .css_classes(["suggested-action"])
        .valign(gtk::Align::Center)
        .sensitive(!no_browsers)
        .build();

    let url_for_open = url_row.clone();
    let browser_for_open = open_browser_row.clone();
    let browsers_for_open = browsers.to_vec();
    open_btn.connect_clicked(move |_| {
        let text = url_for_open.text().to_string();
        if text.is_empty() {
            return;
        }
        let processed = silo_core::url::process_url(&text);
        let selected = browser_for_open.selected() as usize;
        if let Some(entry) = browsers_for_open.get(selected) {
            if let Err(e) = silo_core::launcher::launch(entry, &processed.final_url) {
                eprintln!("silo: {e}");
            }
        }
    });

    open_browser_row.add_suffix(&open_btn);

    let redirect_row = adw::ActionRow::builder()
        .visible(false)
        .build();
    redirect_row.add_prefix(&gtk::Image::from_icon_name("mail-forward-symbolic"));

    let redirect_row_ref = redirect_row.clone();
    let debounce_id: Rc<RefCell<Option<gtk::glib::SourceId>>> = Rc::new(RefCell::new(None));
    url_row.connect_changed(move |row| {
        let text = row.text().to_string();
        if text.is_empty() {
            redirect_row_ref.set_visible(false);
            return;
        }

        // Cancel any pending debounce
        if let Some(id) = debounce_id.borrow_mut().take() {
            id.remove();
        }

        let redirect_row = redirect_row_ref.clone();
        let debounce = debounce_id.clone();
        let id = gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
            let processed = silo_core::url::process_url(&text);
            if processed.was_redirected {
                redirect_row.set_title("Unwrapped from redirect");
                redirect_row.set_subtitle(&processed.final_url);
                redirect_row.set_visible(true);
            } else {
                redirect_row.set_visible(false);
            }
            *debounce.borrow_mut() = None;
        });
        *debounce_id.borrow_mut() = Some(id);
    });

    if no_browsers {
        let no_browsers_row = adw::ActionRow::builder()
            .title("No browsers found")
            .subtitle("Install a browser or check the Browsers tab")
            .build();
        no_browsers_row.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
        open_group.add(&no_browsers_row);
    }

    open_group.add(&url_row);
    open_group.add(&redirect_row);
    open_group.add(&open_browser_row);

    open_page.add(&open_group);

    // Safety check
    let safety_group = adw::PreferencesGroup::builder()
        .title("Safety check")
        .description("Check the URL above against Google's Transparency Report for known threats.")
        .build();

    let check_btn_row = adw::ActionRow::builder()
        .title("Check URL")
        .subtitle("Opens the Transparency Report in the browser selected above")
        .activatable(!no_browsers)
        .sensitive(!no_browsers)
        .build();
    check_btn_row.add_prefix(&gtk::Image::from_icon_name("security-high-symbolic"));

    let url_for_check = url_row.clone();
    let browser_for_check = open_browser_row.clone();
    let browsers_for_check = browsers.to_vec();
    check_btn_row.connect_activated(move |_| {
        let text = url_for_check.text().to_string();
        if text.is_empty() {
            return;
        }
        let processed = silo_core::url::process_url(&text);
        let report_url = format!(
            "https://transparencyreport.google.com/safe-browsing/search?url={}",
            gtk::glib::Uri::escape_string(&processed.final_url, None, true)
        );
        let selected = browser_for_check.selected() as usize;
        if let Some(entry) = browsers_for_check.get(selected) {
            if let Err(e) = silo_core::launcher::launch(entry, &report_url) {
                eprintln!("silo: {e}");
            }
        }
    });

    safety_group.add(&check_btn_row);
    open_page.add(&safety_group);

    // -- about --

    let about_page = adw::PreferencesPage::builder()
        .title("About")
        .icon_name("help-about-symbolic")
        .build();

    let info_group = adw::PreferencesGroup::builder()
        .title(&format!("Silo {}", env!("CARGO_PKG_VERSION")))
        .description("Browser picker with profile support. MIT licence.")
        .build();

    let github_row = adw::ActionRow::builder()
        .title("GitHub")
        .subtitle("github.com/no-faff/silo")
        .activatable(true)
        .build();
    github_row.add_prefix(&gtk::Image::from_icon_name("starred-symbolic"));
    github_row.add_suffix(&gtk::Image::from_icon_name("external-link-symbolic"));
    let window_for_github = window.clone();
    github_row.connect_activated(move |_| {
        let launcher = gtk::UriLauncher::new("https://github.com/no-faff/silo");
        launcher.launch(Some(&window_for_github), gtk::gio::Cancellable::NONE, |_| {});
    });

    let issue_row = adw::ActionRow::builder()
        .title("Report an issue")
        .activatable(true)
        .build();
    issue_row.add_prefix(&gtk::Image::from_icon_name("mail-send-symbolic"));
    issue_row.add_suffix(&gtk::Image::from_icon_name("external-link-symbolic"));
    let window_for_issue = window.clone();
    issue_row.connect_activated(move |_| {
        let launcher = gtk::UriLauncher::new("https://github.com/no-faff/silo/issues");
        launcher.launch(Some(&window_for_issue), gtk::gio::Cancellable::NONE, |_| {});
    });

    let donate_row = adw::ActionRow::builder()
        .title("Buy me a cuppa")
        .activatable(true)
        .build();
    donate_row.add_prefix(&gtk::Image::from_icon_name("emblem-favorite-symbolic"));
    donate_row.add_suffix(&gtk::Image::from_icon_name("external-link-symbolic"));
    let window_for_donate = window.clone();
    donate_row.connect_activated(move |_| {
        let launcher = gtk::UriLauncher::new("https://nofaff.netlify.app");
        launcher.launch(Some(&window_for_donate), gtk::gio::Cancellable::NONE, |_| {});
    });

    info_group.add(&github_row);
    info_group.add(&issue_row);
    info_group.add(&donate_row);
    about_page.add(&info_group);

    // -- uninstall --

    let uninstall_page = adw::PreferencesPage::builder()
        .title("Uninstall")
        .icon_name("user-trash-symbolic")
        .build();

    let previous_browser_name = config.previous_default_browser.as_deref()
        .and_then(|desktop_file| {
            silo_core::desktop::find_by_id(desktop_file)
                .map(|entry| entry.name)
        });

    let (uninstall_description, uninstall_subtitle) = match &previous_browser_name {
        Some(name) => (
            format!("This removes Silo from your system and restores {name} as your default browser."),
            format!("Remove Silo and restore {name} as your default browser"),
        ),
        None => (
            "This removes Silo from your system. Your default browser was not recorded, so you may need to set one manually afterwards.".to_string(),
            "Remove Silo from your system".to_string(),
        ),
    };

    let uninstall_group = adw::PreferencesGroup::builder()
        .description(&uninstall_description)
        .build();

    let uninstall_row = adw::ActionRow::builder()
        .title("Uninstall Silo")
        .subtitle(&uninstall_subtitle)
        .activatable(true)
        .build();
    uninstall_row.add_prefix(&gtk::Image::from_icon_name("user-trash-symbolic"));

    let window_for_uninstall = window.clone();
    let prev_name_for_uninstall = previous_browser_name.clone();
    uninstall_row.connect_activated(move |_| {
        let confirm_body = match &prev_name_for_uninstall {
            Some(name) => format!("This will remove Silo, its config and rules and restore {name} as your default browser."),
            None => "This will remove Silo, its config and rules. You may need to set a default browser manually afterwards.".to_string(),
        };

        let dialog = adw::AlertDialog::builder()
            .heading("Uninstall Silo?")
            .body(&confirm_body)
            .build();
        dialog.add_responses(&[("cancel", "Cancel"), ("uninstall", "Uninstall")]);
        dialog.set_response_appearance("uninstall", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));

        let win = window_for_uninstall.clone();
        let prev_name = prev_name_for_uninstall.clone();
        dialog.connect_response(None, move |_, response| {
            if response == "uninstall" {
                match silo_core::register::uninstall() {
                    Ok(()) => {
                        eprintln!("silo: uninstalled");
                        let done_body = match &prev_name {
                            Some(name) => format!("{name} has been restored as your default browser."),
                            None => "You may need to set a default browser manually.".to_string(),
                        };
                        let done = adw::AlertDialog::builder()
                            .heading("Silo has been uninstalled")
                            .body(&done_body)
                            .build();
                        done.add_responses(&[("close", "Close")]);
                        done.connect_response(None, move |_, _| {
                            std::process::exit(0);
                        });
                        done.present(Some(&win));
                    }
                    Err(e) => {
                        eprintln!("silo: uninstall failed: {e}");
                        let err = adw::AlertDialog::builder()
                            .heading("Uninstall failed")
                            .body(&format!("{e}"))
                            .build();
                        err.add_responses(&[("close", "Dismiss")]);
                        err.present(Some(&win));
                    }
                }
            }
        });
        dialog.present(Some(&window_for_uninstall));
    });

    uninstall_group.add(&uninstall_row);
    uninstall_page.add(&uninstall_group);

    // -- assemble window --

    window.add(&welcome_page);
    window.add(&browsers_page);
    window.add(&rules_page);
    window.add(&open_page);
    window.add(&about_page);
    window.add(&uninstall_page);

    // Show the requested page, or welcome if not registered
    if let Some(name) = page {
        window.set_visible_page_name(name);
    } else if !is_default {
        window.set_visible_page_name("welcome");
    }

    // Refresh rules when window regains focus
    {
        let rules_page = rules_page.clone();
        let current_rules_group = current_rules_group.clone();
        let suspend_group = suspend_group.clone();
        let config_group = config_group.clone();
        let browsers = browsers.to_vec();
        window.connect_notify_local(Some("is-active"), move |win, _| {
            if !win.is_active() {
                return;
            }
            let config = config::load();
            let old_group = current_rules_group.borrow().clone();
            rules_page.remove(&old_group);
            rules_page.remove(&suspend_group);
            rules_page.remove(&config_group);
            let new_group = build_rules_group(&browsers, &config, win);
            rules_page.add(&new_group);
            rules_page.add(&suspend_group);
            rules_page.add(&config_group);
            *current_rules_group.borrow_mut() = new_group;
        });
    }

    window.connect_close_request(move |win| {
        if is_default {
            return gtk::glib::Propagation::Proceed;
        }

        // Check if Silo is now the default browser
        let now_default = std::process::Command::new("xdg-settings")
            .args(["get", "default-web-browser"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "com.nofaff.Silo.desktop")
            .unwrap_or(false);

        if now_default {
            return gtk::glib::Propagation::Proceed;
        }

        // Not default yet: warn the user
        let dialog = adw::AlertDialog::builder()
            .heading("Silo is not your default browser")
            .body("Silo won\u{2019}t work until you set it as your default browser.")
            .build();
        dialog.add_responses(&[("close", "Close anyway"), ("back", "Go back")]);
        dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("back", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("back"));

        let win_ref = win.clone();
        dialog.connect_response(None, move |_, response| {
            if response == "close" {
                win_ref.destroy();
            } else {
                win_ref.set_visible_page_name("welcome");
            }
        });
        dialog.present(Some(win));

        gtk::glib::Propagation::Stop
    });

    window.present();

    window
}
