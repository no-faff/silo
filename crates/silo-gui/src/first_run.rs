use silo_core::config;

/// On first run (no config), shows settings on the Welcome tab.
/// If a URL was passed, shows the picker behind it.
pub fn show(app: &adw::Application, url: Option<String>) {
    let browsers = silo_core::browser::discover();

    // Record the current default browser before we replace it
    let current = silo_core::register::get_default_browser();
    let previous = if current.is_empty() || current == "com.nofaff.Silo.desktop" {
        None
    } else {
        Some(current)
    };

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
        let is_file_url = url.starts_with("file://");
        if processed.domain.is_none() && !is_file_url {
            crate::app::show_error_dialog(app, "Cannot open link", &format!("Received a malformed URL:\n\n{url}"));
        } else {
            let (heading, hint, rule_pattern) = if is_file_url {
                let path = std::path::Path::new(processed.path.as_str());
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Local file");
                let ext = path.extension().and_then(|e| e.to_str());
                let heading = ext.map(|e| format!(".{e} files"));
                let hint = format!("Choose a browser to open {filename}. Esc to close.");
                let pattern = ext.map(|e| format!("ext:{e}")).unwrap_or_default();
                (heading, Some(hint), Some(pattern))
            } else {
                (processed.domain.as_deref().map(|s| s.to_string()), None, None)
            };
            crate::picker::show(app, &processed.final_url, heading.as_deref(), hint.as_deref(), &browsers, &default_config, processed.was_redirected, processed.office_doc, rule_pattern.as_deref());
        }
    }

    crate::settings::show_on_page(app, &default_config, &browsers, Some("welcome"));
}
