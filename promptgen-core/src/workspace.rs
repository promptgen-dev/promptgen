//! Workspace module for multi-library template management.
//!
//! A Workspace holds multiple libraries and provides methods for:
//! - Parsing and validating templates against all loaded libraries
//! - Resolving library references (both qualified and unqualified)
//! - Autocomplete suggestions
//! - Rendering templates
//!
//! The Workspace is immutable - all mutations return new instances.

use std::rc::Rc;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ast::{LibraryRef, Node, SlotDefinition, Template};
use crate::library::{Library, PromptVariable};
use crate::parser::parse_template;
use crate::span::Span;

/// A workspace containing multiple libraries.
///
/// Workspace is designed to be immutable - use `with_library()` and
/// `without_library()` to create modified copies. Libraries are stored
/// in `Rc` for efficient cloning.
#[derive(Debug, Clone)]
pub struct Workspace {
    libraries: Vec<Rc<Library>>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Workspace {
    /// Create an empty workspace.
    pub fn new() -> Self {
        Self {
            libraries: Vec::new(),
        }
    }

    /// Create a workspace with a single library.
    pub fn with_single_library(library: Library) -> Self {
        Self {
            libraries: vec![Rc::new(library)],
        }
    }

    /// Add or update a library in the workspace.
    /// If a library with the same ID exists, it will be replaced.
    /// Returns a new workspace with the library added/updated.
    pub fn with_library(&self, library: Library) -> Self {
        let mut libraries = self.libraries.clone();
        let library_rc = Rc::new(library);

        // Check if library with same ID exists
        if let Some(pos) = libraries.iter().position(|l| l.id == library_rc.id) {
            libraries[pos] = library_rc;
        } else {
            libraries.push(library_rc);
        }

        Self { libraries }
    }

    /// Remove a library from the workspace by ID.
    /// Returns a new workspace without the specified library.
    pub fn without_library(&self, library_id: &str) -> Self {
        let libraries = self
            .libraries
            .iter()
            .filter(|l| l.id != library_id)
            .cloned()
            .collect();

        Self { libraries }
    }

    /// Get all library IDs in the workspace.
    pub fn library_ids(&self) -> Vec<&str> {
        self.libraries.iter().map(|l| l.id.as_str()).collect()
    }

    /// Get a library by ID.
    pub fn get_library(&self, id: &str) -> Option<&Library> {
        self.libraries.iter().find(|l| l.id == id).map(|rc| &**rc)
    }

    /// Get a library by name.
    pub fn get_library_by_name(&self, name: &str) -> Option<&Library> {
        self.libraries
            .iter()
            .find(|l| l.name == name)
            .map(|rc| &**rc)
    }

    /// Get all libraries in the workspace.
    pub fn libraries(&self) -> impl Iterator<Item = &Library> {
        self.libraries.iter().map(|rc| &**rc)
    }

    /// Get all variable names across all libraries.
    /// If `library_id` is Some, only returns variables from that library.
    pub fn variable_names(&self, library_id: Option<&str>) -> Vec<VariableInfo> {
        let mut variables = Vec::new();

        for lib in &self.libraries {
            if let Some(id) = library_id
                && lib.id != id
            {
                continue;
            }

            for variable in &lib.variables {
                variables.push(VariableInfo {
                    library_id: lib.id.clone(),
                    library_name: lib.name.clone(),
                    variable_name: variable.name.clone(),
                    option_count: variable.options.len(),
                });
            }
        }

        variables
    }

    /// Find a variable by name across all libraries.
    /// Returns all matches (for ambiguity detection).
    pub fn find_variables(&self, variable_name: &str) -> Vec<(&Library, &PromptVariable)> {
        let mut matches = Vec::new();

        for lib in &self.libraries {
            if let Some(variable) = lib.find_variable(variable_name) {
                matches.push((&**lib, variable));
            }
        }

        matches
    }

    /// Find a variable in a specific library by library name.
    pub fn find_variable_in_library(
        &self,
        library_name: &str,
        variable_name: &str,
    ) -> Option<(&Library, &PromptVariable)> {
        self.get_library_by_name(library_name)
            .and_then(|lib| lib.find_variable(variable_name).map(|g| (lib, g)))
    }

    /// Parse a template and validate all references against the workspace.
    pub fn parse_template(&self, source: &str) -> ParseResult {
        // First, parse the template
        let ast = match parse_template(source) {
            Ok(ast) => ast,
            Err(e) => {
                return ParseResult {
                    ast: None,
                    errors: vec![DiagnosticError {
                        message: format!("Parse error: {}", e),
                        span: 0..source.len(),
                        kind: ErrorKind::Syntax,
                        suggestion: None,
                    }],
                    warnings: vec![],
                };
            }
        };

        // Then validate all references
        let errors = self.validate_references(&ast);

        ParseResult {
            ast: Some(ast),
            errors,
            warnings: vec![],
        }
    }

    /// Validate all library references in a template.
    fn validate_references(&self, ast: &Template) -> Vec<DiagnosticError> {
        let mut errors = Vec::new();

        for (node, span) in &ast.nodes {
            if let Node::LibraryRef(lib_ref) = node
                && let Err(e) = self.validate_reference(lib_ref, span.clone())
            {
                errors.push(e);
            }
        }

        errors
    }

    /// Validate a single library reference.
    fn validate_reference(&self, lib_ref: &LibraryRef, span: Span) -> Result<(), DiagnosticError> {
        match &lib_ref.library {
            // Qualified reference: @"LibName:VariableName"
            Some(lib_name) => {
                let lib = self.get_library_by_name(lib_name).ok_or_else(|| {
                    let suggestion = self.suggest_library_name(lib_name);
                    DiagnosticError {
                        message: format!("Unknown library: {}", lib_name),
                        span: span.clone(),
                        kind: ErrorKind::UnknownLibrary,
                        suggestion,
                    }
                })?;

                if lib.find_variable(&lib_ref.variable).is_none() {
                    let suggestion = self.suggest_variable_name(&lib_ref.variable, Some(lib_name));
                    return Err(DiagnosticError {
                        message: format!(
                            "Unknown variable '{}' in library '{}'",
                            lib_ref.variable, lib_name
                        ),
                        span,
                        kind: ErrorKind::UnknownReference,
                        suggestion,
                    });
                }
            }

            // Unqualified reference: @VariableName
            None => {
                let matches = self.find_variables(&lib_ref.variable);

                if matches.is_empty() {
                    let suggestion = self.suggest_variable_name(&lib_ref.variable, None);
                    return Err(DiagnosticError {
                        message: format!("Unknown variable: {}", lib_ref.variable),
                        span,
                        kind: ErrorKind::UnknownReference,
                        suggestion,
                    });
                }

                if matches.len() > 1 {
                    let lib_names: Vec<_> = matches.iter().map(|(l, _)| l.name.as_str()).collect();
                    return Err(DiagnosticError {
                        message: format!(
                            "Ambiguous reference '{}' found in multiple libraries: {}",
                            lib_ref.variable,
                            lib_names.join(", ")
                        ),
                        span,
                        kind: ErrorKind::AmbiguousReference,
                        suggestion: Some(format!(
                            "Use qualified syntax: @\"{}:{}\"",
                            lib_names[0], lib_ref.variable
                        )),
                    });
                }
            }
        }

        Ok(())
    }

    /// Suggest a similar library name (for "did you mean?" errors).
    fn suggest_library_name(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();

        self.libraries
            .iter()
            .filter(|l| {
                let lib_lower = l.name.to_lowercase();
                lib_lower.contains(&name_lower)
                    || name_lower.contains(&lib_lower)
                    || levenshtein_distance(&lib_lower, &name_lower) <= 2
            })
            .min_by_key(|l| levenshtein_distance(&l.name.to_lowercase(), &name_lower))
            .map(|l| format!("Did you mean '{}'?", l.name))
    }

    /// Suggest a similar variable name.
    fn suggest_variable_name(&self, name: &str, library_name: Option<&str>) -> Option<String> {
        let name_lower = name.to_lowercase();
        let mut best_match: Option<(&str, &str, usize)> = None;

        for lib in &self.libraries {
            if let Some(lib_name) = library_name
                && lib.name != lib_name
            {
                continue;
            }

            for variable in &lib.variables {
                let variable_lower = variable.name.to_lowercase();
                let dist = levenshtein_distance(&variable_lower, &name_lower);

                if dist <= 3 && (best_match.is_none() || dist < best_match.unwrap().2) {
                    best_match = Some((&lib.name, &variable.name, dist));
                }
            }
        }

        best_match.map(|(lib_name, variable_name, _)| {
            if self.libraries.len() == 1 {
                format!("Did you mean @{}?", variable_name)
            } else {
                format!("Did you mean @\"{}:{}\"?", lib_name, variable_name)
            }
        })
    }

    /// Get autocomplete suggestions at a cursor position.
    pub fn get_completions(&self, source: &str, cursor_pos: usize) -> Vec<CompletionItem> {
        // Analyze context at cursor position
        let context = self.analyze_completion_context(source, cursor_pos);

        match context {
            CompletionContext::AfterAt { prefix, in_quotes } => {
                self.complete_variable_reference(&prefix, in_quotes)
            }
            CompletionContext::AfterLibraryColon {
                library_name,
                prefix,
            } => self.complete_qualified_variable(&library_name, &prefix),
            CompletionContext::InInlineOptions { prefix } => self.complete_in_options(&prefix),
            CompletionContext::None => vec![],
        }
    }

    /// Analyze the context around the cursor for autocomplete.
    fn analyze_completion_context(&self, source: &str, cursor_pos: usize) -> CompletionContext {
        let before_cursor = &source[..cursor_pos.min(source.len())];

        // Check if we're after @
        if let Some(at_pos) = before_cursor.rfind('@') {
            let after_at = &before_cursor[at_pos + 1..];

            // Check for quoted reference with library
            if let Some(content) = after_at.strip_prefix('"') {
                if let Some(colon_pos) = content.find(':') {
                    // After @"LibName:
                    let library_name = content[..colon_pos].to_string();
                    let prefix = content[colon_pos + 1..].to_string();
                    return CompletionContext::AfterLibraryColon {
                        library_name,
                        prefix,
                    };
                } else {
                    // After @" but no colon yet
                    return CompletionContext::AfterAt {
                        prefix: content.to_string(),
                        in_quotes: true,
                    };
                }
            } else {
                // Simple @identifier
                return CompletionContext::AfterAt {
                    prefix: after_at.to_string(),
                    in_quotes: false,
                };
            }
        }

        // Check if we're inside {options|...}
        if let Some(brace_pos) = before_cursor.rfind('{') {
            let after_brace = &before_cursor[brace_pos + 1..];
            // Don't match if we've closed the brace
            if !after_brace.contains('}') {
                // Get the current option text (after last |)
                let prefix = after_brace
                    .rfind('|')
                    .map(|p| &after_brace[p + 1..])
                    .unwrap_or(after_brace)
                    .trim()
                    .to_string();

                return CompletionContext::InInlineOptions { prefix };
            }
        }

        CompletionContext::None
    }

    /// Complete variable references after @.
    fn complete_variable_reference(&self, prefix: &str, in_quotes: bool) -> Vec<CompletionItem> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let prefix = prefix.trim();
        let mut scored_completions: Vec<(i64, CompletionItem)> = Vec::new();

        for lib in &self.libraries {
            // If multiple libraries, also suggest library names
            if self.libraries.len() > 1 && in_quotes {
                let score = if prefix.is_empty() {
                    Some(0)
                } else {
                    matcher.fuzzy_match(&lib.name, prefix)
                };

                if let Some(score) = score {
                    scored_completions.push((
                        score,
                        CompletionItem {
                            label: format!("{}:", lib.name),
                            kind: CompletionKind::Library,
                            detail: Some(format!("{} variables", lib.variables.len())),
                            insert_text: format!("{}:", lib.name),
                            library_id: Some(lib.id.clone()),
                        },
                    ));
                }
            }

            // Suggest variables
            for variable in &lib.variables {
                let score = if prefix.is_empty() {
                    Some(0)
                } else {
                    matcher.fuzzy_match(&variable.name, prefix)
                };

                if let Some(score) = score {
                    let insert_text = if variable.name.contains(' ') || in_quotes {
                        if self.libraries.len() > 1 {
                            format!("\"{}:{}\"", lib.name, variable.name)
                        } else {
                            format!("\"{}\"", variable.name)
                        }
                    } else {
                        variable.name.clone()
                    };

                    scored_completions.push((
                        score,
                        CompletionItem {
                            label: variable.name.clone(),
                            kind: CompletionKind::Variable,
                            detail: Some(format!("{} options", variable.options.len())),
                            insert_text,
                            library_id: Some(lib.id.clone()),
                        },
                    ));
                }
            }
        }

        // Sort by score descending (highest first)
        scored_completions.sort_by(|a, b| b.0.cmp(&a.0));
        scored_completions
            .into_iter()
            .map(|(_, item)| item)
            .collect()
    }

    /// Complete variables within a specific library.
    fn complete_qualified_variable(&self, library_name: &str, prefix: &str) -> Vec<CompletionItem> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let prefix = prefix.trim();
        let mut scored_completions: Vec<(i64, CompletionItem)> = Vec::new();

        if let Some(lib) = self.get_library_by_name(library_name) {
            for variable in &lib.variables {
                let score = if prefix.is_empty() {
                    Some(0)
                } else {
                    matcher.fuzzy_match(&variable.name, prefix)
                };

                if let Some(score) = score {
                    scored_completions.push((
                        score,
                        CompletionItem {
                            label: variable.name.clone(),
                            kind: CompletionKind::Variable,
                            detail: Some(format!("{} options", variable.options.len())),
                            insert_text: format!("{}\"", variable.name), // Close the quote
                            library_id: Some(lib.id.clone()),
                        },
                    ));
                }
            }
        }

        // Sort by score descending (highest first)
        scored_completions.sort_by(|a, b| b.0.cmp(&a.0));
        scored_completions
            .into_iter()
            .map(|(_, item)| item)
            .collect()
    }

    /// Complete inside inline options.
    fn complete_in_options(&self, prefix: &str) -> Vec<CompletionItem> {
        // If prefix starts with @, complete references
        if let Some(ref_prefix) = prefix.strip_prefix('@') {
            return self.complete_variable_reference(ref_prefix, false);
        }

        // Otherwise, no completions for plain text
        vec![]
    }

    /// Extract slot names from a parsed template.
    pub fn get_slots(&self, ast: &Template) -> Vec<String> {
        let mut slots = Vec::new();

        for (node, _span) in &ast.nodes {
            if let Node::SlotBlock(slot_block) = node {
                let name = &slot_block.label.0;
                if !slots.contains(name) {
                    slots.push(name.clone());
                }
            }
        }

        slots
    }

    /// Extract slot definitions from a parsed template.
    /// Returns normalized SlotDefinition structs with full type information.
    pub fn get_slot_definitions(&self, ast: &Template) -> Vec<SlotDefinition> {
        let mut slots = Vec::new();
        let mut seen_labels = std::collections::HashSet::new();

        for (node, _span) in &ast.nodes {
            if let Node::SlotBlock(slot_block) = node {
                let label = &slot_block.label.0;
                // Only include first occurrence of each slot label
                if seen_labels.insert(label.clone())
                    && let Ok(def) = slot_block.to_definition()
                {
                    slots.push(def);
                }
            }
        }

        slots
    }

    /// Extract library references from a parsed template.
    pub fn get_references(&self, ast: &Template) -> Vec<ReferenceInfo> {
        let mut refs = Vec::new();

        for (node, span) in &ast.nodes {
            if let Node::LibraryRef(lib_ref) = node {
                refs.push(ReferenceInfo {
                    variable: lib_ref.variable.clone(),
                    library: lib_ref.library.clone(),
                    span: span.clone(),
                });
            }
        }

        refs
    }
}

/// Builder for constructing a Workspace.
#[derive(Debug, Default)]
pub struct WorkspaceBuilder {
    libraries: Vec<Library>,
}

impl WorkspaceBuilder {
    /// Create a new workspace builder.
    pub fn new() -> Self {
        Self {
            libraries: Vec::new(),
        }
    }

    /// Add a library to the workspace.
    pub fn add_library(mut self, library: Library) -> Self {
        self.libraries.push(library);
        self
    }

    /// Build the workspace.
    pub fn build(self) -> Workspace {
        Workspace {
            libraries: self.libraries.into_iter().map(Rc::new).collect(),
        }
    }
}

/// Information about a variable.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VariableInfo {
    pub library_id: String,
    pub library_name: String,
    pub variable_name: String,
    pub option_count: usize,
}

/// Result of parsing a template.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ParseResult {
    /// The parsed AST, if parsing succeeded.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub ast: Option<Template>,
    /// Errors encountered during parsing/validation.
    pub errors: Vec<DiagnosticError>,
    /// Warnings (non-blocking issues).
    pub warnings: Vec<DiagnosticWarning>,
}

impl ParseResult {
    /// Check if the parse was successful (no errors).
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if there were errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// A diagnostic error from parsing or validation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiagnosticError {
    pub message: String,
    pub span: Span,
    pub kind: ErrorKind,
    pub suggestion: Option<String>,
}

/// Kind of diagnostic error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ErrorKind {
    Syntax,
    UnknownReference,
    UnknownLibrary,
    AmbiguousReference,
    Cycle,
}

/// A diagnostic warning.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiagnosticWarning {
    pub message: String,
    pub span: Span,
    pub kind: WarningKind,
}

/// Kind of diagnostic warning.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum WarningKind {
    Deprecated,
    Unused,
}

/// Autocomplete suggestion.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompletionItem {
    /// Display text.
    pub label: String,
    /// Kind of completion.
    pub kind: CompletionKind,
    /// Additional detail (e.g., option count).
    pub detail: Option<String>,
    /// Text to insert.
    pub insert_text: String,
    /// Source library ID.
    pub library_id: Option<String>,
}

/// Kind of autocomplete item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CompletionKind {
    Variable,
    Library,
    Option,
}

/// Information about a library reference in the AST.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReferenceInfo {
    pub variable: String,
    pub library: Option<String>,
    pub span: Span,
}

/// Context for autocomplete.
#[derive(Debug)]
enum CompletionContext {
    /// After @ symbol, possibly with a prefix.
    AfterAt { prefix: String, in_quotes: bool },
    /// After @"LibraryName:
    AfterLibraryColon {
        library_name: String,
        prefix: String,
    },
    /// Inside {option|...}
    InInlineOptions { prefix: String },
    /// No completion context.
    None,
}

/// Simple Levenshtein distance for fuzzy matching.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *val = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_workspace() -> Workspace {
        let mut lib1 = Library::with_id("lib1", "Characters");
        lib1.variables.push(PromptVariable::with_options(
            "Hair",
            vec!["blonde hair", "red hair", "black hair"],
        ));
        lib1.variables.push(PromptVariable::with_options(
            "Eyes",
            vec!["blue eyes", "green eyes"],
        ));
        lib1.variables.push(PromptVariable::with_options(
            "Eye Color",
            vec!["amber", "violet"],
        ));

        let mut lib2 = Library::with_id("lib2", "Settings");
        lib2.variables.push(PromptVariable::with_options(
            "Weather",
            vec!["sunny", "rainy", "cloudy"],
        ));
        lib2.variables.push(PromptVariable::with_options(
            "Time",
            vec!["morning", "afternoon", "evening"],
        ));

        WorkspaceBuilder::new()
            .add_library(lib1)
            .add_library(lib2)
            .build()
    }

    fn make_single_library_workspace() -> Workspace {
        let mut lib = Library::with_id("lib1", "TestLib");
        lib.variables.push(PromptVariable::with_options(
            "Hair",
            vec!["blonde", "red", "black"],
        ));
        lib.variables
            .push(PromptVariable::with_options("Eyes", vec!["blue", "green"]));

        Workspace::with_single_library(lib)
    }

    // =========================================================================
    // Workspace construction tests
    // =========================================================================

    #[test]
    fn test_workspace_new() {
        let ws = Workspace::new();
        assert!(ws.library_ids().is_empty());
    }

    #[test]
    fn test_workspace_with_library() {
        let ws = Workspace::new();
        let lib = Library::with_id("test", "Test");

        let ws2 = ws.with_library(lib);
        assert_eq!(ws2.library_ids(), vec!["test"]);
        assert!(ws.library_ids().is_empty()); // Original unchanged
    }

    #[test]
    fn test_workspace_without_library() {
        let ws = make_test_workspace();
        let ws2 = ws.without_library("lib1");

        assert_eq!(ws2.library_ids(), vec!["lib2"]);
        assert_eq!(ws.library_ids().len(), 2); // Original unchanged
    }

    #[test]
    fn test_workspace_builder() {
        let ws = WorkspaceBuilder::new()
            .add_library(Library::with_id("a", "LibA"))
            .add_library(Library::with_id("b", "LibB"))
            .build();

        assert_eq!(ws.library_ids().len(), 2);
    }

    // =========================================================================
    // Reference resolution tests
    // =========================================================================

    #[test]
    fn test_find_variables_single_match() {
        let ws = make_test_workspace();
        let matches = ws.find_variables("Hair");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0.name, "Characters");
    }

    #[test]
    fn test_find_variables_no_match() {
        let ws = make_test_workspace();
        let matches = ws.find_variables("NonExistent");

        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_variable_in_library() {
        let ws = make_test_workspace();
        let result = ws.find_variable_in_library("Characters", "Hair");

        assert!(result.is_some());
        let (lib, variable) = result.unwrap();
        assert_eq!(lib.name, "Characters");
        assert_eq!(variable.name, "Hair");
    }

    // =========================================================================
    // Parse and validate tests
    // =========================================================================

    #[test]
    fn test_parse_valid_template() {
        let ws = make_single_library_workspace();
        let result = ws.parse_template("A character with @Hair");

        assert!(result.is_ok());
        assert!(result.ast.is_some());
    }

    #[test]
    fn test_parse_unknown_reference() {
        let ws = make_single_library_workspace();
        let result = ws.parse_template("@NonExistent");

        assert!(result.has_errors());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, ErrorKind::UnknownReference);
        assert!(result.errors[0].message.contains("Unknown variable"));
    }

    #[test]
    fn test_parse_with_suggestion() {
        let ws = make_single_library_workspace();
        let result = ws.parse_template("@Hiar"); // Typo

        assert!(result.has_errors());
        assert!(result.errors[0].suggestion.is_some());
        assert!(
            result.errors[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("Hair")
        );
    }

    #[test]
    fn test_parse_quoted_reference() {
        let ws = make_test_workspace();
        let result = ws.parse_template(r#"@"Eye Color""#);

        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_qualified_reference() {
        let ws = make_test_workspace();
        let result = ws.parse_template(r#"@"Characters:Hair""#);

        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_unknown_library() {
        let ws = make_test_workspace();
        let result = ws.parse_template(r#"@"FakeLib:Hair""#);

        assert!(result.has_errors());
        assert_eq!(result.errors[0].kind, ErrorKind::UnknownLibrary);
    }

    // =========================================================================
    // Autocomplete tests
    // =========================================================================

    #[test]
    fn test_completions_after_at() {
        let ws = make_single_library_workspace();
        let completions = ws.get_completions("@", 1);

        assert!(!completions.is_empty());
        let labels: Vec<_> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"Hair"));
        assert!(labels.contains(&"Eyes"));
    }

    #[test]
    fn test_completions_with_prefix() {
        let ws = make_single_library_workspace();
        let completions = ws.get_completions("@Ha", 3);

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "Hair");
    }

    #[test]
    fn test_completions_in_quotes() {
        let ws = make_test_workspace();
        let completions = ws.get_completions("@\"", 2);

        // Should include library names and variable names
        let labels: Vec<_> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"Characters:"));
        assert!(labels.contains(&"Settings:"));
    }

    #[test]
    fn test_completions_qualified() {
        let ws = make_test_workspace();
        let completions = ws.get_completions("@\"Characters:", 13);

        let labels: Vec<_> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"Hair"));
        assert!(labels.contains(&"Eyes"));
        assert!(!labels.contains(&"Weather")); // From other library
    }

    // =========================================================================
    // Slot extraction tests
    // =========================================================================

    #[test]
    fn test_get_slots() {
        let ws = make_single_library_workspace();
        let result = ws.parse_template("Hello {{ name }}, welcome to {{ place }}!");

        assert!(result.is_ok());
        let slots = ws.get_slots(result.ast.as_ref().unwrap());

        assert_eq!(slots.len(), 2);
        assert!(slots.contains(&"name".to_string()));
        assert!(slots.contains(&"place".to_string()));
    }

    // =========================================================================
    // Reference extraction tests
    // =========================================================================

    #[test]
    fn test_get_references() {
        let ws = make_single_library_workspace();
        let result = ws.parse_template("@Hair and @Eyes");

        assert!(result.is_ok());
        let refs = ws.get_references(result.ast.as_ref().unwrap());

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].variable, "Hair");
        assert_eq!(refs[1].variable, "Eyes");
    }

    // =========================================================================
    // Levenshtein distance tests
    // =========================================================================

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn test_levenshtein_same() {
        assert_eq!(levenshtein_distance("hair", "hair"), 0);
    }

    #[test]
    fn test_levenshtein_typo() {
        assert_eq!(levenshtein_distance("hair", "hiar"), 2); // swap
        assert_eq!(levenshtein_distance("hair", "har"), 1); // deletion
        assert_eq!(levenshtein_distance("hair", "hairs"), 1); // insertion
    }
}
