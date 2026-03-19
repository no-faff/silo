use adw::prelude::*;
use silo_core::browser::BrowserEntry;
use silo_core::config::{self, Config};

pub fn show(app: &adw::Application, config: &Config, browsers: &[BrowserEntry]) {
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

    if let Some(ref fb) = config.fallback_browser {
        if let Some(pos) = browsers.iter().position(|b| {
            b.desktop_file == fb.desktop_file
                && b.profile_args.as_deref() == fb.args.as_deref()
        }) {
            fallback_row.set_selected(pos as u32 + 1);
        }
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
            let result = std::process::Command::new("xdg-settings")
                .args(["set", "default-web-browser", "com.nofaff.Silo.desktop"])
                .status();

            match result {
                Ok(status) if status.success() => {
                    let mut config = config::load();
                    config.setup_declined = false;
                    let _ = config::save(&config);
                    window_for_reg.close();
                }
                _ => {
                    eprintln!("silo: failed to register as default browser");
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

    let rules_group = adw::PreferencesGroup::builder()
        .title("Domain rules")
        .description("Links matching these domains open silently in the assigned browser")
        .build();

    for rule in &config.rules {
        let browser_name = browsers
            .iter()
            .find(|b| {
                b.desktop_file == rule.browser.desktop_file
                    && b.profile_args.as_deref() == rule.browser.args.as_deref()
            })
            .map(|b| b.display_name.as_str())
            .unwrap_or(&rule.browser.desktop_file);

        let row = adw::ActionRow::builder()
            .title(&rule.domain)
            .subtitle(browser_name)
            .build();

        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let domain_for_delete = rule.domain.clone();
        let window_ref = window.clone();
        delete_btn.connect_clicked(move |_| {
            let mut config = config::load();
            config.rules.retain(|r| r.domain != domain_for_delete);
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

    rules_page.add(&rules_group);

    window.add(&behaviour_page);
    window.add(&rules_page);

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
}
