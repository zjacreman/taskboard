use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DefaultsConfig {
    #[serde(default = "default_view")]
    pub view: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ThemeConfig {
    #[serde(default = "default_colors")]
    pub colors: String,
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
        assert_eq!(config.workspace.path, "/home/user/vault");
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
        assert_eq!(config.workspace.path, "/tmp/vault");
        assert_eq!(config.defaults.view, "All Tasks"); // default
        assert_eq!(config.theme.colors, "dark"); // default
    }

    #[test]
    fn test_config_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[workspace]\npath = \"/tmp/test\"").unwrap();

        let config = Config::from_file(file.path()).unwrap();
        assert_eq!(config.workspace.path, "/tmp/test");
    }

    #[test]
    fn test_config_from_missing_file() {
        let result = Config::from_file(std::path::Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }
}
