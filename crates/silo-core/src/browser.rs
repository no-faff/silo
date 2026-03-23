use crate::desktop;

#[derive(Debug, Clone)]
pub struct BrowserEntry {
    pub desktop_file: String,
    pub display_name: String,
    pub icon: String,
    pub profile_args: Option<String>,
    pub exec: String,
}

enum BrowserFamily {
    Chromium,
    Firefox,
    Unknown,
}

fn detect_family(desktop_id: &str) -> BrowserFamily {
    let id = desktop_id.to_lowercase();

    let chromium_ids = [
        "google-chrome",
        "chromium",
        "brave",
        "vivaldi",
        "microsoft-edge",
        "opera",
        "com.brave.browser",
        "com.google.chrome",
        "org.chromium.chromium",
        "com.vivaldi.vivaldi",
    ];
    let firefox_ids = [
        "firefox",
        "librewolf",
        "waterfox",
        "floorp",
        "zen-browser",
        "mullvad-browser",
        "org.mozilla.firefox",
        "io.gitlab.librewolf-community",
        "one.ablaze.floorp",
    ];

    for known in &chromium_ids {
        if id.contains(known) {
            return BrowserFamily::Chromium;
        }
    }
    for known in &firefox_ids {
        if id.contains(known) {
            return BrowserFamily::Firefox;
        }
    }

    BrowserFamily::Unknown
}

/// Returns (profile_directory, display_name) pairs.
pub fn detect_chromium_profiles(local_state_path: &std::path::Path) -> Vec<(String, String)> {
    let content = match std::fs::read_to_string(local_state_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let info_cache = match json.get("profile").and_then(|p| p.get("info_cache")) {
        Some(serde_json::Value::Object(map)) => map,
        _ => return Vec::new(),
    };

    info_cache
        .iter()
        .filter_map(|(dir, info)| {
            let name = info.get("name")?.as_str()?;
            Some((dir.clone(), name.to_string()))
        })
        .collect()
}

/// Returns (profile_name for -P flag, display_name) pairs.
pub fn detect_firefox_profiles(profiles_ini_path: &std::path::Path) -> Vec<(String, String)> {
    let mut parser = configparser::ini::Ini::new();
    if parser.load(profiles_ini_path).is_err() {
        return Vec::new();
    }

    let mut profiles = Vec::new();

    for section in parser.sections() {
        if !section.to_lowercase().starts_with("profile") {
            continue;
        }
        // skip bare [profile] without a number
        if section.len() <= "profile".len() {
            continue;
        }

        if let Some(name) = parser.get(&section, "name") {
            profiles.push((name.clone(), name));
        }
    }

    profiles
}

fn desktop_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_dirs.split(':') {
            dirs.push(std::path::PathBuf::from(dir).join("applications"));
        }
    } else {
        dirs.push("/usr/share/applications".into());
        dirs.push("/usr/local/share/applications".into());
    }

    if let Some(data_home) = dirs::data_dir() {
        dirs.push(data_home.join("applications"));
    }

    // flatpak
    dirs.push("/var/lib/flatpak/exports/share/applications".into());
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    // snap
    dirs.push("/var/lib/snapd/desktop/applications".into());

    dirs
}

fn chromium_config_dirs(desktop_id: &str) -> Vec<&'static str> {
    let id = desktop_id.to_lowercase();
    if id.contains("google-chrome") || id.contains("google.chrome") {
        vec!["google-chrome", "google-chrome-stable"]
    } else if id.contains("chromium") {
        vec!["chromium"]
    } else if id.contains("brave") {
        vec!["BraveSoftware/Brave-Browser"]
    } else if id.contains("vivaldi") {
        vec!["vivaldi"]
    } else if id.contains("microsoft-edge") {
        vec!["microsoft-edge"]
    } else if id.contains("opera") {
        vec!["opera"]
    } else {
        vec![]
    }
}

fn firefox_config_dirs(desktop_id: &str) -> Vec<&'static str> {
    let id = desktop_id.to_lowercase();
    if id.contains("firefox") {
        vec![".mozilla/firefox"]
    } else if id.contains("librewolf") {
        vec![".librewolf"]
    } else if id.contains("waterfox") {
        vec![".waterfox"]
    } else if id.contains("floorp") {
        vec![".floorp"]
    } else if id.contains("zen") {
        vec![".zen"]
    } else if id.contains("mullvad") {
        vec![".mullvad"]
    } else {
        vec![]
    }
}

/// Scans .desktop file dirs and expands profiles for known browser families.
pub fn discover() -> Vec<BrowserEntry> {
    let mut entries = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for dir in desktop_dirs() {
        let read_dir = match std::fs::read_dir(&dir) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop")
                && let Some(de) = desktop::parse(&path)
                    && de.is_browser() && seen_ids.insert(de.id.clone()) {
                        expand_entry(&de, &mut entries);
                    }
        }
    }

    entries
}

/// Returns the Flatpak app ID for a desktop file, if it looks like a Flatpak ID.
/// Strips the `.desktop` suffix: `com.google.Chrome.desktop` -> `com.google.Chrome`.
fn flatpak_id(desktop_id: &str) -> Option<&str> {
    let id = desktop_id.strip_suffix(".desktop")?;
    // Flatpak IDs have at least two dots (e.g. org.mozilla.firefox)
    if id.matches('.').count() >= 2 {
        Some(id)
    } else {
        None
    }
}

/// Returns the snap package name for known snap browsers, if any.
fn snap_name(desktop_id: &str) -> Option<&'static str> {
    let id = desktop_id.to_lowercase();
    if id.contains("firefox") {
        Some("firefox")
    } else if id.contains("chromium") {
        Some("chromium")
    } else {
        None
    }
}

/// Builds a list of candidate paths for a Chromium `Local State` file:
/// standard, Flatpak and Snap locations.
fn chromium_local_state_paths(
    desktop_id: &str,
    config_home: &std::path::Path,
    home: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    let config_dirs = chromium_config_dirs(desktop_id);
    let mut paths = Vec::new();

    // Standard paths
    for dir_name in &config_dirs {
        paths.push(config_home.join(dir_name).join("Local State"));
    }

    // Flatpak paths: ~/.var/app/{flatpak_id}/config/{config_dir_name}/Local State
    if let Some(fp_id) = flatpak_id(desktop_id) {
        for dir_name in &config_dirs {
            paths.push(
                home.join(".var/app")
                    .join(fp_id)
                    .join("config")
                    .join(dir_name)
                    .join("Local State"),
            );
        }
    }

    // Snap paths: ~/snap/{snap_name}/current/.config/{config_dir_name}/Local State
    if let Some(snap) = snap_name(desktop_id) {
        for dir_name in &config_dirs {
            paths.push(
                home.join("snap")
                    .join(snap)
                    .join("current/.config")
                    .join(dir_name)
                    .join("Local State"),
            );
        }
    }

    paths
}

/// Builds a list of candidate paths for a Firefox `profiles.ini` file:
/// standard, Flatpak and Snap locations.
fn firefox_profiles_ini_paths(
    desktop_id: &str,
    home: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    let config_dirs = firefox_config_dirs(desktop_id);
    let mut paths = Vec::new();

    // Standard paths
    for dir_name in &config_dirs {
        paths.push(home.join(dir_name).join("profiles.ini"));
    }

    // Flatpak paths: ~/.var/app/{flatpak_id}/{firefox_config_dir}/profiles.ini
    if let Some(fp_id) = flatpak_id(desktop_id) {
        for dir_name in &config_dirs {
            paths.push(
                home.join(".var/app")
                    .join(fp_id)
                    .join(dir_name)
                    .join("profiles.ini"),
            );
        }
    }

    // Snap paths: ~/snap/{snap_name}/current/{firefox_config_dir}/profiles.ini
    if let Some(snap) = snap_name(desktop_id) {
        for dir_name in &config_dirs {
            paths.push(
                home.join("snap")
                    .join(snap)
                    .join("current")
                    .join(dir_name)
                    .join("profiles.ini"),
            );
        }
    }

    paths
}

fn expand_entry(de: &desktop::DesktopEntry, entries: &mut Vec<BrowserEntry>) {
    let home = dirs::home_dir().unwrap_or_default();
    let config_home = dirs::config_dir().unwrap_or_else(|| home.join(".config"));

    match detect_family(&de.id) {
        BrowserFamily::Chromium => {
            let mut found_profiles = false;
            for local_state in chromium_local_state_paths(&de.id, &config_home, &home) {
                let profiles = detect_chromium_profiles(&local_state);
                if !profiles.is_empty() {
                    found_profiles = true;
                    for (dir, name) in profiles {
                        entries.push(BrowserEntry {
                            desktop_file: de.id.clone(),
                            display_name: format!("{} - {}", de.name, name),
                            icon: de.icon.clone(),
                            profile_args: Some(format!("'--profile-directory={dir}'")),
                            exec: de.exec.clone(),
                        });
                    }
                    break;
                }
            }
            if !found_profiles {
                entries.push(single_entry(de));
            }
        }
        BrowserFamily::Firefox => {
            let mut found_profiles = false;
            for profiles_ini in firefox_profiles_ini_paths(&de.id, &home) {
                let profiles = detect_firefox_profiles(&profiles_ini);
                if !profiles.is_empty() {
                    found_profiles = true;
                    for (profile_name, display_name) in profiles {
                        entries.push(BrowserEntry {
                            desktop_file: de.id.clone(),
                            display_name: format!("{} - {}", de.name, display_name),
                            icon: de.icon.clone(),
                            profile_args: Some(format!("-P '{profile_name}'")),
                            exec: de.exec.clone(),
                        });
                    }
                    break;
                }
            }
            if !found_profiles {
                entries.push(single_entry(de));
            }
        }
        BrowserFamily::Unknown => {
            entries.push(single_entry(de));
        }
    }
}

/// Like discover(), but applies overrides (hiding, name changes) and appends
/// custom browsers from the config.
pub fn discover_with_config(config: &crate::config::Config) -> Vec<BrowserEntry> {
    let mut entries = discover();

    // Apply overrides
    for entry in &mut entries {
        if let Some(ov) = config.browser_overrides.iter().find(|o| {
            o.desktop_file == entry.desktop_file
                && o.profile_args.as_deref() == entry.profile_args.as_deref()
        }) {
            if let Some(ref name) = ov.display_name {
                entry.display_name = name.clone();
            }
        }
    }

    // Remove hidden browsers
    entries.retain(|e| {
        !config.browser_overrides.iter().any(|o| {
            o.desktop_file == e.desktop_file
                && o.profile_args.as_deref() == e.profile_args.as_deref()
                && o.hidden
        })
    });

    // Append custom browsers
    for cb in &config.custom_browsers {
        entries.push(BrowserEntry {
            desktop_file: format!("custom:{}", cb.name),
            display_name: cb.name.clone(),
            icon: "application-x-executable".to_string(),
            profile_args: cb.args.clone(),
            exec: cb.command.clone(),
        });
    }

    entries
}

fn single_entry(de: &desktop::DesktopEntry) -> BrowserEntry {
    BrowserEntry {
        desktop_file: de.id.clone(),
        display_name: de.name.clone(),
        icon: de.icon.clone(),
        profile_args: None,
        exec: de.exec.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn detect_chromium_profiles_from_local_state() {
        let profiles = detect_chromium_profiles(&fixture("chromium-local-state.json"));
        assert_eq!(profiles.len(), 2);
        assert!(profiles
            .iter()
            .any(|(dir, name)| dir == "Default" && name == "Personal"));
        assert!(profiles
            .iter()
            .any(|(dir, name)| dir == "Profile 1" && name == "Work"));
    }

    #[test]
    fn detect_firefox_profiles_from_ini() {
        let profiles = detect_firefox_profiles(&fixture("firefox-profiles.ini"));
        assert_eq!(profiles.len(), 2);
        assert!(profiles
            .iter()
            .any(|(id, name)| id == "default-release" && name == "default-release"));
        assert!(profiles
            .iter()
            .any(|(id, name)| id == "Work" && name == "Work"));
    }

    #[test]
    fn detect_chromium_profiles_missing_file() {
        let profiles = detect_chromium_profiles(&PathBuf::from("/nonexistent"));
        assert!(profiles.is_empty());
    }

    #[test]
    fn detect_firefox_profiles_missing_file() {
        let profiles = detect_firefox_profiles(&PathBuf::from("/nonexistent"));
        assert!(profiles.is_empty());
    }

    #[test]
    fn detect_family_chrome() {
        assert!(matches!(
            detect_family("google-chrome.desktop"),
            BrowserFamily::Chromium
        ));
    }

    #[test]
    fn detect_family_firefox() {
        assert!(matches!(
            detect_family("firefox.desktop"),
            BrowserFamily::Firefox
        ));
    }

    #[test]
    fn detect_family_unknown() {
        assert!(matches!(
            detect_family("someapp.desktop"),
            BrowserFamily::Unknown
        ));
    }

    #[test]
    fn flatpak_id_extracts_correctly() {
        assert_eq!(flatpak_id("com.google.Chrome.desktop"), Some("com.google.Chrome"));
        assert_eq!(flatpak_id("org.mozilla.firefox.desktop"), Some("org.mozilla.firefox"));
        assert_eq!(flatpak_id("firefox.desktop"), None);
        assert_eq!(flatpak_id("google-chrome.desktop"), None);
    }

    #[test]
    fn snap_name_known_browsers() {
        assert_eq!(snap_name("firefox.desktop"), Some("firefox"));
        assert_eq!(snap_name("chromium-browser.desktop"), Some("chromium"));
        assert_eq!(snap_name("google-chrome.desktop"), None);
    }

    #[test]
    fn chromium_local_state_paths_standard_and_flatpak() {
        let home = PathBuf::from("/home/test");
        let config_home = home.join(".config");
        let paths = chromium_local_state_paths(
            "com.google.Chrome.desktop",
            &config_home,
            &home,
        );
        // Standard paths
        assert!(paths.contains(
            &PathBuf::from("/home/test/.config/google-chrome/Local State")
        ));
        // Flatpak paths
        assert!(paths.contains(
            &PathBuf::from("/home/test/.var/app/com.google.Chrome/config/google-chrome/Local State")
        ));
    }

    #[test]
    fn chromium_local_state_paths_snap() {
        let home = PathBuf::from("/home/test");
        let config_home = home.join(".config");
        let paths = chromium_local_state_paths(
            "chromium-browser.desktop",
            &config_home,
            &home,
        );
        assert!(paths.contains(
            &PathBuf::from("/home/test/.config/chromium/Local State")
        ));
        assert!(paths.contains(
            &PathBuf::from("/home/test/snap/chromium/current/.config/chromium/Local State")
        ));
    }

    #[test]
    fn firefox_profiles_ini_paths_standard_and_flatpak() {
        let home = PathBuf::from("/home/test");
        let paths = firefox_profiles_ini_paths(
            "org.mozilla.firefox.desktop",
            &home,
        );
        // Standard
        assert!(paths.contains(
            &PathBuf::from("/home/test/.mozilla/firefox/profiles.ini")
        ));
        // Flatpak
        assert!(paths.contains(
            &PathBuf::from("/home/test/.var/app/org.mozilla.firefox/.mozilla/firefox/profiles.ini")
        ));
    }

    #[test]
    fn firefox_profiles_ini_paths_snap() {
        let home = PathBuf::from("/home/test");
        let paths = firefox_profiles_ini_paths(
            "firefox.desktop",
            &home,
        );
        assert!(paths.contains(
            &PathBuf::from("/home/test/.mozilla/firefox/profiles.ini")
        ));
        assert!(paths.contains(
            &PathBuf::from("/home/test/snap/firefox/current/.mozilla/firefox/profiles.ini")
        ));
    }

    #[test]
    fn flatpak_brave_paths() {
        let home = PathBuf::from("/home/test");
        let config_home = home.join(".config");
        let paths = chromium_local_state_paths(
            "com.brave.Browser.desktop",
            &config_home,
            &home,
        );
        assert!(paths.contains(
            &PathBuf::from("/home/test/.config/BraveSoftware/Brave-Browser/Local State")
        ));
        assert!(paths.contains(
            &PathBuf::from("/home/test/.var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser/Local State")
        ));
    }

    #[test]
    fn flatpak_librewolf_paths() {
        let home = PathBuf::from("/home/test");
        let paths = firefox_profiles_ini_paths(
            "io.gitlab.librewolf-community.desktop",
            &home,
        );
        assert!(paths.contains(
            &PathBuf::from("/home/test/.librewolf/profiles.ini")
        ));
        assert!(paths.contains(
            &PathBuf::from("/home/test/.var/app/io.gitlab.librewolf-community/.librewolf/profiles.ini")
        ));
    }
}
