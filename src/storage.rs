use crate::view::View;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct ViewsFile {
    views: Vec<ViewSerde>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ViewSerde {
    name: String,
    query: String,
    #[serde(default)]
    sort_by: String,
    #[serde(default)]
    group_by: String,
}

pub fn load_views(path: &Path) -> Result<Vec<View>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(vec![View::default()]);
    }

    let content = std::fs::read_to_string(path)?;
    let file: ViewsFile = toml::from_str(&content)?;

    let views = file
        .views
        .into_iter()
        .map(|v| View {
            name: v.name,
            query: v.query,
            sort_by: v.sort_by,
            group_by: v.group_by,
        })
        .collect();

    Ok(views)
}

#[allow(dead_code)]
pub fn save_views(views: &[View], path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = ViewsFile {
        views: views
            .iter()
            .map(|v| ViewSerde {
                name: v.name.clone(),
                query: v.query.clone(),
                sort_by: v.sort_by.clone(),
                group_by: v.group_by.clone(),
            })
            .collect(),
    };

    let content = toml::to_string_pretty(&file)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_views() {
        let file = NamedTempFile::new().unwrap();
        let views = vec![
            View::new("View 1", "not done", "priority", ""),
            View::new("View 2", "tag work", "due", "folder"),
        ];

        save_views(&views, file.path()).unwrap();
        let loaded = load_views(file.path()).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "View 1");
        assert_eq!(loaded[1].name, "View 2");
    }

    #[test]
    fn test_load_missing_file() {
        let views = load_views(std::path::Path::new("/nonexistent/views.toml")).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "All Tasks"); // default view
    }
}
