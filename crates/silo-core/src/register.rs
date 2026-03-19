use std::path::PathBuf;
use std::process::Command;

const DESKTOP_FILENAME: &str = "com.nofaff.Silo.desktop";
const ICON_BYTES: &[u8] = include_bytes!("../../../data/icons/hicolor/128x128/apps/com.nofaff.Silo.png");

fn desktop_install_path() -> PathBuf {
    dirs::data_dir()
        .expect("could not determine XDG_DATA_HOME")
        .join("applications")
        .join(DESKTOP_FILENAME)
}

fn icon_install_path() -> PathBuf {
    dirs::data_dir()
        .expect("could not determine XDG_DATA_HOME")
        .join("icons/hicolor/128x128/apps/com.nofaff.Silo.png")
}

/// Writes the .desktop file to ~/.local/share/applications/ pointing at
/// the current binary, then updates the desktop database.
pub fn install_desktop_file() -> Result<(), String> {
    let binary = std::env::current_exe()
        .map_err(|e| format!("could not find own binary path: {e}"))?;

    let contents = format!(
        "[Desktop Entry]\n\
         Name=Silo\n\
         Comment=Browser picker with profile support\n\
         Exec={} %u\n\
         Icon=com.nofaff.Silo\n\
         Type=Application\n\
         Categories=Network;Utility;\n\
         MimeType=x-scheme-handler/http;x-scheme-handler/https;\n\
         StartupNotify=false\n",
        binary.display()
    );

    let dest = desktop_install_path();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }

    let tmp = dest.with_extension("desktop.tmp");
    std::fs::write(&tmp, contents)
        .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &dest)
        .map_err(|e| format!("could not rename to {}: {e}", dest.display()))?;

    // best-effort, not fatal if missing
    let _ = Command::new("update-desktop-database")
        .arg(dest.parent().unwrap())
        .status();

    // install icon
    let icon_dest = icon_install_path();
    if let Some(parent) = icon_dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&icon_dest, ICON_BYTES);
    let _ = Command::new("gtk-update-icon-cache")
        .arg(dirs::data_dir().unwrap().join("icons/hicolor"))
        .status();

    Ok(())
}

/// Installs the .desktop file then registers Silo as default browser.
/// Returns the previous default browser's .desktop ID, if any.
pub fn set_default_browser() -> Result<Option<String>, String> {
    install_desktop_file()?;

    let previous = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let status = Command::new("xdg-settings")
        .args(["set", "default-web-browser", DESKTOP_FILENAME])
        .status()
        .map_err(|e| format!("could not run xdg-settings: {e}"))?;

    if !status.success() {
        return Err("xdg-settings set failed".to_string());
    }

    Ok(previous)
}

/// Removes the .desktop file, restores the previous default browser,
/// deletes config, and removes the binary.
pub fn uninstall() -> Result<(), String> {
    let config = crate::config::load();

    // restore previous default browser
    let current = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if current == DESKTOP_FILENAME {
        if let Some(ref previous) = config.previous_default_browser {
            let _ = Command::new("xdg-settings")
                .args(["set", "default-web-browser", previous])
                .status();
        }
    }

    // remove .desktop file
    let desktop = desktop_install_path();
    let _ = std::fs::remove_file(&desktop);
    if let Some(parent) = desktop.parent() {
        let _ = Command::new("update-desktop-database")
            .arg(parent)
            .status();
    }

    // remove icon
    let _ = std::fs::remove_file(icon_install_path());

    // remove config
    let config_dir = crate::config::config_path()
        .parent()
        .map(|p| p.to_path_buf());
    if let Some(dir) = config_dir {
        let _ = std::fs::remove_dir_all(&dir);
    }

    // remove the binary itself
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(&exe);
    }

    Ok(())
}
