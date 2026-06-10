use std::path::{Path, PathBuf};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc;
use std::time::Duration;

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
        let mut changed_files = Vec::new();

        while let Ok(Ok(event)) = self.receiver.recv_timeout(Duration::from_millis(0)) {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        if path.extension().is_some_and(|ext| ext == "md" || ext == "markdown") {
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

        // Skip hidden directories and common non-project dirs
        if path.is_dir() {
            if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
                continue;
            }
            find_markdown_files_recursive(&path, files);
        } else if path.is_file()
            && (name_str.ends_with(".md") || name_str.ends_with(".markdown"))
        {
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

        // Create a markdown file
        let md_path = dir.path().join("test.md");
        std::fs::write(&md_path, "- [ ] New task").unwrap();

        // Give the watcher a moment to detect the change
        std::thread::sleep(Duration::from_millis(100));

        let changed = watcher.poll_changes();
        assert!(changed.iter().any(|p| p.ends_with("test.md")));
    }

    #[test]
    fn test_file_watcher_ignores_non_markdown() {
        let dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(dir.path()).unwrap();

        // Create a non-markdown file
        let txt_path = dir.path().join("test.txt");
        std::fs::write(&txt_path, "not a markdown file").unwrap();

        // Give the watcher a moment
        std::thread::sleep(Duration::from_millis(100));

        let changed = watcher.poll_changes();
        assert!(changed.is_empty());
    }
}
