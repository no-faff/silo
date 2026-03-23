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
    silo_core::register::install_launcher_entry();

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
        Some(url) => {
            if app.windows().is_empty() {
                // fresh launch, handle immediately
                handle_url(app, url);
            } else {
                // D-Bus activation while already running, defer to
                // avoid focus issues with existing windows
                let app = app.clone();
                let url = url.clone();
                gtk::glib::idle_add_local_once(move || {
                    handle_url(&app, &url);
                });
            }
        }
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
    let browsers = silo_core::browser::discover_with_config(&config);
    crate::settings::show(app, &config, &browsers);
}

fn handle_url(app: &adw::Application, url: &str) {
    // close any existing picker windows (but not the settings PreferencesWindow)
    for win in app.windows() {
        if win.downcast_ref::<adw::PreferencesWindow>().is_none() {
            win.destroy();
        }
    }

    if !silo_core::config::exists() {
        crate::first_run::show(app, Some(url.to_string()));
        return;
    }

    let config = silo_core::config::load();
    let processed = silo_core::url::process_url(url);

    if processed.domain.is_none() {
        show_error_dialog(app, "Cannot open link", &format!("Received a malformed URL:\n\n{url}"));
        return;
    }

    let browsers = silo_core::browser::discover_with_config(&config);

    if browsers.is_empty() {
        show_error_dialog(app, "No browsers found", "Install a browser and try again.");
        return;
    }

    // Use the unwrapped URL for launching
    let launch_url = &processed.final_url;

    if browsers.len() == 1 {
        if let Err(e) = silo_core::launcher::launch(&browsers[0], launch_url) {
            eprintln!("silo: {e}");
        }
        return;
    }

    let domain = processed.domain.as_deref().unwrap_or("");

    // Check rules (unless suspended)
    if !config.rules_suspended
        && let Some(rule) = silo_core::rule::find_matching_rule(&config.rules, domain, &processed.path) {
            match &rule.browser {
                Some(browser) => {
                    // Normal rule: launch matching browser
                    if let Some(entry) = browsers.iter().find(|b| {
                        b.desktop_file == browser.desktop_file
                            && b.profile_args.as_deref() == browser.args.as_deref()
                    }) {
                        if let Err(e) = silo_core::launcher::launch(entry, launch_url) {
                            eprintln!("silo: {e}");
                        }
                        return;
                    }
                    // Browser not found (stale rule), fall through to picker
                }
                None => {
                    // Exception rule: always show picker (fall through)
                }
            }
        }

    if !config.always_ask
        && let Some(ref fallback) = config.fallback_browser
            && let Some(entry) = browsers.iter().find(|b| {
                b.desktop_file == fallback.desktop_file
                    && b.profile_args.as_deref() == fallback.args.as_deref()
            }) {
                if let Err(e) = silo_core::launcher::launch(entry, launch_url) {
                    eprintln!("silo: {e}");
                }
                return;
            }

    crate::picker::show(app, launch_url, Some(domain), &browsers, &config, processed.was_redirected, processed.office_doc);
}

pub fn show_error_dialog(app: &adw::Application, heading: &str, body: &str) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Silo")
        .default_width(360)
        .default_height(200)
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