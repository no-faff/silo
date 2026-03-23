use silo_core::config;

/// On first run (no config), shows settings on the Welcome tab.
/// If a URL was passed, shows the picker behind it.
pub fn show(app: &adw::Application, url: Option<String>) {
    let browsers = silo_core::browser::discover();

    // Record the current default browser before we replace it
    let previous = std::process::Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "com.nofaff.Silo.desktop");

    // Save a default config with the previous browser recorded.
    let default_config = config::Config {
        previous_default_browser: previous,
        ..config::Config::default()
    };
    if let Err(e) = config::save(&default_config) {
        eprintln!("silo: failed to save config: {e}");
    }

    if let Some(ref url) = url {
        if browsers.is_empty() {
            crate::app::show_error_dialog(
                app,
                "No browsers found",
                "Install a browser and try again.",
            );
            return;
        }
        let processed = silo_core::url::process_url(url);
        crate::picker::show(app, &processed.final_url, processed.domain.as_deref(), &browsers, &default_config, processed.was_redirected, processed.office_doc);
    }

    crate::settings::show_on_page(app, &default_config, &browsers, Some("welcome"));
}
