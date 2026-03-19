# Silo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Silo, a native Linux browser picker that detects browsers and profiles, shows a picker popup and routes links via rules.

**Architecture:** Three-crate Cargo workspace. `silo-core` holds all pure logic (browser detection, config, rules, URL parsing). `silo-gui` holds the GTK4/libadwaita UI (picker, settings, first-run). A thin binary crate wires them together. Single-instance via `GtkApplication` D-Bus activation.

**Tech Stack:** Rust (edition 2024, requires rustup >= 1.85), GTK4 (gtk4-rs), libadwaita (libadwaita-rs), serde/serde_json for config, dirs for XDG paths. Blueprint and Meson are in the spec but deferred for v1; all UI is built programmatically in Rust, which is simpler for a first iteration. Blueprint/Meson can be added when the UI stabilises.

**Spec:** `docs/superpowers/specs/2026-03-19-silo-design.md`

---

## Prerequisites

Before starting, install the required system packages on Fedora:

```bash
sudo dnf install gtk4-devel libadwaita-devel gcc blueprint-compiler \
    glib2-devel pkgconf-pkg-config desktop-file-utils
```

Install Rust via rustup if not already present (minimum Rust 1.85 for
edition 2024):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## File Map

```
silo/
├── Cargo.toml                          # Workspace definition
├── .gitignore
├── CLAUDE.md                           # Project instructions for Claude
├── LICENSE                             # MIT
├── crates/
│   ├── silo-core/
│   │   ├── Cargo.toml                  # serde, serde_json, dirs, configparser
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports all modules
│   │       ├── url.rs                  # extract_domain()
│   │       ├── config.rs              # Config struct, load/save, atomic writes
│   │       ├── rule.rs                 # domain_matches(), find_matching_rule()
│   │       ├── desktop.rs             # .desktop file parsing
│   │       ├── browser.rs             # Browser discovery + profile detection
│   │       └── launcher.rs            # Launch browser process
│   ├── silo-gui/
│   │   ├── Cargo.toml                  # gtk4, libadwaita, silo-core
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports, app setup
│   │       ├── app.rs                  # adw::Application, signal handlers
│   │       ├── picker.rs              # Picker window
│   │       ├── settings.rs            # Settings/preferences window
│   │       └── first_run.rs           # Registration dialog
│   └── silo/
│       ├── Cargo.toml                  # silo-gui, clap
│       └── src/
│           └── main.rs                 # Entry point, arg parsing
├── tests/
│   └── integration.rs                  # Integration tests
├── data/
│   └── com.nofaff.Silo.desktop         # Desktop entry file
└── install.sh                          # Simple install script
```

---

## Task 1: Project scaffolding

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `CLAUDE.md`, `LICENSE`
- Create: `crates/silo-core/Cargo.toml`, `crates/silo-core/src/lib.rs`
- Create: `crates/silo-gui/Cargo.toml`, `crates/silo-gui/src/lib.rs`
- Create: `crates/silo/Cargo.toml`, `crates/silo/src/main.rs`

- [ ] **Step 1: Create the GitHub repo and clone it**

Create the repo at github.com/no-faff/silo (public, MIT licence, no README).
Clone it locally.

- [ ] **Step 2: Create the workspace Cargo.toml**

```toml
# Cargo.toml
[workspace]
members = [
    "crates/silo-core",
    "crates/silo-gui",
    "crates/silo",
]
resolver = "2"
```

- [ ] **Step 3: Create silo-core crate**

```toml
# crates/silo-core/Cargo.toml
[package]
name = "silo-core"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
configparser = "3"
url = "2"
shlex = "1"

[dev-dependencies]
tempfile = "3"
```

```rust
// crates/silo-core/src/lib.rs
pub mod url;
pub mod config;
pub mod rule;
pub mod desktop;
pub mod browser;
pub mod launcher;
```

- [ ] **Step 4: Create silo-gui crate**

```toml
# crates/silo-gui/Cargo.toml
[package]
name = "silo-gui"
version = "0.1.0"
edition = "2024"

[dependencies]
silo-core = { path = "../silo-core" }
gtk = { version = "0.11", package = "gtk4", features = ["v4_12"] }
adw = { version = "0.9", package = "libadwaita", features = ["v1_4"] }
```

```rust
// crates/silo-gui/src/lib.rs
pub mod app;
pub mod picker;
pub mod settings;
pub mod first_run;
```

- [ ] **Step 5: Create silo binary crate**

```toml
# crates/silo/Cargo.toml
[package]
name = "silo"
version = "0.1.0"
edition = "2024"

[dependencies]
silo-gui = { path = "../silo-gui" }
silo-core = { path = "../silo-core" }
gtk = { version = "0.11", package = "gtk4" }
glib = { version = "0.20", package = "glib" }
```

```rust
// crates/silo/src/main.rs
fn main() {
    println!("Silo starting");
}
```

- [ ] **Step 6: Create .gitignore, CLAUDE.md and LICENSE**

`.gitignore`:
```
/target
HANDOFF.md
/non-repo-files/
```

`LICENSE`: standard MIT licence text with "No Faff contributors" as copyright holder.

`CLAUDE.md`: see the spec for project conventions (British English, no Oxford
comma, sentence case, No Faff brand). Include:
- What the app does (one paragraph)
- Tech stack
- How to build: `cargo build`
- How to run: `cargo run --package silo`
- How to test: `cargo test --workspace`
- Required system packages (the dnf install line from Prerequisites)
- Link to the spec: `docs/superpowers/specs/2026-03-19-silo-design.md`

- [ ] **Step 7: Verify it compiles**

Run: `cargo build --workspace`
Expected: compiles with no errors.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: scaffold Cargo workspace with three crates"
```

---

## Task 2: URL parsing (silo-core)

**Files:**
- Create: `crates/silo-core/src/url.rs`
- Test: inline `#[cfg(test)]` module

- [ ] **Step 1: Write the failing tests**

```rust
// crates/silo-core/src/url.rs
/// Extract the domain (host) from a URL, lowercased.
pub fn extract_domain(url: &str) -> Option<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_https_url() {
        assert_eq!(
            extract_domain("https://github.com/no-faff/silo"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn http_url() {
        assert_eq!(
            extract_domain("http://example.com/page?q=1"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn url_with_port() {
        assert_eq!(
            extract_domain("https://localhost:8080/path"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn url_with_subdomain() {
        assert_eq!(
            extract_domain("https://mail.google.com"),
            Some("mail.google.com".to_string())
        );
    }

    #[test]
    fn uppercase_domain_lowercased() {
        assert_eq!(
            extract_domain("https://GitHub.COM/page"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn garbage_input() {
        assert_eq!(extract_domain("not a url"), None);
    }

    #[test]
    fn empty_input() {
        assert_eq!(extract_domain(""), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package silo-core -- url`
Expected: all tests fail with "not yet implemented"

- [ ] **Step 3: Implement extract_domain**

```rust
pub fn extract_domain(input: &str) -> Option<String> {
    let parsed = url::Url::parse(input).ok()?;
    let host = parsed.host_str()?;
    Some(host.to_lowercase())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package silo-core -- url`
Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/silo-core/src/url.rs
git commit -m "feat: add URL domain extraction"
```

---

## Task 3: Rule matching (silo-core)

**Files:**
- Create: `crates/silo-core/src/rule.rs`
- Test: inline `#[cfg(test)]` module

- [ ] **Step 1: Write the failing tests**

```rust
// crates/silo-core/src/rule.rs
/// Check if a domain pattern matches a given domain.
/// Patterns:
/// - "github.com" matches exactly "github.com"
/// - "*.google.com" matches "mail.google.com", "docs.google.com",
///   but NOT "google.com" itself
pub fn domain_matches(pattern: &str, domain: &str) -> bool {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(domain_matches("github.com", "github.com"));
    }

    #[test]
    fn exact_no_match() {
        assert!(!domain_matches("github.com", "gitlab.com"));
    }

    #[test]
    fn wildcard_matches_subdomain() {
        assert!(domain_matches("*.google.com", "mail.google.com"));
    }

    #[test]
    fn wildcard_matches_deep_subdomain() {
        assert!(domain_matches("*.google.com", "a.b.google.com"));
    }

    #[test]
    fn wildcard_does_not_match_bare_domain() {
        assert!(!domain_matches("*.google.com", "google.com"));
    }

    #[test]
    fn wildcard_does_not_match_unrelated() {
        assert!(!domain_matches("*.google.com", "evil-google.com"));
    }

    #[test]
    fn case_insensitive() {
        assert!(domain_matches("GitHub.com", "github.com"));
        assert!(domain_matches("github.com", "GitHub.COM"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package silo-core -- rule`
Expected: all tests fail with "not yet implemented"

- [ ] **Step 3: Implement domain_matches**

```rust
pub fn domain_matches(pattern: &str, domain: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let domain = domain.to_lowercase();

    if let Some(suffix) = pattern.strip_prefix("*.") {
        // Wildcard: domain must end with .suffix
        domain.ends_with(&format!(".{suffix}"))
    } else {
        pattern == domain
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package silo-core -- rule`
Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/silo-core/src/rule.rs
git commit -m "feat: add domain rule matching with wildcard support"
```

Note: `find_matching_rule` (which uses config types) is added in Task 4,
Step 6, after the `Rule` and `BrowserRef` types exist.

---

## Task 4: Config module (silo-core)

**Files:**
- Create: `crates/silo-core/src/config.rs`
- Test: inline `#[cfg(test)]` module

- [ ] **Step 1: Write the config types and failing tests**

```rust
// crates/silo-core/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrowserRef {
    pub desktop_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    pub domain: String,
    pub browser: BrowserRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub always_ask: bool,
    #[serde(default)]
    pub remember_choice: bool,
    #[serde(default)]
    pub setup_declined: bool,
    pub fallback_browser: Option<BrowserRef>,
    pub previous_default_browser: Option<String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            always_ask: false,
            remember_choice: true,
            setup_declined: false,
            fallback_browser: None,
            previous_default_browser: None,
            rules: Vec::new(),
        }
    }
}

/// Return the config file path: $XDG_CONFIG_HOME/silo/config.json
pub fn config_path() -> PathBuf {
    todo!()
}

/// Load config from disk. Returns default config if file does not exist.
pub fn load() -> Config {
    todo!()
}

/// Save config to disk with atomic write (write .tmp, rename).
pub fn save(config: &Config) -> Result<(), std::io::Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config_serialises_to_valid_json() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn config_with_rules_round_trips() {
        let config = Config {
            always_ask: true,
            remember_choice: false,
            setup_declined: false,
            fallback_browser: Some(BrowserRef {
                desktop_file: "firefox.desktop".to_string(),
                args: Some("-P default".to_string()),
            }),
            previous_default_browser: Some("chromium.desktop".to_string()),
            rules: vec![
                Rule {
                    domain: "github.com".to_string(),
                    browser: BrowserRef {
                        desktop_file: "vivaldi-stable.desktop".to_string(),
                        args: Some("--profile-directory=Profile 1".to_string()),
                    },
                },
            ],
        };
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn args_none_omitted_from_json() {
        let browser = BrowserRef {
            desktop_file: "firefox.desktop".to_string(),
            args: None,
        };
        let json = serde_json::to_string(&browser).unwrap();
        assert!(!json.contains("args"));
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        let config = Config {
            always_ask: true,
            remember_choice: false,
            setup_declined: false,
            fallback_browser: None,
            previous_default_browser: None,
            rules: vec![
                Rule {
                    domain: "example.com".to_string(),
                    browser: BrowserRef {
                        desktop_file: "firefox.desktop".to_string(),
                        args: None,
                    },
                },
            ],
        };

        save_to(&config, &path).unwrap();
        let loaded = load_from(&path);
        assert_eq!(config, loaded);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        let config = load_from(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("config.json");
        save_to(&Config::default(), &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_is_atomic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        // Save twice, second should overwrite cleanly
        save_to(&Config::default(), &path).unwrap();
        let config2 = Config { always_ask: true, ..Config::default() };
        save_to(&config2, &path).unwrap();

        let loaded = load_from(&path);
        assert!(loaded.always_ask);

        // No .tmp file left behind
        assert!(!dir.path().join("config.json.tmp").exists());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package silo-core -- config`
Expected: fails with "not yet implemented"

- [ ] **Step 3: Implement config_path, load and save**

```rust
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("could not determine XDG_CONFIG_HOME")
        .join("silo")
        .join("config.json")
}

/// Whether a config file exists on disk.
pub fn exists() -> bool {
    config_path().exists()
}

/// Load from the default config path.
pub fn load() -> Config {
    load_from(&config_path())
}

/// Load from a specific path. Returns default if file is missing.
pub fn load_from(path: &std::path::Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            serde_json::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("silo: failed to parse config: {e}");
                Config::default()
            })
        }
        Err(_) => Config::default(),
    }
}

/// Save to the default config path.
pub fn save(config: &Config) -> Result<(), std::io::Error> {
    save_to(config, &config_path())
}

/// Save to a specific path with atomic write.
pub fn save_to(config: &Config, path: &std::path::Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package silo-core -- config`
Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/silo-core/src/config.rs
git commit -m "feat: add config types with atomic save/load"
```

- [ ] **Step 6: Add find_matching_rule to rule.rs**

Now that `Rule` and `BrowserRef` types exist in config.rs, add the rule
lookup function to rule.rs:

```rust
use crate::config::Rule;

/// Find the first rule whose domain pattern matches the given domain.
pub fn find_matching_rule<'a>(rules: &'a [Rule], domain: &str) -> Option<&'a Rule> {
    rules.iter().find(|r| domain_matches(&r.domain, domain))
}
```

Add tests to the existing `tests` module in rule.rs:

```rust
use crate::config::{Rule, BrowserRef};

#[test]
fn find_rule_returns_first_match() {
    let rules = vec![
        Rule {
            domain: "github.com".to_string(),
            browser: BrowserRef {
                desktop_file: "firefox.desktop".to_string(),
                args: None,
            },
        },
        Rule {
            domain: "*.github.com".to_string(),
            browser: BrowserRef {
                desktop_file: "chrome.desktop".to_string(),
                args: None,
            },
        },
    ];
    let result = find_matching_rule(&rules, "github.com");
    assert_eq!(result.unwrap().browser.desktop_file, "firefox.desktop");
}

#[test]
fn find_rule_returns_none_when_no_match() {
    let rules = vec![
        Rule {
            domain: "github.com".to_string(),
            browser: BrowserRef {
                desktop_file: "firefox.desktop".to_string(),
                args: None,
            },
        },
    ];
    assert!(find_matching_rule(&rules, "gitlab.com").is_none());
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test --package silo-core -- rule`
Expected: all 9 tests pass (7 domain_matches + 2 find_matching_rule).

- [ ] **Step 8: Commit**

```bash
git add crates/silo-core/src/rule.rs
git commit -m "feat: add find_matching_rule using config types"
```

---

## Task 5: .desktop file parsing (silo-core)

**Files:**
- Create: `crates/silo-core/src/desktop.rs`
- Create: `crates/silo-core/tests/fixtures/` (test fixture .desktop files)
- Test: inline `#[cfg(test)]` module

- [ ] **Step 1: Create test fixture .desktop files**

Create `crates/silo-core/tests/fixtures/firefox.desktop`:
```ini
[Desktop Entry]
Name=Firefox Web Browser
Exec=/usr/bin/firefox %u
Icon=firefox
Type=Application
MimeType=x-scheme-handler/http;x-scheme-handler/https;text/html;
Categories=Network;WebBrowser;
```

Create `crates/silo-core/tests/fixtures/text-editor.desktop`:
```ini
[Desktop Entry]
Name=Text Editor
Exec=/usr/bin/gedit %f
Icon=text-editor
Type=Application
MimeType=text/plain;
Categories=Utility;TextEditor;
```

Create `crates/silo-core/tests/fixtures/flatpak-browser.desktop`:
```ini
[Desktop Entry]
Name=Brave Web Browser
Exec=/usr/bin/flatpak run --branch=stable com.brave.Browser %u
Icon=com.brave.Browser
Type=Application
MimeType=x-scheme-handler/http;x-scheme-handler/https;
```

- [ ] **Step 2: Write the types and failing tests**

```rust
// crates/silo-core/src/desktop.rs

/// A parsed .desktop file entry.
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    /// The .desktop filename (e.g. "firefox.desktop")
    pub id: String,
    /// Display name (e.g. "Firefox Web Browser")
    pub name: String,
    /// Exec command (e.g. "/usr/bin/firefox %u")
    pub exec: String,
    /// Icon name (e.g. "firefox")
    pub icon: String,
    /// MIME types this entry handles
    pub mime_types: Vec<String>,
}

impl DesktopEntry {
    /// Whether this entry handles HTTP URLs.
    pub fn is_browser(&self) -> bool {
        self.mime_types.iter().any(|m| m == "x-scheme-handler/http")
    }

    /// Extract the executable path from the Exec line,
    /// stripping field codes (%u, %f, etc.) and arguments.
    pub fn executable(&self) -> &str {
        todo!()
    }
}

/// Parse a .desktop file at the given path.
pub fn parse(path: &std::path::Path) -> Option<DesktopEntry> {
    todo!()
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
        assert_eq!(entry.name, "Firefox Web Browser");
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
        assert_eq!(entry.name, "Brave Web Browser");
        assert!(entry.is_browser());
        assert_eq!(entry.id, "flatpak-browser.desktop");
    }

    #[test]
    fn executable_strips_field_codes() {
        let entry = parse(&fixture("firefox.desktop")).unwrap();
        assert_eq!(entry.executable(), "/usr/bin/firefox");
    }

    #[test]
    fn parse_nonexistent_file_returns_none() {
        assert!(parse(&PathBuf::from("/nonexistent.desktop")).is_none());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --package silo-core -- desktop`
Expected: fails with "not yet implemented"

- [ ] **Step 4: Implement parse and executable**

```rust
pub fn parse(path: &std::path::Path) -> Option<DesktopEntry> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut mime_types = Vec::new();
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            // A different section has started
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
                // Take the first Name= (ignore locale variants like Name[fr]=)
                name = Some(val.to_string());
            }
        } else if let Some(val) = line.strip_prefix("Exec=") {
            exec = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("Icon=") {
            icon = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("MimeType=") {
            mime_types = val.split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    let id = path.file_name()?.to_str()?.to_string();

    Some(DesktopEntry {
        id,
        name: name?,
        exec: exec?,
        icon: icon.unwrap_or_default(),
        mime_types,
    })
}

impl DesktopEntry {
    pub fn executable(&self) -> &str {
        // The exec line may contain field codes like %u, %f, %U, %F
        // and may have multiple space-separated parts.
        // Return just the first part (the executable path).
        self.exec.split_whitespace().next().unwrap_or(&self.exec)
    }
}
```

Note: `Name[xx]=` locale lines contain a `[` after `Name` so they will not
match the `strip_prefix("Name=")` check. This is correct.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --package silo-core -- desktop`
Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/silo-core/src/desktop.rs crates/silo-core/tests/
git commit -m "feat: add .desktop file parser with test fixtures"
```

---

## Task 6: Browser detection (silo-core)

**Files:**
- Create: `crates/silo-core/src/browser.rs`
- Create: additional test fixtures for profile detection
- Test: inline `#[cfg(test)]` module

- [ ] **Step 1: Create profile detection test fixtures**

Create `crates/silo-core/tests/fixtures/chromium-local-state.json`:
```json
{
    "profile": {
        "info_cache": {
            "Default": {
                "name": "Personal",
                "shortcut_name": "Personal"
            },
            "Profile 1": {
                "name": "Work",
                "shortcut_name": "Work"
            }
        }
    }
}
```

Create `crates/silo-core/tests/fixtures/firefox-profiles.ini`:
```ini
[Profile0]
Name=default-release
IsRelative=1
Path=abcd1234.default-release
Default=1

[Profile1]
Name=Work
IsRelative=1
Path=efgh5678.Work

[General]
StartWithLastProfile=1
```

- [ ] **Step 2: Write the types and failing tests**

```rust
// crates/silo-core/src/browser.rs
use crate::desktop::DesktopEntry;

/// A browser or browser profile available on the system.
#[derive(Debug, Clone)]
pub struct BrowserEntry {
    /// The .desktop file ID (e.g. "firefox.desktop")
    pub desktop_file: String,
    /// Display name (e.g. "Firefox - Work")
    pub display_name: String,
    /// Icon name for GTK icon theme lookup
    pub icon: String,
    /// Additional args to pass when launching (e.g. "--profile-directory=Profile 1")
    pub profile_args: Option<String>,
    /// The Exec line from the .desktop file
    pub exec: String,
}

/// Known browser families for profile detection.
enum BrowserFamily {
    Chromium,
    Firefox,
    Unknown,
}

/// Detect the browser family from a .desktop file ID.
fn detect_family(desktop_id: &str) -> BrowserFamily {
    todo!()
}

/// Detect Chromium profiles from a Local State JSON file.
/// Returns (profile_directory, display_name) pairs.
pub fn detect_chromium_profiles(
    local_state_path: &std::path::Path,
) -> Vec<(String, String)> {
    todo!()
}

/// Detect Firefox profiles from a profiles.ini file.
/// Returns (profile_name_for_P_flag, display_name) pairs.
pub fn detect_firefox_profiles(
    profiles_ini_path: &std::path::Path,
) -> Vec<(String, String)> {
    todo!()
}

/// Discover all browsers on the system by scanning .desktop files.
/// Expands profiles for known browser families.
pub fn discover() -> Vec<BrowserEntry> {
    todo!()
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
        assert!(profiles.iter().any(|(dir, name)| dir == "Default" && name == "Personal"));
        assert!(profiles.iter().any(|(dir, name)| dir == "Profile 1" && name == "Work"));
    }

    #[test]
    fn detect_firefox_profiles_from_ini() {
        let profiles = detect_firefox_profiles(&fixture("firefox-profiles.ini"));
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().any(|(id, name)| id == "default-release" && name == "default-release"));
        assert!(profiles.iter().any(|(id, name)| id == "Work" && name == "Work"));
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
        assert!(matches!(detect_family("google-chrome.desktop"), BrowserFamily::Chromium));
    }

    #[test]
    fn detect_family_firefox() {
        assert!(matches!(detect_family("firefox.desktop"), BrowserFamily::Firefox));
    }

    #[test]
    fn detect_family_unknown() {
        assert!(matches!(detect_family("someapp.desktop"), BrowserFamily::Unknown));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --package silo-core -- browser`
Expected: fails with "not yet implemented"

- [ ] **Step 4: Implement detect_family**

```rust
fn detect_family(desktop_id: &str) -> BrowserFamily {
    let id = desktop_id.to_lowercase();

    let chromium_ids = [
        "google-chrome", "chromium", "brave", "vivaldi",
        "microsoft-edge", "opera", "com.brave.browser",
        "com.google.chrome", "org.chromium.chromium",
    ];
    let firefox_ids = [
        "firefox", "librewolf", "waterfox", "floorp",
        "zen-browser", "mullvad-browser",
        "org.mozilla.firefox", "io.gitlab.librewolf-community",
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
```

- [ ] **Step 5: Implement detect_chromium_profiles**

```rust
pub fn detect_chromium_profiles(
    local_state_path: &std::path::Path,
) -> Vec<(String, String)> {
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

    info_cache.iter().filter_map(|(dir, info)| {
        let name = info.get("name")?.as_str()?;
        Some((dir.clone(), name.to_string()))
    }).collect()
}
```

- [ ] **Step 6: Implement detect_firefox_profiles**

```rust
pub fn detect_firefox_profiles(
    profiles_ini_path: &std::path::Path,
) -> Vec<(String, String)> {
    let mut parser = configparser::ini::Ini::new();
    if parser.load(profiles_ini_path).is_err() {
        return Vec::new();
    }

    let mut profiles = Vec::new();

    for section in parser.sections() {
        // Profile sections are named "Profile0", "Profile1", etc.
        if !section.to_lowercase().starts_with("profile") {
            continue;
        }
        // Skip if it parses as just "profile" without a number
        if section.len() <= "profile".len() {
            continue;
        }

        if let Some(name) = parser.get(&section, "name") {
            profiles.push((name.clone(), name));
        }
    }

    profiles
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --package silo-core -- browser`
Expected: all 7 tests pass.

- [ ] **Step 8: Implement discover (browser scanning)**

This function scans the system `.desktop` file directories. Since it reads
the actual filesystem, the full integration test happens in Task 12. For
now, implement the function and verify it compiles.

```rust
use crate::desktop;

/// Directories to scan for .desktop files.
fn desktop_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    // XDG_DATA_DIRS (system-wide)
    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_dirs.split(':') {
            dirs.push(std::path::PathBuf::from(dir).join("applications"));
        }
    } else {
        dirs.push("/usr/share/applications".into());
        dirs.push("/usr/local/share/applications".into());
    }

    // XDG_DATA_HOME (user)
    if let Some(data_home) = dirs::data_dir() {
        dirs.push(data_home.join("applications"));
    }

    // Flatpak
    dirs.push("/var/lib/flatpak/exports/share/applications".into());
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    // Snap
    dirs.push("/var/lib/snapd/desktop/applications".into());

    dirs
}

/// Known config directory names for Chromium-family browsers.
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

/// Known config directory names for Firefox-family browsers.
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

fn expand_entry(de: &DesktopEntry, entries: &mut Vec<BrowserEntry>) {
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

fn single_entry(de: &DesktopEntry) -> BrowserEntry {
    BrowserEntry {
        desktop_file: de.id.clone(),
        display_name: de.name.clone(),
        icon: de.icon.clone(),
        profile_args: None,
        exec: de.exec.clone(),
    }
}
```

- [ ] **Step 9: Verify it compiles**

Run: `cargo build --package silo-core`
Expected: compiles with no errors.

- [ ] **Step 10: Commit**

```bash
git add crates/silo-core/src/browser.rs crates/silo-core/tests/fixtures/
git commit -m "feat: add browser detection with Chromium and Firefox profile support"
```

---

## Task 7: Browser launching (silo-core)

**Files:**
- Create: `crates/silo-core/src/launcher.rs`

- [ ] **Step 1: Implement the launcher**

```rust
// crates/silo-core/src/launcher.rs
use crate::browser::BrowserEntry;
use std::process::Command;

/// Launch a browser entry with the given URL.
/// Returns Ok(()) if the process was spawned, Err with a message if not.
pub fn launch(entry: &BrowserEntry, url: &str) -> Result<(), String> {
    // Parse the Exec line: take the first token as executable,
    // replace %u/%U with the URL, drop other field codes (%f, %F, etc.)
    let parts: Vec<&str> = entry.exec.split_whitespace().collect();
    let executable = parts.first().ok_or("empty Exec line")?;

    let mut cmd = Command::new(executable);

    // Add remaining Exec args, substituting field codes
    for part in &parts[1..] {
        match *part {
            "%u" | "%U" => cmd.arg(url),
            "%f" | "%F" | "%i" | "%c" | "%k" => continue,
            other => cmd.arg(other),
        };
    }

    // Add profile args if present (shell-style split to handle quoted
    // profile names like -P 'My Work')
    if let Some(ref args) = entry.profile_args {
        if let Some(parsed) = shlex::split(args) {
            for arg in parsed {
                cmd.arg(arg);
            }
        }
    }

    // Add URL if no %u/%U was in the Exec line
    if !entry.exec.contains("%u") && !entry.exec.contains("%U") {
        cmd.arg(url);
    }

    // Detach from parent process group
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    cmd.spawn().map_err(|e| format!("failed to launch {}: {e}", executable))?;

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --package silo-core`
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/silo-core/src/launcher.rs
git commit -m "feat: add browser launcher with process detaching"
```

---

## Task 8: GTK application shell (silo-gui)

**Files:**
- Create: `crates/silo-gui/src/app.rs`
- Modify: `crates/silo-gui/src/lib.rs`
- Modify: `crates/silo/src/main.rs`

- [ ] **Step 1: Implement the app shell**

```rust
// crates/silo-gui/src/app.rs
use gtk::prelude::*;
use gtk::gio;
use adw::prelude::*;

pub const APP_ID: &str = "com.nofaff.Silo";

/// Run the application. Uses HANDLES_COMMAND_LINE instead of HANDLES_OPEN
/// because GtkApplication's HANDLES_OPEN can mangle URLs into gio::File
/// paths. With HANDLES_COMMAND_LINE we get the raw string arguments.
pub fn run() -> glib::ExitCode {
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
) -> i32 {
    let args = cmd.arguments();
    // First arg is the executable name, rest are URLs or flags
    let args: Vec<String> = args.iter()
        .skip(1)
        .filter_map(|a| a.to_str().map(|s| s.to_string()))
        .collect();

    // Check for --settings flag
    if args.iter().any(|a| a == "--settings") {
        show_settings_or_first_run(app, None);
        return 0;
    }

    // Find the first argument that looks like a URL
    let url = args.iter().find(|a| {
        a.starts_with("http://") || a.starts_with("https://") || a.contains("://")
    });

    match url {
        Some(url) => handle_url(app, url),
        None => show_settings_or_first_run(app, None),
    }

    0
}

fn show_settings_or_first_run(app: &adw::Application, then_url: Option<String>) {
    if !silo_core::config::exists() {
        // First run - show registration dialog
        // TODO: Task 10 (first_run)
        eprintln!("silo: first run - registration dialog not yet implemented");
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Silo")
            .default_width(500)
            .default_height(400)
            .build();
        window.present();
        return;
    }

    // Show settings
    // TODO: Task 11
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Silo Settings")
        .default_width(500)
        .default_height(400)
        .build();
    window.present();
}

fn handle_url(app: &adw::Application, url: &str) {
    // First-run check
    if !silo_core::config::exists() {
        // TODO: Task 10 (first_run with URL)
        eprintln!("silo: first run with URL - not yet implemented");
        return;
    }

    let config = silo_core::config::load();
    let domain = silo_core::url::extract_domain(url);
    let browsers = silo_core::browser::discover();

    // Single browser: open silently
    if browsers.len() == 1 {
        if let Err(e) = silo_core::launcher::launch(&browsers[0], url) {
            eprintln!("silo: {e}");
        }
        return;
    }

    // Check rules
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

    // Fallback browser (if not "always ask")
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

    // Show picker
    // TODO: Task 9 (picker window)
    eprintln!("silo: would show picker for {url}");
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Silo")
        .build();
    window.present();
}
```

- [ ] **Step 2: Update main.rs to use the app shell**

```rust
// crates/silo/src/main.rs
fn main() -> glib::ExitCode {
    silo_gui::app::run()
}
```

- [ ] **Step 3: Update silo-gui lib.rs**

```rust
// crates/silo-gui/src/lib.rs
pub mod app;
// pub mod picker;     // Task 9
// pub mod settings;   // Task 11
// pub mod first_run;  // Task 10
```

- [ ] **Step 4: Verify it compiles and launches**

Run: `cargo build --package silo`
Expected: compiles with no errors.

Run: `cargo run --package silo`
Expected: a blank window appears with the title "Silo". Close it.

Run: `cargo run --package silo -- "https://example.com"`
Expected: prints "silo: would show picker for https://example.com" to stderr
and shows a blank window (or silently opens the browser if rules/fallback
match). This confirms the `open` signal handler is wired up.

- [ ] **Step 5: Commit**

```bash
git add crates/silo-gui/src/app.rs crates/silo-gui/src/lib.rs crates/silo/src/main.rs
git commit -m "feat: add GTK application shell with single-instance D-Bus activation"
```

---

## Task 9: Picker window (silo-gui)

**Files:**
- Create: `crates/silo-gui/src/picker.rs`
- Modify: `crates/silo-gui/src/app.rs` (wire up picker)

- [ ] **Step 1: Implement the picker window**

```rust
// crates/silo-gui/src/picker.rs
use gtk::prelude::*;
use adw::prelude::*;
use silo_core::browser::BrowserEntry;
use silo_core::config::{self, BrowserRef, Config, Rule};

/// Show the picker window for a given URL.
pub fn show(
    app: &adw::Application,
    url: &str,
    domain: Option<&str>,
    browsers: &[BrowserEntry],
    config: &Config,
) {
    let url = url.to_string();
    let domain_str = domain.unwrap_or("").to_string();

    // Header bar
    let header = adw::HeaderBar::builder()
        .show_title(false)
        .build();

    // Domain label in header: "Open github.com in"
    let header_label = gtk::Label::builder()
        .label(if domain_str.is_empty() {
            "Open in".to_string()
        } else {
            format!("Open {} in", domain_str)
        })
        .css_classes(["title"])
        .build();

    // Remember checkbox in header area
    let remember_check = gtk::CheckButton::builder()
        .label(if domain_str.is_empty() {
            "Always use for this link".to_string()
        } else {
            format!("Always use for {}", domain_str)
        })
        .active(config.remember_choice)
        .build();

    let header_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    header_box.append(&header_label);
    header_box.append(&remember_check);

    // Browser list
    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    for (i, entry) in browsers.iter().enumerate() {
        let shortcut = match i {
            0..=8 => format!("{}", i + 1),
            9 => "0".to_string(),
            _ => String::new(),
        };

        let row = adw::ActionRow::builder()
            .title(&entry.display_name)
            .activatable(true)
            .build();

        // Browser icon
        let icon = gtk::Image::from_icon_name(&entry.icon);
        icon.set_pixel_size(32);
        row.add_prefix(&icon);

        // Keyboard shortcut label
        if !shortcut.is_empty() {
            let shortcut_label = gtk::Label::builder()
                .label(&shortcut)
                .css_classes(["dim-label"])
                .build();
            row.add_suffix(&shortcut_label);
        }

        list_box.append(&row);
    }

    let scrolled = gtk::ScrolledWindow::builder()
        .child(&list_box)
        .vexpand(true)
        .build();

    // Main layout
    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    content_box.append(&header_box);
    content_box.append(&scrolled);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content_box));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .content(&toolbar)
        .default_width(400)
        .default_height(500)
        .build();

    // Clone data for closures
    let browsers_clone = browsers.to_vec();
    let url_clone = url.clone();
    let domain_clone = domain_str.clone();
    let remember_ref = remember_check.clone();
    let window_ref = window.clone();

    // Row activation: launch browser
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(entry) = browsers_clone.get(index) {
            // Save "remember" state to config
            let mut config = config::load();
            config.remember_choice = remember_ref.is_active();

            // If remember is ticked and we have a domain, save a rule
            if remember_ref.is_active() && !domain_clone.is_empty() {
                let new_rule = Rule {
                    domain: domain_clone.clone(),
                    browser: BrowserRef {
                        desktop_file: entry.desktop_file.clone(),
                        args: entry.profile_args.clone(),
                    },
                };
                // Don't duplicate existing rules for the same domain
                config.rules.retain(|r| r.domain != domain_clone);
                config.rules.push(new_rule);
            }

            if let Err(e) = config::save(&config) {
                eprintln!("silo: failed to save config: {e}");
            }

            if let Err(e) = silo_core::launcher::launch(entry, &url_clone) {
                eprintln!("silo: {e}");
                // TODO: show error dialog
            }

            window_ref.close();
        }
    });

    // Keyboard shortcuts (1-9, 0)
    let key_controller = gtk::EventControllerKey::new();
    let browsers_for_keys = browsers.to_vec();
    let url_for_keys = url.clone();
    let domain_for_keys = domain_str.clone();
    let remember_for_keys = remember_check.clone();
    let window_for_keys = window.clone();

    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        // Escape closes
        if keyval == gtk::gdk::Key::Escape {
            window_for_keys.close();
            return gtk::glib::Propagation::Stop;
        }

        // Number keys
        let index = match keyval {
            gtk::gdk::Key::_1 => Some(0),
            gtk::gdk::Key::_2 => Some(1),
            gtk::gdk::Key::_3 => Some(2),
            gtk::gdk::Key::_4 => Some(3),
            gtk::gdk::Key::_5 => Some(4),
            gtk::gdk::Key::_6 => Some(5),
            gtk::gdk::Key::_7 => Some(6),
            gtk::gdk::Key::_8 => Some(7),
            gtk::gdk::Key::_9 => Some(8),
            gtk::gdk::Key::_0 => Some(9),
            _ => None,
        };

        if let Some(i) = index {
            if let Some(entry) = browsers_for_keys.get(i) {
                let mut config = config::load();
                config.remember_choice = remember_for_keys.is_active();

                if remember_for_keys.is_active() && !domain_for_keys.is_empty() {
                    let new_rule = Rule {
                        domain: domain_for_keys.clone(),
                        browser: BrowserRef {
                            desktop_file: entry.desktop_file.clone(),
                            args: entry.profile_args.clone(),
                        },
                    };
                    config.rules.retain(|r| r.domain != domain_for_keys);
                    config.rules.push(new_rule);
                }

                if let Err(e) = config::save(&config) {
                    eprintln!("silo: failed to save config: {e}");
                }

                if let Err(e) = silo_core::launcher::launch(entry, &url_for_keys) {
                    eprintln!("silo: {e}");
                }

                window_for_keys.close();
                return gtk::glib::Propagation::Stop;
            }
        }

        gtk::glib::Propagation::Proceed
    });

    window.add_controller(key_controller);

    // Close on focus loss
    let window_for_focus = window.clone();
    window.connect_is_active_notify(move |win| {
        if !win.is_active() {
            window_for_focus.close();
        }
    });

    window.present();
}
```

Note: `BrowserEntry` needs to derive `Clone`. Go back to browser.rs and
ensure it has `#[derive(Debug, Clone)]`.

- [ ] **Step 2: Wire the picker into app.rs**

Replace the `// TODO: Task 9 (picker window)` block in `on_open` with:

```rust
    // Show picker
    crate::picker::show(app, &url, domain.as_deref(), &browsers, &config);
```

Uncomment `pub mod picker;` in lib.rs.

- [ ] **Step 3: Verify it compiles and the picker appears**

Run: `cargo run --package silo -- "https://github.com"`
Expected: a picker window appears showing "Open github.com in" with a list
of detected browsers. Pressing a number or clicking a row launches the
browser and closes the picker. Pressing Escape closes the picker.

- [ ] **Step 4: Commit**

```bash
git add crates/silo-gui/src/picker.rs crates/silo-gui/src/app.rs crates/silo-gui/src/lib.rs
git commit -m "feat: add picker window with browser list and keyboard shortcuts"
```

---

## Task 10: First-run dialog (silo-gui)

**Files:**
- Create: `crates/silo-gui/src/first_run.rs`
- Modify: `crates/silo-gui/src/app.rs` (wire up first-run)

- [ ] **Step 1: Implement the first-run dialog**

```rust
// crates/silo-gui/src/first_run.rs
use gtk::prelude::*;
use adw::prelude::*;
use silo_core::config;
use std::process::Command;

/// Show the first-run registration dialog.
/// Returns after the user responds. If a URL was provided, the caller
/// should proceed to the picker afterwards.
pub fn show(app: &adw::Application, then_open_url: Option<String>) {
    // Detect current default browser name for the message
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
        // TODO: during development, verify whether uninstalling Silo
        // actually restores the previous default. If it does, use the
        // first message. If not, use a simpler message without the
        // restore promise.
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

    // We need a parent window for the dialog
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
            // Store previous default
            if !current_default_clone.is_empty() {
                new_config.previous_default_browser = Some(current_default_clone.clone());
            }

            // Register as default
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

        // If a URL was provided, open it now
        if let Some(ref url) = then_open_url {
            let browsers = silo_core::browser::discover();
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
```

- [ ] **Step 2: Wire first-run into app.rs**

Update `show_settings_or_first_run` in app.rs to use the first-run dialog:

```rust
fn show_settings_or_first_run(app: &adw::Application, then_url: Option<String>) {
    if !silo_core::config::exists() {
        crate::first_run::show(app, then_url);
        return;
    }

    let config = silo_core::config::load();
    if config.setup_declined && then_url.is_none() {
        // Settings with registration reminder
    }

    // Show settings
    // TODO: Task 11
    let browsers = silo_core::browser::discover();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Silo Settings")
        .default_width(500)
        .default_height(400)
        .build();
    window.present();
}
```

Update `handle_url` similarly:

```rust
fn handle_url(app: &adw::Application, url: &str) {
    if !silo_core::config::exists() {
        crate::first_run::show(app, Some(url.to_string()));
        return;
    }
    // ... rest of the existing flow (rules, fallback, picker)
```

Uncomment `pub mod first_run;` in lib.rs.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build --package silo`
Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/silo-gui/src/first_run.rs crates/silo-gui/src/app.rs crates/silo-gui/src/lib.rs
git commit -m "feat: add first-run registration dialog"
```

---

## Task 11: Settings window (silo-gui)

**Files:**
- Create: `crates/silo-gui/src/settings.rs`
- Modify: `crates/silo-gui/src/app.rs` (wire up settings)

- [ ] **Step 1: Implement the settings window**

```rust
// crates/silo-gui/src/settings.rs
use gtk::prelude::*;
use adw::prelude::*;
use silo_core::config::{self, Config};
use silo_core::browser::BrowserEntry;

pub fn show(app: &adw::Application, config: &Config, browsers: &[BrowserEntry]) {
    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("Silo")
        .default_width(550)
        .default_height(500)
        .build();

    // === Behaviour page ===
    let behaviour_page = adw::PreferencesPage::builder()
        .title("Behaviour")
        .icon_name("preferences-system-symbolic")
        .build();

    // Always ask toggle
    let always_ask_row = adw::SwitchRow::builder()
        .title("Always ask")
        .subtitle("Show picker for every link without a rule")
        .active(config.always_ask)
        .build();

    // Fallback browser
    let fallback_row = adw::ComboRow::builder()
        .title("Fallback browser")
        .subtitle("Used when no rule matches and 'Always ask' is off")
        .build();

    let browser_names: Vec<String> = std::iter::once("None".to_string())
        .chain(browsers.iter().map(|b| b.display_name.clone()))
        .collect();
    let model = gtk::StringList::new(
        &browser_names.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    );
    fallback_row.set_model(Some(&model));

    // Select current fallback
    if let Some(ref fb) = config.fallback_browser {
        if let Some(pos) = browsers.iter().position(|b| {
            b.desktop_file == fb.desktop_file
                && b.profile_args.as_deref() == fb.args.as_deref()
        }) {
            fallback_row.set_selected(pos as u32 + 1); // +1 for "None"
        }
    }

    let behaviour_group = adw::PreferencesGroup::new();
    behaviour_group.add(&always_ask_row);
    behaviour_group.add(&fallback_row);
    behaviour_page.add(&behaviour_group);

    // === Rules page ===
    let rules_page = adw::PreferencesPage::builder()
        .title("Rules")
        .icon_name("view-list-symbolic")
        .build();

    let rules_group = adw::PreferencesGroup::builder()
        .title("Domain rules")
        .description("Links matching these domains open silently in the assigned browser")
        .build();

    for rule in &config.rules {
        let browser_name = browsers.iter()
            .find(|b| {
                b.desktop_file == rule.browser.desktop_file
                    && b.profile_args.as_deref() == rule.browser.args.as_deref()
            })
            .map(|b| b.display_name.as_str())
            .unwrap_or(&rule.browser.desktop_file);

        let row = adw::ActionRow::builder()
            .title(&rule.domain)
            .subtitle(browser_name)
            .build();

        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let domain_for_delete = rule.domain.clone();
        let window_ref = window.clone();
        delete_btn.connect_clicked(move |_| {
            let mut config = config::load();
            config.rules.retain(|r| r.domain != domain_for_delete);
            if let Err(e) = config::save(&config) {
                eprintln!("silo: failed to save config: {e}");
            }
            // Close and reopen to refresh
            // (A proper implementation would use a model, but this
            // is simple and correct for v1)
            window_ref.close();
        });

        row.add_suffix(&delete_btn);
        rules_group.add(&row);
    }

    if config.rules.is_empty() {
        let empty_row = adw::ActionRow::builder()
            .title("No rules yet")
            .subtitle("Use 'Always use for [domain]' in the picker to create rules")
            .build();
        rules_group.add(&empty_row);
    }

    rules_page.add(&rules_group);

    // === Registration status ===
    if config.setup_declined {
        let status_group = adw::PreferencesGroup::new();
        let register_row = adw::ActionRow::builder()
            .title("Silo is not your default browser")
            .subtitle("Links from other apps will not be routed through Silo")
            .build();

        let register_btn = gtk::Button::builder()
            .label("Set as default")
            .valign(gtk::Align::Center)
            .css_classes(["suggested-action"])
            .build();

        let window_for_reg = window.clone();
        register_btn.connect_clicked(move |_| {
            let result = std::process::Command::new("xdg-settings")
                .args(["set", "default-web-browser", "com.nofaff.Silo.desktop"])
                .status();

            match result {
                Ok(status) if status.success() => {
                    let mut config = config::load();
                    config.setup_declined = false;
                    let _ = config::save(&config);
                    window_for_reg.close();
                }
                _ => {
                    eprintln!("silo: failed to register as default browser");
                }
            }
        });

        register_row.add_suffix(&register_btn);
        status_group.add(&register_row);
        behaviour_page.add(&status_group);
    }

    window.add(&behaviour_page);
    window.add(&rules_page);

    // Save on close
    let browsers_clone = browsers.to_vec();
    window.connect_close_request(move |_| {
        let mut config = config::load();
        config.always_ask = always_ask_row.is_active();

        let selected = fallback_row.selected();
        config.fallback_browser = if selected == 0 {
            None
        } else {
            browsers_clone.get(selected as usize - 1).map(|b| {
                silo_core::config::BrowserRef {
                    desktop_file: b.desktop_file.clone(),
                    args: b.profile_args.clone(),
                }
            })
        };

        if let Err(e) = config::save(&config) {
            eprintln!("silo: failed to save config: {e}");
        }

        gtk::glib::Propagation::Proceed
    });

    window.present();
}
```

- [ ] **Step 2: Wire settings into app.rs**

Update `show_settings_or_first_run` in app.rs to use the settings window:

```rust
fn show_settings_or_first_run(app: &adw::Application, then_url: Option<String>) {
    if !silo_core::config::exists() {
        crate::first_run::show(app, then_url);
        return;
    }

    let config = silo_core::config::load();
    let browsers = silo_core::browser::discover();
    crate::settings::show(app, &config, &browsers);
}
```

Uncomment `pub mod settings;` in lib.rs.

- [ ] **Step 3: Verify it compiles and settings appear**

Run: `cargo run --package silo`
Expected: settings window appears with Behaviour and Rules tabs. The
"Always ask" toggle and fallback browser dropdown should be visible.

- [ ] **Step 4: Commit**

```bash
git add crates/silo-gui/src/settings.rs crates/silo-gui/src/app.rs crates/silo-gui/src/lib.rs
git commit -m "feat: add settings window with rules, fallback and always-ask toggle"
```

---

## Task 12: Error dialogs and edge cases (silo-gui)

**Files:**
- Modify: `crates/silo-gui/src/app.rs`

- [ ] **Step 1: Add error dialog for malformed URLs**

In `on_open`, after the empty URL check, add URL validation:

```rust
    // Validate URL
    let domain = silo_core::url::extract_domain(&url);
    if domain.is_none() {
        // Malformed URL - show error
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Silo")
            .default_width(1)
            .default_height(1)
            .build();
        window.present();

        let dialog = adw::AlertDialog::builder()
            .heading("Cannot open link")
            .body(&format!("Received a malformed URL:\n\n{}", url))
            .build();
        dialog.add_responses(&[("close", "Dismiss")]);
        let window_ref = window.clone();
        dialog.connect_response(None, move |_, _| {
            window_ref.close();
        });
        dialog.present(Some(&window));
        return;
    }
```

- [ ] **Step 2: Add error dialog for zero browsers**

After `let browsers = silo_core::browser::discover();`, add:

```rust
    if browsers.is_empty() {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Silo")
            .default_width(1)
            .default_height(1)
            .build();
        window.present();

        let dialog = adw::AlertDialog::builder()
            .heading("No browsers found")
            .body("Install a browser and try again.")
            .build();
        dialog.add_responses(&[("close", "Dismiss")]);
        let window_ref = window.clone();
        dialog.connect_response(None, move |_, _| {
            window_ref.close();
        });
        dialog.present(Some(&window));
        return;
    }
```

- [ ] **Step 3: Add error dialog for browser launch failure in picker**

Update the `list_box.connect_row_activated` and key handler closures in
picker.rs to show an error dialog on launch failure instead of just logging:

```rust
if let Err(e) = silo_core::launcher::launch(entry, &url_clone) {
    let dialog = adw::AlertDialog::builder()
        .heading("Failed to open browser")
        .body(&e)
        .build();
    dialog.add_responses(&[("close", "Dismiss")]);
    dialog.present(Some(&window_ref));
    return; // Don't close the picker, let user try another browser
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build --package silo`
Expected: compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/silo-gui/src/app.rs crates/silo-gui/src/picker.rs
git commit -m "feat: add error dialogs for malformed URLs, missing browsers and launch failures"
```

---

## Task 13: .desktop file and install script

**Files:**
- Create: `data/com.nofaff.Silo.desktop`
- Create: `install.sh`

- [ ] **Step 1: Create the .desktop file**

```ini
[Desktop Entry]
Name=Silo
Comment=Browser picker with profile support
Exec=silo %u
Icon=com.nofaff.Silo
Type=Application
Categories=Network;Utility;
MimeType=x-scheme-handler/http;x-scheme-handler/https;
StartupNotify=false
```

- [ ] **Step 2: Create the install script**

```bash
#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="silo"
DESKTOP_FILE="com.nofaff.Silo.desktop"
INSTALL_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${HOME}/.local/share/applications"

echo "Installing Silo..."

# Copy binary
mkdir -p "${INSTALL_DIR}"
cp "${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# Install .desktop file, substituting the correct binary path
mkdir -p "${DESKTOP_DIR}"
sed "s|Exec=silo|Exec=${INSTALL_DIR}/${BINARY_NAME}|" \
    "data/${DESKTOP_FILE}" > "${DESKTOP_DIR}/${DESKTOP_FILE}"

# Update desktop database
update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true

echo "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
echo ""
echo "To set Silo as your default browser, run:"
echo "  silo"
echo ""
echo "Or set it manually:"
echo "  xdg-settings set default-web-browser ${DESKTOP_FILE}"
```

- [ ] **Step 3: Verify the .desktop file is valid**

Run: `desktop-file-validate data/com.nofaff.Silo.desktop`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add data/com.nofaff.Silo.desktop install.sh
git commit -m "feat: add .desktop file and install script"
```

---

## Task 14: Integration testing and polish

**Files:**
- Modify: various files for polish
- Manual testing

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --workspace`
Expected: all unit tests pass.

- [ ] **Step 2: Build a release binary**

Run: `cargo build --package silo --release`
Expected: binary at `target/release/silo`. Check its size.

- [ ] **Step 3: Manual integration test - picker flow**

Run: `./target/release/silo "https://github.com"`
Expected: picker appears with detected browsers. Click one. Browser opens
at the URL. Picker closes.

- [ ] **Step 4: Manual integration test - remember choice**

1. Run: `./target/release/silo "https://example.com"`
2. Tick "Always use for example.com", pick a browser.
3. Run: `./target/release/silo "https://example.com"` again.
Expected: browser opens silently, no picker shown.

- [ ] **Step 5: Manual integration test - settings**

Run: `./target/release/silo` (no URL)
Expected: settings window appears. Rules tab shows the example.com rule
created in step 4. Delete it. Toggle "Always ask". Close. Verify config
file at `~/.config/silo/config.json` reflects changes.

- [ ] **Step 6: Manual integration test - keyboard shortcuts**

Run: `./target/release/silo "https://test.com"`
Expected: press number keys 1-9/0 to select browsers. Press Escape to
close. Click outside to close.

- [ ] **Step 7: Manual integration test - first run**

Delete config: `rm ~/.config/silo/config.json`
Run: `./target/release/silo`
Expected: first-run dialog appears asking to set as default browser.

- [ ] **Step 8: Commit any fixes from testing**

```bash
git add -A
git commit -m "fix: integration testing polish"
```

---

## Done

At this point you have a working Silo v0.1.0:
- Browser and profile detection
- Picker window with keyboard shortcuts
- Rule creation via "remember my choice"
- Settings window with rules, fallback and always-ask
- First-run registration dialog
- Error handling for edge cases
- .desktop file and install script

Next steps (not in this plan):
- Icon design
- README
- GitHub release with `.tar.gz`
- Fedora COPR RPM
