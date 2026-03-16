//! Text input field with cursor management.

/// Editable single-line text input state.
#[derive(Debug, Clone)]
pub struct TextInput {
    /// Current text content.
    pub value: String,
    /// Cursor position (byte offset into value, always on a char boundary).
    pub cursor: usize,
}

impl TextInput {
    pub fn new(value: String) -> Self {
        let cursor = value.len();
        Self { value, cursor }
    }

    pub fn from_u64(v: u64) -> Self {
        Self::new(v.to_string())
    }

    /// Set value and move cursor to end.
    pub fn set(&mut self, value: String) {
        self.cursor = value.len();
        self.value = value;
    }

    /// Insert a character at the cursor position.
    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the previous char boundary
            let prev = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }

    /// Delete the character at the cursor position.
    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Move cursor left one character.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right one character.
    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.value.len());
        }
    }

    /// Move cursor to the beginning.
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end.
    pub fn end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Parse as u64, returns 0 on failure.
    pub fn as_u64(&self) -> u64 {
        self.value.parse().unwrap_or(0)
    }

    /// Parse as u16, returns 0 on failure.
    pub fn as_u16(&self) -> u16 {
        self.value.parse().unwrap_or(0)
    }

    /// Returns the text before the cursor and after the cursor for rendering.
    pub fn split_at_cursor(&self) -> (&str, &str) {
        self.value.split_at(self.cursor)
    }
}
