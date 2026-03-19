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
    if id.contains("google-chrome") {
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
            if path.extension().is_some_and(|e| e == "desktop") {
                if let Some(de) = desktop::parse(&path) {
                    if de.is_browser() && seen_ids.insert(de.id.clone()) {
                        expand_entry(&de, &mut entries);
                    }
                }
            }
        }
    }

    entries
}

fn expand_entry(de: &desktop::DesktopEntry, entries: &mut Vec<BrowserEntry>) {
    let home = dirs::home_dir().unwrap_or_default();
    let config_home = dirs::config_dir().unwrap_or_else(|| home.join(".config"));

    match detect_family(&de.id) {
        BrowserFamily::Chromium => {
            let mut found_profiles = false;
            for config_dir_name in chromium_config_dirs(&de.id) {
                let local_state = config_home.join(config_dir_name).join("Local State");
                let profiles = detect_chromium_profiles(&local_state);
                if !profiles.is_empty() {
                    found_profiles = true;
                    for (dir, name) in profiles {
                        entries.push(BrowserEntry {
                            desktop_file: de.id.clone(),
                            display_name: format!("{} - {}", de.name, name),
                            icon: de.icon.clone(),
                            profile_args: Some(format!("--profile-directory={dir}")),
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
            for config_dir_name in firefox_config_dirs(&de.id) {
                let profiles_ini = home.join(config_dir_name).join("profiles.ini");
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
}
