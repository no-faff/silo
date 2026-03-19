use adw::prelude::*;
use gtk::gio;

pub const APP_ID: &str = "com.nofaff.Silo";

/// Uses HANDLES_COMMAND_LINE rather than HANDLES_OPEN because the latter
/// mangles URLs into gio::File paths.
pub fn run() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(on_command_line);

    app.run()
}

fn on_command_line(
    app: &adw::Application,
    cmd: &gio::ApplicationCommandLine,
) -> gtk::glib::ExitCode {
    let args = cmd.arguments();
    let args: Vec<String> = args
        .iter()
        .skip(1)
        .filter_map(|a| a.to_str().map(|s| s.to_string()))
        .collect();

    if args.iter().any(|a| a == "--settings") {
        show_settings_or_first_run(app, None);
        return gtk::glib::ExitCode::SUCCESS;
    }

    let url = args.iter().find(|a| {
        a.starts_with("http://") || a.starts_with("https://") || a.contains("://")
    });

    match url {
        Some(url) => handle_url(app, url),
        None => show_settings_or_first_run(app, None),
    }

    gtk::glib::ExitCode::SUCCESS
}

fn show_settings_or_first_run(app: &adw::Application, then_url: Option<String>) {
    if !silo_core::config::exists() {
        crate::first_run::show(app, then_url);
        return;
    }

    let config = silo_core::config::load();
    let browsers = silo_core::browser::discover();
    crate::settings::show(app, &config, &browsers);
}

fn handle_url(app: &adw::Application, url: &str) {
    if !silo_core::config::exists() {
        crate::first_run::show(app, Some(url.to_string()));
        return;
    }

    let config = silo_core::config::load();
    let domain = silo_core::url::extract_domain(url);

    if domain.is_none() {
        show_error_dialog(app, "Cannot open link", &format!("Received a malformed URL:\n\n{url}"));
        return;
    }

    let browsers = silo_core::browser::discover();

    if browsers.is_empty() {
        show_error_dialog(app, "No browsers found", "Install a browser and try again.");
        return;
    }

    if browsers.len() == 1 {
        if let Err(e) = silo_core::launcher::launch(&browsers[0], url) {
            eprintln!("silo: {e}");
        }
        return;
    }

    if let Some(ref domain) = domain {
        if let Some(rule) = silo_core::rule::find_matching_rule(&config.rules, domain) {
            if let Some(entry) = browsers.iter().find(|b| {
                b.desktop_file == rule.browser.desktop_file
                    && b.profile_args.as_deref() == rule.browser.args.as_deref()
            }) {
                if let Err(e) = silo_core::launcher::launch(entry, url) {
                    eprintln!("silo: {e}");
                }
                return;
            }
        }
    }

    if !config.always_ask {
        if let Some(ref fallback) = config.fallback_browser {
            if let Some(entry) = browsers.iter().find(|b| {
                b.desktop_file == fallback.desktop_file
                    && b.profile_args.as_deref() == fallback.args.as_deref()
            }) {
                if let Err(e) = silo_core::launcher::launch(entry, url) {
                    eprintln!("silo: {e}");
                }
                return;
            }
        }
    }

    crate::picker::show(app, url, domain.as_deref(), &browsers, &config);
}

pub fn show_error_dialog(app: &adw::Application, heading: &str, body: &str) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Silo")
        .default_width(1)
        .default_height(1)
        .build();
    window.present();

    let dialog = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    dialog.add_responses(&[("close", "Dismiss")]);
    let window_ref = window.clone();
    dialog.connect_response(None, move |_, _| {
        window_ref.close();
    });
    dialog.present(Some(&window));
}
