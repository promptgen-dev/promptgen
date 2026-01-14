//! Editor state for the main prompt editor and variable editor.

use promptgen_core::ParseResult;

/// What the central editor panel is currently showing
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Normal prompt editing mode
    #[default]
    Prompt,
    /// Editing an existing variable
    VariableEditor { variable_name: String },
    /// Creating a new variable
    NewVariable,
}

/// What editor element currently has focus
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditorFocus {
    /// No editor focused
    #[default]
    None,
    /// Main prompt editor is focused
    MainEditor,
    /// A textarea slot is focused
    TextareaSlot { label: String },
    /// A pick slot is focused (opens sidebar picker)
    PickSlot { label: String },
}

/// State for the main editor and variable editor
#[derive(Debug, Clone, Default)]
pub struct EditorState {
    /// Current editor content (the prompt text being edited)
    pub content: String,
    /// Parse result for the current content
    pub parse_result: Option<ParseResult>,
    /// Current editor mode (prompt vs variable editor)
    pub mode: EditorMode,
    /// What editor element currently has focus
    pub focus: EditorFocus,
}

impl EditorState {
    /// Focus the main editor (and unfocus any slots)
    pub fn focus_main_editor(&mut self) {
        self.focus = EditorFocus::MainEditor;
    }

    /// Focus a textarea slot
    pub fn focus_textarea_slot(&mut self, slot_label: &str) {
        self.focus = EditorFocus::TextareaSlot {
            label: slot_label.to_string(),
        };
    }

    /// Focus a pick slot
    pub fn focus_pick_slot(&mut self, slot_label: &str) {
        self.focus = EditorFocus::PickSlot {
            label: slot_label.to_string(),
        };
    }

    /// Unfocus the current editor/slot
    pub fn unfocus(&mut self) {
        self.focus = EditorFocus::None;
    }

    /// Check if a specific slot is focused (pick or textarea)
    pub fn is_slot_focused(&self, slot_label: &str) -> bool {
        matches!(
            &self.focus,
            EditorFocus::PickSlot { label } | EditorFocus::TextareaSlot { label } if label == slot_label
        )
    }

    /// Check if the main editor is focused
    pub fn is_main_editor_focused(&self) -> bool {
        matches!(self.focus, EditorFocus::MainEditor)
    }

    /// Clear editor state (used when resetting app state)
    pub fn clear(&mut self) {
        self.content.clear();
        self.parse_result = None;
        self.mode = EditorMode::Prompt;
        self.focus = EditorFocus::None;
    }
}

/// State for the variable editor panel
#[derive(Debug, Clone, Default)]
pub struct VariableEditorState {
    /// Variable name being edited
    pub name: String,
    /// Variable content (options text)
    pub content: String,
    /// Original name if editing an existing variable
    pub original_name: Option<String>,
    /// Whether the variable editor has unsaved changes
    pub dirty: bool,
}

impl VariableEditorState {
    /// Parse options text into a Vec of options.
    ///
    /// Format:
    /// - Each line is a separate option by default
    /// - `---` on its own line marks the START of a multiline option
    /// - The multiline option continues until the next `---` or end of text
    pub fn parse_options(text: &str) -> Vec<String> {
        let mut options = Vec::new();
        let mut in_multiline = false;
        let mut multiline_buffer = String::new();

        for line in text.lines() {
            if line.trim() == "---" {
                if in_multiline {
                    // End of multiline option
                    let trimmed = multiline_buffer.trim().to_string();
                    if !trimmed.is_empty() {
                        options.push(trimmed);
                    }
                    multiline_buffer.clear();
                    in_multiline = false;
                } else {
                    // Start of multiline option
                    in_multiline = true;
                }
            } else if in_multiline {
                // Inside a multiline option - preserve newlines
                if !multiline_buffer.is_empty() {
                    multiline_buffer.push('\n');
                }
                multiline_buffer.push_str(line);
            } else {
                // Single-line option
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    options.push(trimmed.to_string());
                }
            }
        }

        // Handle unclosed multiline block
        if in_multiline && !multiline_buffer.trim().is_empty() {
            options.push(multiline_buffer.trim().to_string());
        }

        options
    }

    /// Convert options Vec to text format.
    ///
    /// Single-line options are output as-is (one per line).
    /// Multi-line options are wrapped with `---` delimiters.
    pub fn options_to_text(options: &[String]) -> String {
        options
            .iter()
            .map(|opt| {
                if opt.contains('\n') {
                    // Multiline option - wrap with ---
                    format!("---\n{}\n---", opt)
                } else {
                    opt.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get the current options count from the editor content
    pub fn get_option_count(&self) -> usize {
        Self::parse_options(&self.content).len()
    }

    /// Clear variable editor state
    pub fn clear(&mut self) {
        self.name.clear();
        self.content.clear();
        self.original_name = None;
        self.dirty = false;
    }

    /// Mark as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
