use adw::prelude::*;
use silo_core::config;

/// Shows the picker (or settings), then overlays the
/// "set as default browser" dialog on top.
pub fn show(app: &adw::Application, url: Option<String>) {
    let browsers = silo_core::browser::discover();
    let default_config = config::Config::default();

    let dialog = build_dialog();

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
        let picker = crate::picker::show(app, &processed.final_url, processed.domain.as_deref(), &browsers, &default_config, processed.was_redirected);
        wire_response(dialog.clone());
        dialog.present(Some(&picker));
    } else {
        let settings = crate::settings::show(app, &default_config, &browsers);
        wire_response(dialog.clone());
        dialog.present(Some(&settings));
    };
}

fn build_dialog() -> adw::AlertDialog {
    let dialog = adw::AlertDialog::builder()
        .heading("Set as default browser?")
        .body("Silo needs to be your default browser to work.")
        .build();

    dialog.add_responses(&[("decline", "Not now"), ("accept", "Set as default")]);
    dialog.set_response_appearance("accept", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("accept"));

    dialog
}

fn wire_response(dialog: adw::AlertDialog) {
    dialog.connect_response(None, move |_dialog, response| {
        if response == "accept" {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                tx.send(silo_core::register::set_default_browser()).ok();
            });

            gtk::glib::idle_add_local(move || match rx.try_recv() {
                Ok(result) => {
                    let mut new_config = config::Config::default();
                    match result {
                        Ok(previous) => {
                            new_config.previous_default_browser = previous;
                            eprintln!("silo: registered as default browser");
                        }
                        Err(e) => eprintln!("silo: {e}"),
                    }
                    let _ = config::save(&new_config);
                    gtk::glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                Err(_) => gtk::glib::ControlFlow::Break,
            });
        } else {
            let new_config = config::Config {
                setup_declined: true,
                ..config::Config::default()
            };
            let _ = config::save(&new_config);
        }
    });
}
