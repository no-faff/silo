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
pub struct WindowSize {
    pub width: i32,
    pub height: i32,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picker_size: Option<WindowSize>,
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
            picker_size: None,
        }
    }
}

/// $XDG_CONFIG_HOME/silo/config.json
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("could not determine XDG_CONFIG_HOME")
        .join("silo")
        .join("config.json")
}

pub fn exists() -> bool {
    config_path().exists()
}

pub fn load() -> Config {
    load_from(&config_path())
}

/// Returns default config if the file is missing or corrupt.
pub fn load_from(path: &std::path::Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("silo: failed to parse config: {e}");
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

pub fn save(config: &Config) -> Result<(), std::io::Error> {
    save_to(config, &config_path())
}

/// Atomic write: writes to .tmp then renames.
pub fn save_to(config: &Config, path: &std::path::Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config)
        .map_err(std::io::Error::other)?;

    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
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
            rules: vec![Rule {
                domain: "github.com".to_string(),
                browser: BrowserRef {
                    desktop_file: "vivaldi-stable.desktop".to_string(),
                    args: Some("--profile-directory=Profile 1".to_string()),
                },
            }],
            picker_size: None,
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
            rules: vec![Rule {
                domain: "example.com".to_string(),
                browser: BrowserRef {
                    desktop_file: "firefox.desktop".to_string(),
                    args: None,
                },
            }],
            picker_size: None,
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

        save_to(&Config::default(), &path).unwrap();
        let config2 = Config {
            always_ask: true,
            ..Config::default()
        };
        save_to(&config2, &path).unwrap();

        let loaded = load_from(&path);
        assert!(loaded.always_ask);

        // no .tmp left behind
        assert!(!dir.path().join("config.json.tmp").exists());
    }
}
