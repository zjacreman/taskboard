use std::path::{Path, PathBuf};

pub fn find_markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    find_markdown_files_recursive(root, &mut files);
    files
}

fn find_markdown_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden directories and common non-project dirs
        if path.is_dir() {
            if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
                continue;
            }
            find_markdown_files_recursive(&path, files);
        } else if path.is_file() {
            if name_str.ends_with(".md") || name_str.ends_with(".markdown") {
                files.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_vault() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create markdown files
        fs::write(dir.path().join("tasks.md"), "- [ ] Task 1\n- [x] Task 2").unwrap();
        fs::write(dir.path().join("notes.md"), "# Notes\nNot a task").unwrap();

        // Create subdirectory with tasks
        fs::create_dir(dir.path().join("projects")).unwrap();
        fs::write(dir.path().join("projects/todo.md"), "- [ ] Project task").unwrap();

        // Create files that should be skipped
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/config"), "git config").unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/pkg.js"), "code").unwrap();
        fs::write(dir.path().join("image.png"), "binary").unwrap();

        dir
    }

    #[test]
    fn test_find_markdown_files() {
        let dir = create_test_vault();
        let files = find_markdown_files(dir.path());

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.ends_with("tasks.md")));
        assert!(files.iter().any(|f| f.ends_with("notes.md")));
        assert!(files.iter().any(|f| f.ends_with("todo.md")));
    }

    #[test]
    fn test_skip_git_and_node_modules() {
        let dir = create_test_vault();
        let files = find_markdown_files(dir.path());

        assert!(!files.iter().any(|f| f.to_string_lossy().contains(".git")));
        assert!(!files.iter().any(|f| f.to_string_lossy().contains("node_modules")));
    }

    #[test]
    fn test_skip_non_markdown() {
        let dir = create_test_vault();
        let files = find_markdown_files(dir.path());

        assert!(!files.iter().any(|f| f.ends_with("image.png")));
        assert!(!files.iter().any(|f| f.ends_with("pkg.js")));
    }
}
