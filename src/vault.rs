use std::collections::HashSet;
use std::path::{Path, PathBuf};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc;
use std::time::Duration;

fn is_markdown_file(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("md") | Some("markdown"))
}

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            tx.send(res).ok();
        })?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    pub fn poll_changes(&self) -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        let mut changed_files = Vec::new();

        while let Ok(Ok(event)) = self.receiver.recv_timeout(Duration::from_millis(0)) {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        if is_markdown_file(&path) && seen.insert(path.clone()) {
                            changed_files.push(path);
                        }
                    }
                }
                _ => {}
            }
        }

        changed_files
    }
}

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

        if path.is_dir() {
            if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
                continue;
            }
            find_markdown_files_recursive(&path, files);
        } else if path.is_file() && is_markdown_file(&path) {
            files.push(path);
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

        fs::write(dir.path().join("tasks.md"), "- [ ] Task 1\n- [x] Task 2").unwrap();
        fs::write(dir.path().join("notes.md"), "# Notes\nNot a task").unwrap();

        fs::create_dir(dir.path().join("projects")).unwrap();
        fs::write(dir.path().join("projects/todo.md"), "- [ ] Project task").unwrap();

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

    #[test]
    fn test_file_watcher_creation() {
        let dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(dir.path());
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_detects_markdown_changes() {
        let dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(dir.path()).unwrap();

        let md_path = dir.path().join("test.md");
        std::fs::write(&md_path, "- [ ] New task").unwrap();

        // Brief sleep to let the filesystem watcher deliver events.
        // 50ms is usually sufficient; increase if tests flake on slow CI.
        std::thread::sleep(Duration::from_millis(50));

        let changed = watcher.poll_changes();
        assert!(changed.iter().any(|p| p.ends_with("test.md")));
    }

    #[test]
    fn test_file_watcher_ignores_non_markdown() {
        let dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(dir.path()).unwrap();

        let txt_path = dir.path().join("test.txt");
        std::fs::write(&txt_path, "not a markdown file").unwrap();

        // Brief sleep to let the filesystem watcher deliver events.
        std::thread::sleep(Duration::from_millis(50));

        let changed = watcher.poll_changes();
        assert!(changed.is_empty());
    }

    #[test]
    fn test_file_watcher_detects_deletion() {
        let dir = TempDir::new().unwrap();
        let md_path = dir.path().join("to_delete.md");
        std::fs::write(&md_path, "- [ ] doomed task").unwrap();

        let watcher = FileWatcher::new(dir.path()).unwrap();

        // Let watcher settle after creation
        std::thread::sleep(Duration::from_millis(50));
        let _ = watcher.poll_changes();

        // Delete the file
        std::fs::remove_file(&md_path).unwrap();

        // Give the watcher a moment to detect the deletion
        std::thread::sleep(Duration::from_millis(50));

        let changed = watcher.poll_changes();
        assert!(
            changed.iter().any(|p| p.ends_with("to_delete.md")),
            "Watcher should detect deleted markdown files, got: {:?}",
            changed
        );
    }
}
