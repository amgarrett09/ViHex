//! An enum to represent the mode of the editor. This helps facilitate the use of
//! Vim-style commands to navigate and manipulate text.

/// Represents the mode of the editor.
pub enum EditorMode {
    /// In normal mode, keys are used primarily for navigation and as macros for
    /// common functions.
    Normal,
    /// In insert mode, user can edit text content
    Insert,
}

impl ToString for EditorMode {
    fn to_string(&self) -> String {
        match self {
            EditorMode::Normal => "NORMAL".to_string(),
            EditorMode::Insert => "INSERT".to_string(),
        }
    }
}

impl EditorMode {
    /// Tests if editor is in normal mode.
    pub fn is_normal(&self) -> bool {
        match self {
            EditorMode::Normal => true,
            _ => false,
        }
    }

    /// Tests if editor is in insert mode.
    pub fn is_insert(&self) -> bool {
        match self {
            EditorMode::Insert => true,
            _ => false,
        }
    }
}
