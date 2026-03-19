use adw::prelude::*;
use silo_core::config;
use std::process::Command;

pub fn show(app: &adw::Application, then_open_url: Option<String>) {
    let current_default = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let default_name = current_default.trim_end_matches(".desktop");
    let body = if current_default.is_empty() {
        "Silo needs to be your default browser to work.".to_string()
    } else {
        format!(
            "Silo needs to be your default browser to work.\n\n\
             Your current default is {default_name}. If you uninstall \
             Silo, you may need to set a new default browser in your \
             system settings."
        )
    };

    let dialog = adw::AlertDialog::builder()
        .heading("Set as default browser?")
        .body(&body)
        .build();

    dialog.add_responses(&[("decline", "Not now"), ("accept", "Set as default")]);
    dialog.set_response_appearance("accept", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("accept"));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Silo")
        .default_width(1)
        .default_height(1)
        .build();
    window.present();

    let app_clone = app.clone();
    let current_default_clone = current_default.clone();
    let window_clone = window.clone();

    dialog.connect_response(None, move |_dialog, response| {
        let mut new_config = config::Config::default();

        if response == "accept" {
            if !current_default_clone.is_empty() {
                new_config.previous_default_browser = Some(current_default_clone.clone());
            }

            let result = Command::new("xdg-settings")
                .args(["set", "default-web-browser", "com.nofaff.Silo.desktop"])
                .status();

            match result {
                Ok(status) if status.success() => {
                    eprintln!("silo: registered as default browser");
                }
                _ => {
                    eprintln!("silo: failed to register as default browser");
                }
            }
        } else {
            new_config.setup_declined = true;
        }

        if let Err(e) = config::save(&new_config) {
            eprintln!("silo: failed to save config: {e}");
        }

        window_clone.close();

        if let Some(ref url) = then_open_url {
            let browsers = silo_core::browser::discover();
            if browsers.is_empty() {
                return;
            }
            if browsers.len() == 1 {
                if let Err(e) = silo_core::launcher::launch(&browsers[0], url) {
                    eprintln!("silo: {e}");
                }
            } else {
                let domain = silo_core::url::extract_domain(url);
                crate::picker::show(&app_clone, url, domain.as_deref(), &browsers, &new_config);
            }
        }
    });

    dialog.present(Some(&window));
}
