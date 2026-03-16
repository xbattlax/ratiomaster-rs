//! Reusable dropdown selector state.

/// State for a dropdown/popup list selector.
#[derive(Debug, Clone)]
pub struct Dropdown {
    /// All available items.
    pub items: Vec<String>,
    /// Currently selected index.
    pub selected: usize,
    /// Whether the dropdown popup is open.
    pub open: bool,
    /// Scroll offset for long lists.
    pub scroll: usize,
}

impl Dropdown {
    pub fn new(items: Vec<String>, initial: &str) -> Self {
        let selected = items.iter().position(|s| s == initial).unwrap_or(0);
        Self {
            items,
            selected,
            open: false,
            scroll: 0,
        }
    }

    /// Opens the dropdown popup.
    pub fn open(&mut self) {
        self.open = true;
        // Ensure selected item is visible
        self.scroll_to_selected();
    }

    /// Closes the dropdown popup without changing selection.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Returns the currently selected item text.
    pub fn current(&self) -> &str {
        self.items
            .get(self.selected)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Move selection up.
    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.scroll_to_selected();
        }
    }

    /// Move selection down.
    pub fn down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
            self.scroll_to_selected();
        }
    }

    /// Confirm selection and close.
    pub fn confirm(&mut self) -> &str {
        self.open = false;
        self.current()
    }

    /// Ensure the selected item is within the visible scroll window.
    fn scroll_to_selected(&mut self) {
        let visible = 15_usize; // max visible items in popup
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + visible {
            self.scroll = self.selected - visible + 1;
        }
    }

    /// Number of visible items in the popup.
    pub fn visible_count(&self) -> usize {
        15.min(self.items.len())
    }
}
