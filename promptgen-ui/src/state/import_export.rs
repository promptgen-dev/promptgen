//! Import and export state for variables and prompts.

use std::collections::HashSet;

use promptgen_core::{Library, PromptVariable, SavedPrompt, SlotDefaults};

use super::sidebar::VariableSortOrder;

/// An imported variable with its original name, renamed name, and options
#[derive(Debug, Clone)]
pub struct ImportedVariable {
    /// Original name from the YAML
    pub original_name: String,
    /// Renamed name (editable by user to resolve conflicts)
    pub renamed_name: String,
    /// The options for this variable
    pub options: Vec<String>,
}

/// An imported prompt with its original name, renamed name, content, and slot defaults
#[derive(Debug, Clone)]
pub struct ImportedPrompt {
    /// Original name from the YAML
    pub original_name: String,
    /// Renamed name (editable by user to resolve conflicts)
    pub renamed_name: String,
    /// The prompt content
    pub content: String,
    /// Slot defaults for this prompt
    pub slot_defaults: SlotDefaults,
}

/// State for variable export mode
#[derive(Debug, Clone, Default)]
pub struct VariableExportState {
    /// Whether export mode is active
    pub active: bool,
    /// Selected variables for export
    pub selected: HashSet<String>,
    /// Search query for filtering
    pub search_query: String,
    /// Sort order for the export list
    pub sort_order: VariableSortOrder,
}

impl VariableExportState {
    /// Enter export mode
    pub fn enter(&mut self) {
        self.active = true;
        self.selected.clear();
        self.search_query.clear();
    }

    /// Exit export mode
    pub fn exit(&mut self) {
        self.active = false;
        self.selected.clear();
        self.search_query.clear();
    }

    /// Toggle selection of a variable
    pub fn toggle(&mut self, name: &str) {
        if self.selected.contains(name) {
            self.selected.remove(name);
        } else {
            self.selected.insert(name.to_string());
        }
    }

    /// Select all variables (respects current search filter)
    pub fn select_all(&mut self, library: &Library) {
        let search = self.search_query.to_lowercase();
        for var in &library.variables {
            if search.is_empty() || var.name.to_lowercase().contains(&search) {
                self.selected.insert(var.name.clone());
            }
        }
    }

    /// Deselect all variables
    pub fn deselect_all(&mut self) {
        self.selected.clear();
    }

    /// Export selected variables to YAML string
    pub fn export_to_yaml(&self, library: &Library) -> Option<String> {
        if self.selected.is_empty() {
            return None;
        }

        #[derive(serde::Serialize)]
        struct VariableDto {
            name: String,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            options: Vec<String>,
        }

        #[derive(serde::Serialize)]
        struct VariablesExport {
            variables: Vec<VariableDto>,
        }

        let selected: Vec<VariableDto> = library
            .variables
            .iter()
            .filter(|v| self.selected.contains(&v.name))
            .map(|v| VariableDto {
                name: v.name.clone(),
                options: v.options.clone(),
            })
            .collect();

        if selected.is_empty() {
            return None;
        }

        serde_yaml::to_string(&VariablesExport {
            variables: selected,
        })
        .ok()
    }
}

/// State for variable import dialog
#[derive(Debug, Clone, Default)]
pub struct VariableImportState {
    /// Whether the import dialog is open
    pub dialog_open: bool,
    /// YAML text being imported
    pub yaml_text: String,
    /// Parsed variables from YAML
    pub parsed: Vec<ImportedVariable>,
    /// Parse error message (if any)
    pub parse_error: Option<String>,
}

impl VariableImportState {
    /// Open the import dialog
    pub fn open(&mut self) {
        self.dialog_open = true;
        self.yaml_text.clear();
        self.parsed.clear();
        self.parse_error = None;
    }

    /// Close the import dialog
    pub fn close(&mut self) {
        self.dialog_open = false;
        self.yaml_text.clear();
        self.parsed.clear();
        self.parse_error = None;
    }

    /// Parse the YAML text and update parsed variables
    pub fn parse_yaml(&mut self) {
        if self.yaml_text.trim().is_empty() {
            self.parsed.clear();
            self.parse_error = None;
            return;
        }

        #[derive(serde::Deserialize)]
        struct VariableDto {
            name: String,
            #[serde(default)]
            options: Vec<String>,
        }

        #[derive(serde::Deserialize)]
        struct VariablesImport {
            #[serde(default)]
            variables: Vec<VariableDto>,
        }

        match serde_yaml::from_str::<VariablesImport>(&self.yaml_text) {
            Ok(import) => {
                self.parse_error = None;
                self.parsed = import
                    .variables
                    .into_iter()
                    .map(|v| ImportedVariable {
                        original_name: v.name.clone(),
                        renamed_name: v.name,
                        options: v.options,
                    })
                    .collect();
            }
            Err(e) => {
                self.parse_error = Some(format!("Parse error: {}", e));
                self.parsed.clear();
            }
        }
    }

    /// Check if a specific imported variable originally had a conflict
    pub fn is_originally_conflicting(&self, index: usize, library: &Library) -> bool {
        let Some(var) = self.parsed.get(index) else {
            return false;
        };
        library.variables.iter().any(|v| v.name == var.original_name)
    }

    /// Check if a specific imported variable has a conflict
    pub fn is_name_conflicting(&self, index: usize, library: &Library) -> bool {
        let Some(var) = self.parsed.get(index) else {
            return false;
        };

        let name = &var.renamed_name;

        if name.trim().is_empty() {
            return true;
        }

        if library.variables.iter().any(|v| &v.name == name) {
            return true;
        }

        // Check against other imported variables
        for (i, other) in self.parsed.iter().enumerate() {
            if i != index && other.renamed_name == *name {
                return true;
            }
        }

        false
    }

    /// Check if import can proceed
    pub fn can_import(&self, library: &Library) -> bool {
        if self.parsed.is_empty() {
            return false;
        }

        for idx in 0..self.parsed.len() {
            if self.is_name_conflicting(idx, library) {
                return false;
            }
        }

        true
    }

    /// Perform the import - add all parsed variables to the library
    pub fn perform_import(&mut self, library: &mut Library) {
        if !self.can_import(library) {
            return;
        }

        for var in &self.parsed {
            library.variables.push(PromptVariable {
                name: var.renamed_name.clone(),
                options: var.options.clone(),
            });
        }

        self.close();
    }

    /// Update the renamed name for an imported variable
    pub fn update_renamed_name(&mut self, index: usize, new_name: String) {
        if let Some(var) = self.parsed.get_mut(index) {
            var.renamed_name = new_name;
        }
    }
}

/// State for prompt export mode
#[derive(Debug, Clone, Default)]
pub struct PromptExportState {
    /// Whether export mode is active
    pub active: bool,
    /// Selected prompts for export
    pub selected: HashSet<String>,
    /// Search query for filtering
    pub search_query: String,
}

impl PromptExportState {
    /// Enter export mode
    pub fn enter(&mut self) {
        self.active = true;
        self.selected.clear();
        self.search_query.clear();
    }

    /// Exit export mode
    pub fn exit(&mut self) {
        self.active = false;
        self.selected.clear();
        self.search_query.clear();
    }

    /// Toggle selection of a prompt
    pub fn toggle(&mut self, name: &str) {
        if self.selected.contains(name) {
            self.selected.remove(name);
        } else {
            self.selected.insert(name.to_string());
        }
    }

    /// Select all prompts (respects current search filter)
    pub fn select_all(&mut self, library: &Library) {
        let search = self.search_query.to_lowercase();
        for prompt in &library.prompts {
            if search.is_empty() || prompt.name.to_lowercase().contains(&search) {
                self.selected.insert(prompt.name.clone());
            }
        }
    }

    /// Deselect all prompts
    pub fn deselect_all(&mut self) {
        self.selected.clear();
    }

    /// Export selected prompts to YAML string
    pub fn export_to_yaml(&self, library: &Library) -> Option<String> {
        if self.selected.is_empty() {
            return None;
        }

        #[derive(serde::Serialize)]
        struct PromptDto {
            name: String,
            content: String,
            #[serde(skip_serializing_if = "SlotDefaults::is_default")]
            slot_defaults: SlotDefaults,
        }

        #[derive(serde::Serialize)]
        struct PromptsExport {
            prompts: Vec<PromptDto>,
        }

        let selected: Vec<PromptDto> = library
            .prompts
            .iter()
            .filter(|p| self.selected.contains(&p.name))
            .map(|p| PromptDto {
                name: p.name.clone(),
                content: p.content.clone(),
                slot_defaults: p.slot_defaults.clone(),
            })
            .collect();

        if selected.is_empty() {
            return None;
        }

        serde_yaml::to_string(&PromptsExport { prompts: selected }).ok()
    }
}

/// State for prompt import dialog
#[derive(Debug, Clone, Default)]
pub struct PromptImportState {
    /// Whether the import dialog is open
    pub dialog_open: bool,
    /// YAML text being imported
    pub yaml_text: String,
    /// Parsed prompts from YAML
    pub parsed: Vec<ImportedPrompt>,
    /// Parse error message (if any)
    pub parse_error: Option<String>,
}

impl PromptImportState {
    /// Open the import dialog
    pub fn open(&mut self) {
        self.dialog_open = true;
        self.yaml_text.clear();
        self.parsed.clear();
        self.parse_error = None;
    }

    /// Close the import dialog
    pub fn close(&mut self) {
        self.dialog_open = false;
        self.yaml_text.clear();
        self.parsed.clear();
        self.parse_error = None;
    }

    /// Parse the YAML text and update parsed prompts
    pub fn parse_yaml(&mut self) {
        if self.yaml_text.trim().is_empty() {
            self.parsed.clear();
            self.parse_error = None;
            return;
        }

        #[derive(serde::Deserialize)]
        struct PromptDto {
            name: String,
            #[serde(default)]
            content: String,
            #[serde(default)]
            slot_defaults: SlotDefaults,
        }

        #[derive(serde::Deserialize)]
        struct PromptsImport {
            #[serde(default)]
            prompts: Vec<PromptDto>,
        }

        match serde_yaml::from_str::<PromptsImport>(&self.yaml_text) {
            Ok(import) => {
                self.parse_error = None;
                self.parsed = import
                    .prompts
                    .into_iter()
                    .map(|p| ImportedPrompt {
                        original_name: p.name.clone(),
                        renamed_name: p.name,
                        content: p.content,
                        slot_defaults: p.slot_defaults,
                    })
                    .collect();
            }
            Err(e) => {
                self.parse_error = Some(format!("Parse error: {}", e));
                self.parsed.clear();
            }
        }
    }

    /// Check if a specific imported prompt originally had a conflict
    pub fn is_originally_conflicting(&self, index: usize, library: &Library) -> bool {
        let Some(prompt) = self.parsed.get(index) else {
            return false;
        };
        library.prompts.iter().any(|p| p.name == prompt.original_name)
    }

    /// Check if a specific imported prompt has a conflict
    pub fn is_name_conflicting(&self, index: usize, library: &Library) -> bool {
        let Some(prompt) = self.parsed.get(index) else {
            return false;
        };

        let name = &prompt.renamed_name;

        if name.trim().is_empty() {
            return true;
        }

        if library.prompts.iter().any(|p| &p.name == name) {
            return true;
        }

        // Check against other imported prompts
        for (i, other) in self.parsed.iter().enumerate() {
            if i != index && other.renamed_name == *name {
                return true;
            }
        }

        false
    }

    /// Check if import can proceed
    pub fn can_import(&self, library: &Library) -> bool {
        if self.parsed.is_empty() {
            return false;
        }

        for idx in 0..self.parsed.len() {
            if self.is_name_conflicting(idx, library) {
                return false;
            }
        }

        true
    }

    /// Perform the import - add all parsed prompts to the library
    pub fn perform_import(&mut self, library: &mut Library) {
        if !self.can_import(library) {
            return;
        }

        for prompt in &self.parsed {
            library.prompts.push(SavedPrompt {
                name: prompt.renamed_name.clone(),
                content: prompt.content.clone(),
                slots: std::collections::HashMap::new(),
                slot_defaults: prompt.slot_defaults.clone(),
            });
        }

        self.close();
    }

    /// Update the renamed name for an imported prompt
    pub fn update_renamed_name(&mut self, index: usize, new_name: String) {
        if let Some(prompt) = self.parsed.get_mut(index) {
            prompt.renamed_name = new_name;
        }
    }
}
