use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub views: Vec<ViewConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_view")]
    pub view: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ThemeConfig {
    #[serde(default = "default_colors")]
    pub colors: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ViewConfig {
    pub name: String,
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default)]
    pub group_by: String,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            query: String::new(),
            sort_by: default_sort_by(),
            group_by: String::new(),
        }
    }
}

fn default_sort_by() -> String {
    "due_date".to_string()
}

fn default_view() -> String {
    "All Tasks".to_string()
}

fn default_colors() -> String {
    "dark".to_string()
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            view: default_view(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            colors: default_colors(),
        }
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if let Ok(config) = Config::from_file(Path::new("config.toml")) {
            return Ok(config);
        }

        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("taskboard").join("config.toml");
            if let Ok(config) = Config::from_file(&path) {
                return Ok(config);
            }
        }

        Err("No config file found".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_valid_config() {
        let toml = r#"
[workspace]
path = "/home/user/vault"

[defaults]
view = "All Tasks"

[theme]
colors = "dark"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.workspace.path, PathBuf::from("/home/user/vault"));
        assert_eq!(config.defaults.view, "All Tasks");
        assert_eq!(config.theme.colors, "dark");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[workspace]
path = "/tmp/vault"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.workspace.path, PathBuf::from("/tmp/vault"));
        assert_eq!(config.defaults.view, "All Tasks"); // default
        assert_eq!(config.theme.colors, "dark"); // default
    }

    #[test]
    fn test_config_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[workspace]\npath = \"/tmp/test\"").unwrap();

        let config = Config::from_file(file.path()).unwrap();
        assert_eq!(config.workspace.path, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_config_from_missing_file() {
        let result = Config::from_file(std::path::Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_config_with_views() {
        let toml = r#"
[workspace]
path = "/home/user/vault"

[defaults]
view = "Overdue"

[theme]
colors = "dark"

[[views]]
name = "All Tasks"
query = ""
sort_by = "due_date"
group_by = ""

[[views]]
name = "Overdue"
query = "due < today"
sort_by = "due_date"
group_by = ""
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.views.len(), 2);
        assert_eq!(config.views[0].name, "All Tasks");
        assert_eq!(config.views[1].name, "Overdue");
        assert_eq!(config.views[1].query, "due < today");
        assert_eq!(config.defaults.view, "Overdue");
    }

    #[test]
    fn test_parse_config_without_views() {
        let toml = r#"
[workspace]
path = "/tmp/vault"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.views.is_empty());
    }

    #[test]
    fn test_config_save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = Config {
            workspace: WorkspaceConfig { path: PathBuf::from("/tmp/vault") },
            defaults: DefaultsConfig { view: "My View".to_string() },
            theme: ThemeConfig::default(),
            views: vec![
                ViewConfig {
                    name: "All Tasks".to_string(),
                    query: String::new(),
                    sort_by: "due_date".to_string(),
                    group_by: String::new(),
                },
                ViewConfig {
                    name: "My View".to_string(),
                    query: "tag work".to_string(),
                    sort_by: "priority".to_string(),
                    group_by: String::new(),
                },
            ],
        };

        config.save(&path).unwrap();
        let loaded = Config::from_file(&path).unwrap();

        assert_eq!(loaded.defaults.view, "My View");
        assert_eq!(loaded.views.len(), 2);
        assert_eq!(loaded.views[1].name, "My View");
        assert_eq!(loaded.views[1].query, "tag work");
    }
}
