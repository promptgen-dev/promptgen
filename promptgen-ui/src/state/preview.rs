//! Preview state for the prompt preview panel.

use std::collections::{HashMap, HashSet};

use promptgen_core::{SlotDefKind, SlotDefinition, SlotValue};

/// State for the preview panel
#[derive(Debug, Clone)]
pub struct PreviewState {
    /// The rendered preview output
    pub output: String,
    /// Current random seed for rendering
    pub seed: Option<u64>,
    /// Slot values for the current prompt (working copy)
    pub slot_values: HashMap<String, Vec<String>>,
    /// Whether to auto-randomize seed on each render
    pub auto_randomize_seed: bool,
    /// Whether to auto-render when content changes
    pub auto_render: bool,
    /// Whether to auto-copy rendered output to clipboard
    pub auto_copy: bool,
    /// Whether a render is pending
    pub dirty: bool,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            output: String::new(),
            seed: None,
            slot_values: HashMap::new(),
            auto_randomize_seed: true,
            auto_render: true,
            auto_copy: false,
            dirty: false,
        }
    }
}

impl PreviewState {
    /// Request a preview render (if auto_render is enabled).
    pub fn request_render(&mut self) {
        if self.auto_render {
            self.dirty = true;
        }
    }

    /// Generate a new random seed
    pub fn randomize_seed(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        self.seed = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(42),
        );
    }

    /// Get the textarea value for a slot (first value or empty string)
    pub fn get_textarea_value(&self, slot_label: &str) -> String {
        self.slot_values
            .get(slot_label)
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default()
    }

    /// Set the single value for a textarea slot
    pub fn set_textarea_value(&mut self, slot_label: &str, value: String) -> bool {
        if let Some(values) = self.slot_values.get_mut(slot_label) {
            let old_value = values.first().cloned().unwrap_or_default();
            if old_value != value {
                values.clear();
                if !value.is_empty() {
                    values.push(value);
                }
                return true;
            }
        }
        false
    }

    /// Clear all slot values
    pub fn clear_all_slot_values(&mut self) -> bool {
        let had_values = self.slot_values.values().any(|v| !v.is_empty());
        for values in self.slot_values.values_mut() {
            values.clear();
        }
        had_values
    }

    /// Clear a single slot's values
    pub fn clear_slot(&mut self, slot_label: &str) -> bool {
        if let Some(values) = self.slot_values.get_mut(slot_label)
            && !values.is_empty()
        {
            values.clear();
            return true;
        }
        false
    }

    /// Convert SlotValue HashMap to Vec<String> HashMap (working format)
    pub fn slot_values_from_tab(
        slots: &HashMap<String, SlotValue>,
    ) -> HashMap<String, Vec<String>> {
        slots
            .iter()
            .map(|(k, v)| {
                let vec = match v {
                    SlotValue::Text(s) => {
                        if s.is_empty() {
                            Vec::new()
                        } else {
                            vec![s.clone()]
                        }
                    }
                    SlotValue::Pick(items) => items.clone(),
                };
                (k.clone(), vec)
            })
            .collect()
    }

    /// Convert Vec<String> HashMap back to SlotValue HashMap
    pub fn slot_values_to_tab(
        vec_map: &HashMap<String, Vec<String>>,
        definitions: &[SlotDefinition],
    ) -> HashMap<String, SlotValue> {
        vec_map
            .iter()
            .map(|(k, v)| {
                // Look up the slot definition to determine type
                let is_textarea = definitions
                    .iter()
                    .find(|d| d.label == *k)
                    .is_some_and(|d| matches!(d.kind, SlotDefKind::Textarea));

                let slot_value = if is_textarea {
                    SlotValue::Text(v.first().cloned().unwrap_or_default())
                } else {
                    SlotValue::Pick(v.clone())
                };
                (k.clone(), slot_value)
            })
            .collect()
    }

    /// Clear preview state (used when resetting app state)
    pub fn clear(&mut self) {
        self.output.clear();
        self.seed = None;
        self.slot_values.clear();
        self.dirty = false;
    }
}

/// State for slot manual edit mode
#[derive(Debug, Clone, Default)]
pub struct SlotManualEditState {
    /// Slots currently in manual edit mode
    pub active: HashSet<String>,
    /// Manual edit text per slot
    pub text: HashMap<String, String>,
}

impl SlotManualEditState {
    /// Check if a slot is in manual edit mode
    pub fn is_active(&self, slot_label: &str) -> bool {
        self.active.contains(slot_label)
    }

    /// Get the manual edit text for a slot
    pub fn get_text(&self, slot_label: &str) -> String {
        self.text.get(slot_label).cloned().unwrap_or_default()
    }

    /// Set the manual edit text for a slot
    pub fn set_text(&mut self, slot_label: &str, text: String) {
        self.text.insert(slot_label.to_string(), text);
    }

    /// Enter manual edit mode for a slot, converting current values to text
    pub fn enter(&mut self, slot_label: &str, current_values: &[String], separator: &str) {
        if !self.active.contains(slot_label) {
            let text = current_values.join(separator);
            self.text.insert(slot_label.to_string(), text);
            self.active.insert(slot_label.to_string());
        }
    }

    /// Exit manual edit mode for a slot, returning the parsed values
    /// For single-select: just trim the text as a single value
    /// For multi-select: split by separator, trim each, filter empties
    pub fn exit(
        &mut self,
        slot_label: &str,
        separator: &str,
        is_single_select: bool,
    ) -> Option<Vec<String>> {
        if self.active.remove(slot_label)
            && let Some(text) = self.text.remove(slot_label)
        {
            let new_values = if is_single_select {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    vec![]
                } else {
                    vec![trimmed.to_string()]
                }
            } else {
                text.split(separator)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };
            return Some(new_values);
        }
        None
    }

    /// Clear all manual edit state
    pub fn clear(&mut self) {
        self.active.clear();
        self.text.clear();
    }
}
