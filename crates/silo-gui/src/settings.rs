use adw::prelude::*;
use silo_core::browser::BrowserEntry;
use silo_core::config::{self, Config};
use std::cell::RefCell;
use std::rc::Rc;

fn build_rules_group(
    browsers: &[BrowserEntry],
    config: &Config,
    window: &adw::PreferencesWindow,
) -> adw::PreferencesGroup {
    let rules_group = adw::PreferencesGroup::builder()
        .title("Domain rules")
        .description("Links matching these domains open silently in the assigned browser")
        .build();

    for rule in &config.rules {
        let browser_name = match &rule.browser {
            Some(browser) => browsers
                .iter()
                .find(|b| {
                    b.desktop_file == browser.desktop_file
                        && b.profile_args.as_deref() == browser.args.as_deref()
                })
                .map(|b| b.display_name.as_str())
                .unwrap_or(&browser.desktop_file),
            None => "Always show picker",
        };

        let row = adw::ActionRow::builder()
            .title(&rule.pattern)
            .subtitle(browser_name)
            .build();

        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let pattern_for_delete = rule.pattern.clone();
        let window_ref = window.clone();
        delete_btn.connect_clicked(move |_| {
            let mut config = config::load();
            config.rules.retain(|r| r.pattern != pattern_for_delete);
            if let Err(e) = config::save(&config) {
                eprintln!("silo: failed to save config: {e}");
            }
            window_ref.close();
        });

        row.add_suffix(&delete_btn);
        rules_group.add(&row);
    }

    if config.rules.is_empty() {
        let empty_row = adw::ActionRow::builder()
            .title("No rules yet")
            .subtitle("Use 'Always use for [domain]' in the picker to create rules")
            .build();
        rules_group.add(&empty_row);
    }

    rules_group
}

pub fn show(app: &adw::Application, config: &Config, browsers: &[BrowserEntry]) -> adw::PreferencesWindow {
    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("Silo")
        .default_width(550)
        .default_height(500)
        .build();

    // -- behaviour --

    let behaviour_page = adw::PreferencesPage::builder()
        .title("Behaviour")
        .icon_name("preferences-system-symbolic")
        .build();

    let always_ask_row = adw::SwitchRow::builder()
        .title("Always ask")
        .subtitle("Show picker for every link without a rule")
        .active(config.always_ask)
        .build();

    let fallback_row = adw::ComboRow::builder()
        .title("Fallback browser")
        .subtitle("Used when no rule matches and 'Always ask' is off")
        .build();

    let browser_names: Vec<String> = std::iter::once("None".to_string())
        .chain(browsers.iter().map(|b| b.display_name.clone()))
        .collect();
    let model = gtk::StringList::new(
        &browser_names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
    );
    fallback_row.set_model(Some(&model));

    if let Some(ref fb) = config.fallback_browser
        && let Some(pos) = browsers.iter().position(|b| {
            b.desktop_file == fb.desktop_file
                && b.profile_args.as_deref() == fb.args.as_deref()
        }) {
            fallback_row.set_selected(pos as u32 + 1);
        }

    let behaviour_group = adw::PreferencesGroup::new();
    behaviour_group.add(&always_ask_row);
    behaviour_group.add(&fallback_row);
    behaviour_page.add(&behaviour_group);

    // -- registration status --

    if config.setup_declined {
        let status_group = adw::PreferencesGroup::new();
        let register_row = adw::ActionRow::builder()
            .title("Silo is not your default browser")
            .subtitle("Links from other apps will not be routed through Silo")
            .build();

        let register_btn = gtk::Button::builder()
            .label("Set as default")
            .valign(gtk::Align::Center)
            .css_classes(["suggested-action"])
            .build();

        let window_for_reg = window.clone();
        register_btn.connect_clicked(move |_| {
            match silo_core::register::set_default_browser() {
                Ok(previous) => {
                    let mut config = config::load();
                    config.setup_declined = false;
                    config.previous_default_browser = previous;
                    let _ = config::save(&config);
                    window_for_reg.close();
                }
                Err(e) => {
                    eprintln!("silo: {e}");
                }
            }
        });

        register_row.add_suffix(&register_btn);
        status_group.add(&register_row);
        behaviour_page.add(&status_group);
    }

    // -- rules --

    let rules_page = adw::PreferencesPage::builder()
        .title("Rules")
        .icon_name("view-list-symbolic")
        .build();

    let rules_group = build_rules_group(browsers, config, &window);
    rules_page.add(&rules_group);

    let current_rules_group = Rc::new(RefCell::new(rules_group));

    // -- about --

    let about_page = adw::PreferencesPage::builder()
        .title("About")
        .icon_name("help-about-symbolic")
        .build();

    let info_group = adw::PreferencesGroup::builder()
        .title("Silo 1.0.0")
        .description("Browser picker with profile support. MIT licence.")
        .build();

    let github_row = adw::ActionRow::builder()
        .title("GitHub")
        .subtitle("github.com/no-faff/silo")
        .activatable(true)
        .build();
    github_row.add_prefix(&gtk::Image::from_icon_name("starred-symbolic"));
    github_row.add_suffix(&gtk::Image::from_icon_name("external-link-symbolic"));
    github_row.connect_activated(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/no-faff/silo")
            .spawn();
    });

    let issue_row = adw::ActionRow::builder()
        .title("Report an issue")
        .activatable(true)
        .build();
    issue_row.add_prefix(&gtk::Image::from_icon_name("mail-send-symbolic"));
    issue_row.add_suffix(&gtk::Image::from_icon_name("external-link-symbolic"));
    issue_row.connect_activated(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/no-faff/silo/issues")
            .spawn();
    });

    let donate_row = adw::ActionRow::builder()
        .title("Buy me a cuppa")
        .activatable(true)
        .build();
    donate_row.add_prefix(&gtk::Image::from_icon_name("emblem-favorite-symbolic"));
    donate_row.add_suffix(&gtk::Image::from_icon_name("external-link-symbolic"));
    donate_row.connect_activated(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://nofaff.netlify.app")
            .spawn();
    });

    info_group.add(&github_row);
    info_group.add(&issue_row);
    info_group.add(&donate_row);
    about_page.add(&info_group);

    let uninstall_group = adw::PreferencesGroup::new();
    let uninstall_row = adw::ActionRow::builder()
        .title("Uninstall Silo")
        .subtitle("Remove Silo and restore your previous default browser")
        .activatable(true)
        .build();
    uninstall_row.add_prefix(&gtk::Image::from_icon_name("user-trash-symbolic"));

    let window_for_uninstall = window.clone();
    uninstall_row.connect_activated(move |_| {
        let dialog = adw::AlertDialog::builder()
            .heading("Uninstall Silo?")
            .body("This will remove Silo, its config and rules, and restore your previous default browser.")
            .build();
        dialog.add_responses(&[("cancel", "Cancel"), ("uninstall", "Uninstall")]);
        dialog.set_response_appearance("uninstall", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));

        let win = window_for_uninstall.clone();
        dialog.connect_response(None, move |_, response| {
            if response == "uninstall" {
                match silo_core::register::uninstall() {
                    Ok(()) => {
                        eprintln!("silo: uninstalled");
                        win.close();
                        std::process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("silo: uninstall failed: {e}");
                    }
                }
            }
        });
        dialog.present(Some(&window_for_uninstall));
    });

    uninstall_group.add(&uninstall_row);
    about_page.add(&uninstall_group);

    window.add(&behaviour_page);
    window.add(&rules_page);
    window.add(&about_page);

    // Refresh rules when window regains focus (e.g. after picker saves a new rule)
    {
        let rules_page = rules_page.clone();
        let current_rules_group = current_rules_group.clone();
        let browsers = browsers.to_vec();
        window.connect_notify_local(Some("is-active"), move |win, _| {
            if !win.is_active() {
                return;
            }
            let config = config::load();
            let old_group = current_rules_group.borrow().clone();
            rules_page.remove(&old_group);
            let new_group = build_rules_group(&browsers, &config, win);
            rules_page.add(&new_group);
            *current_rules_group.borrow_mut() = new_group;
        });
    }

    let browsers_clone = browsers.to_vec();
    window.connect_close_request(move |_| {
        let mut config = config::load();
        config.always_ask = always_ask_row.is_active();

        let selected = fallback_row.selected();
        config.fallback_browser = if selected == 0 {
            None
        } else {
            browsers_clone.get(selected as usize - 1).map(|b| {
                silo_core::config::BrowserRef {
                    desktop_file: b.desktop_file.clone(),
                    args: b.profile_args.clone(),
                }
            })
        };

        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }

        gtk::glib::Propagation::Proceed
    });

    window.present();

    window
}
