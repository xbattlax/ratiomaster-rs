/// File browser for selecting .torrent files in the TUI.
use std::path::PathBuf;

/// A single entry in the file browser.
#[derive(Debug, Clone)]
pub struct BrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// File browser state.
pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<BrowserEntry>,
    pub selected: usize,
    pub scroll: usize,
}

impl FileBrowser {
    /// Creates a new file browser starting at the current directory.
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut browser = Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
        };
        browser.refresh();
        browser
    }

    /// Refreshes the file listing.
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected = 0;
        self.scroll = 0;

        // Add parent directory entry
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(BrowserEntry {
                name: "..".into(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }

        // Read directory contents
        let entries = match std::fs::read_dir(&self.current_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                dirs.push(BrowserEntry {
                    name,
                    path,
                    is_dir: true,
                });
            } else if path.extension().is_some_and(|ext| ext == "torrent") {
                files.push(BrowserEntry {
                    name,
                    path,
                    is_dir: false,
                });
            }
        }

        // Sort alphabetically
        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.entries.extend(dirs);
        self.entries.extend(files);
    }

    /// Moves selection up.
    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Moves selection down.
    pub fn down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    /// Selects the current entry. Returns `Some(path)` if a .torrent file was selected.
    pub fn select(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.selected)?.clone();

        if entry.is_dir {
            self.current_dir = entry.path;
            self.refresh();
            None
        } else {
            Some(entry.path)
        }
    }
}
