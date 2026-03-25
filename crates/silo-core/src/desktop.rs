#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub mime_types: Vec<String>,
    pub no_display: bool,
}

impl DesktopEntry {
    pub fn is_browser(&self) -> bool {
        !self.no_display && self.mime_types.iter().any(|m| m == "x-scheme-handler/http")
    }

    /// First token of the Exec line (strips %u, %f etc).
    pub fn executable(&self) -> &str {
        self.exec.split_whitespace().next().unwrap_or(&self.exec)
    }
}

/// Strip common suffixes like "Web Browser" from browser names.
fn clean_browser_name(name: &str) -> String {
    let suffixes = [" Web Browser", " Browser"];
    let mut cleaned = name.to_string();
    for suffix in &suffixes {
        if let Some(stripped) = cleaned.strip_suffix(suffix) {
            if !stripped.is_empty() {
                cleaned = stripped.to_string();
                break;
            }
        }
    }
    cleaned
}

pub fn parse(path: &std::path::Path) -> Option<DesktopEntry> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut mime_types = Vec::new();
    let mut no_display = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            if in_desktop_entry {
                break;
            }
            continue;
        }
        if !in_desktop_entry {
            continue;
        }

        if let Some(val) = line.strip_prefix("Name=") {
            if name.is_none() {
                name = Some(val.to_string());
            }
        } else if let Some(val) = line.strip_prefix("Exec=") {
            exec = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("Icon=") {
            icon = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("MimeType=") {
            mime_types = val
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if line == "NoDisplay=true" {
            no_display = true;
        }
    }

    let id = path.file_name()?.to_str()?.to_string();

    let clean_name = clean_browser_name(&name?);

    Some(DesktopEntry {
        id,
        name: clean_name,
        exec: exec?,
        icon: icon.unwrap_or_default(),
        mime_types,
        no_display,
    })
}

/// Finds a .desktop file by its ID (e.g. "org.mozilla.firefox.desktop")
/// by searching the standard applications directories.
pub fn find_by_id(id: &str) -> Option<DesktopEntry> {
    let search_dirs = [
        dirs::data_dir().map(|d| d.join("applications")),
        Some(std::path::PathBuf::from("/usr/share/applications")),
        Some(std::path::PathBuf::from("/usr/local/share/applications")),
        dirs::home_dir().map(|d| d.join(".local/share/flatpak/exports/share/applications")),
        Some(std::path::PathBuf::from("/var/lib/flatpak/exports/share/applications")),
    ];

    for dir in search_dirs.into_iter().flatten() {
        let path = dir.join(id);
        if let Some(entry) = parse(&path) {
            return Some(entry);
        }
    }
    None
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
    fn parse_firefox_desktop() {
        let entry = parse(&fixture("firefox.desktop")).unwrap();
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.exec, "/usr/bin/firefox %u");
        assert_eq!(entry.icon, "firefox");
        assert_eq!(entry.id, "firefox.desktop");
        assert!(entry.is_browser());
    }

    #[test]
    fn parse_text_editor_is_not_browser() {
        let entry = parse(&fixture("text-editor.desktop")).unwrap();
        assert_eq!(entry.name, "Text Editor");
        assert!(!entry.is_browser());
    }

    #[test]
    fn parse_flatpak_browser() {
        let entry = parse(&fixture("flatpak-browser.desktop")).unwrap();
        assert_eq!(entry.name, "Brave");
        assert!(entry.is_browser());
        assert_eq!(entry.id, "flatpak-browser.desktop");
    }

    #[test]
    fn executable_strips_field_codes() {
        let entry = parse(&fixture("firefox.desktop")).unwrap();
        assert_eq!(entry.executable(), "/usr/bin/firefox");
    }

    #[test]
    fn nodisplay_browser_is_not_browser() {
        let entry = parse(&fixture("nodisplay-browser.desktop")).unwrap();
        assert!(entry.no_display);
        assert!(!entry.is_browser());
    }

    #[test]
    fn parse_nonexistent_file_returns_none() {
        assert!(parse(&PathBuf::from("/nonexistent.desktop")).is_none());
    }
}
