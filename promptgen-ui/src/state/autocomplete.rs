//! Autocomplete state management for per-editor autocomplete functionality.

use std::collections::HashMap;

/// Autocomplete mode - what kind of completions to show
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutocompleteMode {
    /// Completing variable names (@Var...)
    Variables,
    /// Completing options within a specific variable (@Var/opt...)
    Options { variable_name: String },
}

/// Autocomplete state for a single editor instance
#[derive(Debug, Clone, Default)]
pub struct AutocompleteState {
    /// Whether autocomplete popup is active
    pub active: bool,
    /// The query being completed (text after @)
    pub query: String,
    /// The mode (variables or options within a variable)
    pub mode: Option<AutocompleteMode>,
    /// Currently selected completion index
    pub selected_index: usize,
    /// Byte position where the @ symbol starts in the editor
    pub trigger_position: usize,
    /// The response ID of the text editor for popup positioning
    pub editor_response_id: Option<egui::Id>,
    /// Flag to scroll to selection (set on keyboard nav, cleared after scroll)
    pub scroll_to_autocomplete_selection: bool,
    /// Trigger position where autocomplete was dismissed via Escape.
    /// Don't auto-reactivate at this position until cursor moves away or Ctrl+Space is pressed.
    pub dismissed_trigger_position: Option<usize>,
    /// Flag indicating the editor should be refocused after Escape dismissal.
    /// This counters egui's TextEdit default behavior of losing focus on Escape.
    pub needs_refocus: bool,
}

/// Manager for per-editor autocomplete states
#[derive(Debug, Clone, Default)]
pub struct AutocompleteManager {
    /// Per-editor autocomplete states, keyed by editor ID
    states: HashMap<String, AutocompleteState>,
    /// Pending cursor positions per-editor, keyed by editor ID
    pending_cursor_positions: HashMap<String, usize>,
}

impl AutocompleteManager {
    /// Get the autocomplete state for a specific editor
    pub fn get(&self, editor_id: &str) -> Option<&AutocompleteState> {
        self.states.get(editor_id)
    }

    /// Get mutable autocomplete state for a specific editor, creating if needed
    pub fn get_mut(&mut self, editor_id: &str) -> &mut AutocompleteState {
        self.states.entry(editor_id.to_string()).or_default()
    }

    /// Take the scroll_to_selection flag (returns current value and clears it)
    pub fn take_scroll_flag(&mut self, editor_id: &str) -> bool {
        if let Some(state) = self.states.get_mut(editor_id) {
            let should_scroll = state.scroll_to_autocomplete_selection;
            state.scroll_to_autocomplete_selection = false;
            should_scroll
        } else {
            false
        }
    }

    /// Check if autocomplete is active for a specific editor
    pub fn is_active(&self, editor_id: &str) -> bool {
        self.states.get(editor_id).is_some_and(|s| s.active)
    }

    /// Try to activate autocomplete at the given trigger position.
    /// Won't activate if this trigger position was previously dismissed via Escape.
    /// Returns true if activation succeeded.
    pub fn try_activate(&mut self, editor_id: &str, trigger_position: usize) -> bool {
        let state = self.get_mut(editor_id);
        // Don't re-activate if this trigger position was dismissed via Escape
        if state.dismissed_trigger_position == Some(trigger_position) {
            return false;
        }
        state.active = true;
        state.trigger_position = trigger_position;
        state.query.clear();
        state.mode = Some(AutocompleteMode::Variables);
        state.selected_index = 0;
        state.dismissed_trigger_position = None;
        true
    }

    /// Force-activate autocomplete (via Ctrl+Space), clearing any dismissed state.
    pub fn force_activate(&mut self, editor_id: &str, trigger_position: usize) {
        let state = self.get_mut(editor_id);
        state.active = true;
        state.trigger_position = trigger_position;
        state.query.clear();
        state.mode = Some(AutocompleteMode::Variables);
        state.selected_index = 0;
        state.dismissed_trigger_position = None;
    }

    /// Deactivate autocomplete for a specific editor
    pub fn deactivate(&mut self, editor_id: &str) {
        if let Some(state) = self.states.get_mut(editor_id) {
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            state.editor_response_id = None;
            // Note: we don't clear dismissed_trigger_position here so focus loss
            // doesn't reset it - only Escape sets it
        }
    }

    /// Deactivate autocomplete via Escape, remembering the trigger position.
    /// Autocomplete won't auto-reactivate at this position until cursor moves away
    /// or user presses Ctrl+Space.
    pub fn deactivate_escaped(&mut self, editor_id: &str) {
        if let Some(state) = self.states.get_mut(editor_id) {
            let trigger_pos = state.trigger_position;
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            state.editor_response_id = None;
            state.dismissed_trigger_position = Some(trigger_pos);
            state.needs_refocus = true;
        }
    }

    /// Take the needs_refocus flag for an editor (returns true once, then clears).
    pub fn take_needs_refocus(&mut self, editor_id: &str) -> bool {
        if let Some(state) = self.states.get_mut(editor_id) {
            let needs = state.needs_refocus;
            state.needs_refocus = false;
            needs
        } else {
            false
        }
    }

    /// Clear the dismissed trigger position for an editor.
    /// Called when cursor moves away from the dismissed context.
    pub fn clear_dismissed(&mut self, editor_id: &str) {
        if let Some(state) = self.states.get_mut(editor_id) {
            state.dismissed_trigger_position = None;
        }
    }

    /// Deactivate autocomplete for all editors except the specified one
    pub fn deactivate_except(&mut self, editor_id: &str) {
        for (id, state) in &mut self.states {
            if id != editor_id {
                state.active = false;
                state.query.clear();
                state.mode = None;
                state.selected_index = 0;
                state.trigger_position = 0;
                state.editor_response_id = None;
            }
        }
    }

    /// Update autocomplete query based on cursor position and text content for a specific editor
    pub fn update_query(&mut self, editor_id: &str, content: &str, cursor_pos: usize) {
        let Some(state) = self.states.get_mut(editor_id) else {
            return;
        };
        if !state.active {
            return;
        }

        // Extract text from trigger position to cursor
        let trigger = state.trigger_position;
        if cursor_pos <= trigger || cursor_pos > content.len() {
            // Cursor moved before the @, deactivate
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            return;
        }

        // Get the text after @ up to cursor
        let query_text = &content[trigger + 1..cursor_pos]; // +1 to skip the @

        // Check if query contains whitespace or invalid chars (cancel autocomplete)
        if query_text.contains(char::is_whitespace) {
            state.active = false;
            state.query.clear();
            state.mode = None;
            state.selected_index = 0;
            state.trigger_position = 0;
            return;
        }

        // Parse the query to determine mode
        if let Some(slash_pos) = query_text.find('/') {
            // @Variable/option syntax - switch to options mode
            let variable_part = &query_text[..slash_pos];
            let option_part = &query_text[slash_pos + 1..];

            // Only reset selection if the query actually changed
            let new_query = option_part.to_string();
            let query_changed = state.query != new_query;

            state.mode = Some(AutocompleteMode::Options {
                variable_name: variable_part.to_string(),
            });
            state.query = new_query;

            if query_changed {
                state.selected_index = 0;
            }
        } else {
            // @Variable syntax - stay in variables mode
            let new_query = query_text.to_string();
            let query_changed = state.query != new_query;

            state.mode = Some(AutocompleteMode::Variables);
            state.query = new_query;

            if query_changed {
                state.selected_index = 0;
            }
        }
    }

    /// Move autocomplete selection up for a specific editor
    pub fn move_up(&mut self, editor_id: &str, total_items: usize) {
        if total_items == 0 {
            return;
        }
        if let Some(state) = self.states.get_mut(editor_id) {
            if state.selected_index == 0 {
                state.selected_index = total_items - 1;
            } else {
                state.selected_index -= 1;
            }
            state.scroll_to_autocomplete_selection = true;
        }
    }

    /// Move autocomplete selection down for a specific editor
    pub fn move_down(&mut self, editor_id: &str, total_items: usize) {
        if total_items == 0 {
            return;
        }
        if let Some(state) = self.states.get_mut(editor_id) {
            state.selected_index = (state.selected_index + 1) % total_items;
            state.scroll_to_autocomplete_selection = true;
        }
    }

    /// Set pending cursor position for a specific editor
    pub fn set_pending_cursor_position(&mut self, editor_id: &str, position: usize) {
        self.pending_cursor_positions
            .insert(editor_id.to_string(), position);
    }

    /// Take pending cursor position for a specific editor (returns and clears it)
    pub fn take_pending_cursor_position(&mut self, editor_id: &str) -> Option<usize> {
        self.pending_cursor_positions.remove(editor_id)
    }

    /// Clear all autocomplete state (used when resetting app state)
    pub fn clear(&mut self) {
        self.states.clear();
        self.pending_cursor_positions.clear();
    }
}
