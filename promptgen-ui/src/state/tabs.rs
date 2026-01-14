//! Tab state management for prompt editing tabs.

use std::collections::HashMap;

use promptgen_core::{SlotDefaults, SlotValue};
use serde::{Deserialize, Serialize};

/// Origin of a prompt tab - tracks where it came from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PromptSource {
    /// Created via [+ New] button - not yet saved to library
    #[default]
    New,
    /// Opened from a saved prompt in the library
    FromLibrary {
        /// Original name when opened (for tracking renames)
        original_name: String,
    },
}

/// A prompt tab being edited
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTab {
    /// Tab/prompt name (must be unique across tabs and library)
    pub name: String,
    /// Prompt content (the text being edited)
    pub content: String,
    /// Slot values for this tab (independent per tab)
    pub slots: HashMap<String, SlotValue>,
    /// Default separator and suffix for slots
    #[serde(default)]
    pub slot_defaults: SlotDefaults,
    /// Where this tab originated from
    pub source: PromptSource,
    /// Whether this tab has unsaved changes
    #[serde(default)]
    pub dirty: bool,
    /// Whether the prompt editor section is expanded (per-tab)
    #[serde(default = "default_true")]
    pub prompt_editor_expanded: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PromptTab {
    fn default() -> Self {
        Self {
            name: "Prompt 1".to_string(),
            content: String::new(),
            slots: HashMap::new(),
            slot_defaults: SlotDefaults::default(),
            source: PromptSource::New,
            dirty: false,
            prompt_editor_expanded: true,
        }
    }
}

impl PromptTab {
    /// Create a new tab with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            content: String::new(),
            slots: HashMap::new(),
            slot_defaults: SlotDefaults::default(),
            source: PromptSource::New,
            dirty: false,
            prompt_editor_expanded: true,
        }
    }

    /// Create a tab from a saved library prompt
    pub fn from_library_prompt(
        name: String,
        content: String,
        slots: HashMap<String, SlotValue>,
        slot_defaults: SlotDefaults,
    ) -> Self {
        Self {
            name: name.clone(),
            content,
            slots,
            slot_defaults,
            source: PromptSource::FromLibrary {
                original_name: name,
            },
            dirty: false,
            prompt_editor_expanded: true,
        }
    }
}

/// Tab rename state (ephemeral)
#[derive(Debug, Clone, Default)]
pub struct TabRenameState {
    /// Index of the tab being renamed
    pub index: usize,
    /// Current text in the rename field
    pub text: String,
}

/// State for managing prompt tabs
#[derive(Debug, Clone, Default)]
pub struct TabState {
    /// All open prompt tabs
    pub tabs: Vec<PromptTab>,
    /// Currently active tab index
    pub active_index: Option<usize>,
    /// Tab rename state (if renaming)
    pub rename: Option<TabRenameState>,
}

impl TabState {
    /// Get the active tab (immutable reference)
    pub fn get_active(&self) -> Option<&PromptTab> {
        self.active_index.and_then(|idx| self.tabs.get(idx))
    }

    /// Get the active tab (mutable reference)
    pub fn get_active_mut(&mut self) -> Option<&mut PromptTab> {
        self.active_index.and_then(|idx| self.tabs.get_mut(idx))
    }

    /// Check if there are any unsaved tabs
    pub fn has_unsaved(&self) -> bool {
        self.tabs.iter().any(|tab| tab.dirty)
    }

    /// Get the names of all unsaved tabs
    pub fn get_unsaved_names(&self) -> Vec<String> {
        self.tabs
            .iter()
            .filter(|tab| tab.dirty)
            .map(|tab| tab.name.clone())
            .collect()
    }

    /// Mark the active tab as dirty (has unsaved changes)
    pub fn mark_active_dirty(&mut self) {
        if let Some(tab) = self.get_active_mut() {
            tab.dirty = true;
        }
    }

    /// Find the next available sequential prompt name ("Prompt 1", "Prompt 2", etc.)
    pub fn find_next_prompt_name(&self, library_prompts: &[String]) -> String {
        let mut n = 1;
        loop {
            let candidate = format!("Prompt {}", n);
            if self.is_name_available(&candidate, library_prompts) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Check if a name is available (not used by any tab or library prompt)
    pub fn is_name_available(&self, name: &str, library_prompts: &[String]) -> bool {
        // Check tabs
        let used_in_tabs = self.tabs.iter().any(|t| t.name == name);
        if used_in_tabs {
            return false;
        }

        // Check library prompts
        !library_prompts.contains(&name.to_string())
    }

    /// Check if a name is available for renaming a specific tab
    /// (excludes the tab's current name from the check)
    pub fn is_name_available_for_rename(
        &self,
        name: &str,
        tab_index: usize,
        library_prompts: &[String],
    ) -> bool {
        // Check other tabs (excluding the one being renamed)
        let used_in_other_tabs = self
            .tabs
            .iter()
            .enumerate()
            .any(|(i, t)| i != tab_index && t.name == name);
        if used_in_other_tabs {
            return false;
        }

        // Check library prompts (but allow if this tab came from that prompt)
        if let Some(tab) = self.tabs.get(tab_index)
            && let PromptSource::FromLibrary { original_name } = &tab.source
        {
            // Allow keeping/restoring the original name
            if name == original_name {
                return true;
            }
        }

        !library_prompts.contains(&name.to_string())
    }

    /// Find a tab by its source library prompt name
    pub fn find_by_library_prompt(&self, prompt_name: &str) -> Option<usize> {
        self.tabs.iter().position(|tab| {
            matches!(&tab.source, PromptSource::FromLibrary { original_name } if original_name == prompt_name)
        })
    }

    /// Start renaming a tab
    pub fn start_rename(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get(index) {
            self.rename = Some(TabRenameState {
                index,
                text: tab.name.clone(),
            });
        }
    }

    /// Cancel tab rename
    pub fn cancel_rename(&mut self) {
        self.rename = None;
    }

    /// Validate the current tab rename text without committing
    /// Returns true if the name is valid and can be saved
    pub fn is_rename_valid(&self, library_prompts: &[String]) -> bool {
        let Some(rename) = &self.rename else {
            return false;
        };

        let new_name = rename.text.trim();

        // Check if name is empty
        if new_name.is_empty() {
            return false;
        }

        // Check if name is available (excluding current tab)
        self.is_name_available_for_rename(new_name, rename.index, library_prompts)
    }

    /// Commit tab rename if the new name is valid
    /// Returns true if rename was successful, false if name is invalid/duplicate
    pub fn commit_rename(&mut self, library_prompts: &[String]) -> bool {
        let Some(rename) = &self.rename else {
            return false;
        };

        let new_name = rename.text.trim().to_string();
        let index = rename.index;

        // Check if name is empty
        if new_name.is_empty() {
            return false;
        }

        // Check if name is available (excluding current tab)
        if !self.is_name_available_for_rename(&new_name, index, library_prompts) {
            return false;
        }

        // Apply the rename
        if let Some(tab) = self.tabs.get_mut(index)
            && tab.name != new_name
        {
            tab.name = new_name;
            tab.dirty = true;
        }

        // Clear rename state
        self.rename = None;

        true
    }

    /// Check if we're in a "blank" state (no tabs or no active tab)
    pub fn is_blank(&self) -> bool {
        self.tabs.is_empty() || self.active_index.is_none()
    }

    /// Close a tab by index without checking for unsaved changes
    /// Returns true if the tab was closed
    pub fn close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }

        self.tabs.remove(index);

        // Adjust active tab index
        if self.tabs.is_empty() {
            self.active_index = None;
        } else if let Some(active) = self.active_index {
            if active >= self.tabs.len() {
                // Was pointing past end, move to last tab
                self.active_index = Some(self.tabs.len() - 1);
            } else if active > index {
                // Was pointing after removed tab, shift down
                self.active_index = Some(active - 1);
            }
            // If active was pointing before removed tab, no change needed
        }

        true
    }

    /// Clear all tabs (used when resetting app state)
    pub fn clear(&mut self) {
        self.tabs.clear();
        self.active_index = None;
        self.rename = None;
    }
}
