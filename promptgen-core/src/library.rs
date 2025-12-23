//! Library data structures for PromptGen.
//!
//! A Library contains reusable prompt variables and saved prompts.

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ast::{LibraryRef, Node, Prompt, SlotDefinition};
use crate::parser::parse_prompt;
use crate::span::Span;

/// A library is a container for prompt variables and saved prompts.
/// This is the single source of truth - there is no multi-library workspace.
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub name: String,
    pub description: String,
    pub variables: Vec<PromptVariable>,
    pub prompts: Vec<SavedPrompt>,
}

impl Library {
    /// Create a new empty library with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            variables: Vec::new(),
            prompts: Vec::new(),
        }
    }

    /// Find a variable by name.
    pub fn find_variable(&self, name: &str) -> Option<&PromptVariable> {
        self.variables.iter().find(|g| g.name == name)
    }

    /// Find a prompt by name.
    pub fn find_prompt(&self, name: &str) -> Option<&SavedPrompt> {
        self.prompts.iter().find(|p| p.name == name)
    }

    /// Check if a prompt name is already in use.
    pub fn has_prompt_named(&self, name: &str) -> bool {
        self.prompts.iter().any(|p| p.name == name)
    }

    /// Parse a prompt/prompt and validate all references against this library.
    pub fn parse_prompt(&self, source: &str) -> ParseResult {
        // First, parse the prompt
        let ast = match parse_prompt(source) {
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

    /// Validate all library references in a prompt.
    fn validate_references(&self, ast: &Prompt) -> Vec<DiagnosticError> {
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
        // With single library, we ignore any library qualifier - just look up variable name
        if self.find_variable(&lib_ref.variable).is_none() {
            let suggestion = self.suggest_variable_name(&lib_ref.variable);
            return Err(DiagnosticError {
                message: format!("Unknown variable: {}", lib_ref.variable),
                span,
                kind: ErrorKind::UnknownReference,
                suggestion,
            });
        }

        Ok(())
    }

    /// Suggest a similar variable name (for "did you mean?" errors).
    fn suggest_variable_name(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();

        self.variables
            .iter()
            .filter(|v| {
                let variable_lower = v.name.to_lowercase();
                variable_lower.contains(&name_lower)
                    || name_lower.contains(&variable_lower)
                    || levenshtein_distance(&variable_lower, &name_lower) <= 3
            })
            .min_by_key(|v| levenshtein_distance(&v.name.to_lowercase(), &name_lower))
            .map(|v| format!("Did you mean @{}?", v.name))
    }

    /// Get all variable names in the library.
    pub fn variable_names(&self) -> Vec<VariableInfo> {
        self.variables
            .iter()
            .map(|variable| VariableInfo {
                variable_name: variable.name.clone(),
                option_count: variable.options.len(),
            })
            .collect()
    }

    /// Extract slot names from a parsed prompt.
    pub fn get_slots(&self, ast: &Prompt) -> Vec<String> {
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

    /// Extract slot definitions from a parsed prompt.
    /// Returns normalized SlotDefinition structs with full type information.
    pub fn get_slot_definitions(&self, ast: &Prompt) -> Vec<SlotDefinition> {
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

    /// Extract library references from a parsed prompt.
    pub fn get_references(&self, ast: &Prompt) -> Vec<ReferenceInfo> {
        let mut refs = Vec::new();

        for (node, span) in &ast.nodes {
            if let Node::LibraryRef(lib_ref) = node {
                refs.push(ReferenceInfo {
                    variable: lib_ref.variable.clone(),
                    span: span.clone(),
                });
            }
        }

        refs
    }
}

/// A prompt variable is a collection of related prompt options.
/// Variables are identified by their unique name within a library.
///
/// For example, a variable named "Hair" can be referenced as `@Hair` in prompts.
/// Variable names with spaces require quoted syntax: `@"Eye Color"`.
#[derive(Debug, Clone)]
pub struct PromptVariable {
    /// Unique name for this variable within the library.
    /// Examples: "Hair", "Eye Color", "My Character"
    pub name: String,
    /// Options stored as strings, parsed lazily at render time.
    /// Options can contain nested grammar (e.g., `@Color eyes`).
    pub options: Vec<String>,
}

impl PromptVariable {
    /// Create a new variable with the given name and options.
    pub fn new(name: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            name: name.into(),
            options,
        }
    }

    /// Create a new variable with string options.
    pub fn with_options(name: impl Into<String>, options: Vec<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            options: options.into_iter().map(Into::into).collect(),
        }
    }
}

/// A saved prompt with its content and slot values for reproducibility.
/// Prompts are identified by their unique name within a library.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SavedPrompt {
    /// Unique name for this prompt within the library.
    pub name: String,
    /// The prompt content (prompt source).
    pub content: String,
    /// Slot values for reproducibility - maps slot label to its value.
    #[cfg_attr(feature = "serde", serde(default))]
    pub slots: HashMap<String, SlotValue>,
}

impl SavedPrompt {
    /// Create a new saved prompt with the given name and content.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            slots: HashMap::new(),
        }
    }

    /// Create a new saved prompt with name, content, and slots.
    pub fn with_slots(
        name: impl Into<String>,
        content: impl Into<String>,
        slots: HashMap<String, SlotValue>,
    ) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            slots,
        }
    }
}

/// Value stored for a slot - either text (textarea) or picks (pick slot).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum SlotValue {
    /// Single text value from a textarea slot.
    Text(String),
    /// List of selected options from a pick slot.
    Pick(Vec<String>),
}

// ============================================================================
// Diagnostic types (moved from workspace.rs)
// ============================================================================

/// Information about a variable (simplified from workspace version).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VariableInfo {
    pub variable_name: String,
    pub option_count: usize,
}

/// Result of parsing a prompt.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ParseResult {
    /// The parsed AST, if parsing succeeded.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub ast: Option<Prompt>,
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

/// Information about a library reference in the AST.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReferenceInfo {
    pub variable: String,
    pub span: Span,
}

// ============================================================================
// Helpers
// ============================================================================

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

    #[test]
    fn test_library_new() {
        let lib = Library::new("My Library");
        assert_eq!(lib.name, "My Library");
        assert!(lib.description.is_empty());
        assert!(lib.variables.is_empty());
        assert!(lib.prompts.is_empty());
    }

    #[test]
    fn test_library_find_variable() {
        let mut lib = Library::new("Test");
        lib.variables.push(PromptVariable::new("Hair", vec![]));
        lib.variables.push(PromptVariable::new("Eyes", vec![]));

        assert!(lib.find_variable("Hair").is_some());
        assert!(lib.find_variable("Eyes").is_some());
        assert!(lib.find_variable("Nose").is_none());
    }

    #[test]
    fn test_variable_with_options() {
        let variable =
            PromptVariable::with_options("Hair", vec!["blonde hair", "red hair", "black hair"]);
        assert_eq!(variable.name, "Hair");
        assert_eq!(variable.options.len(), 3);
        assert_eq!(variable.options[0], "blonde hair");
    }

    #[test]
    fn test_parse_valid_prompt() {
        let mut lib = Library::new("Test");
        lib.variables.push(PromptVariable::with_options(
            "Hair",
            vec!["blonde", "red", "black"],
        ));

        let result = lib.parse_prompt("A character with @Hair");
        assert!(result.is_ok());
        assert!(result.ast.is_some());
    }

    #[test]
    fn test_parse_unknown_reference() {
        let lib = Library::new("Test");
        let result = lib.parse_prompt("@NonExistent");

        assert!(result.has_errors());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, ErrorKind::UnknownReference);
        assert!(result.errors[0].message.contains("Unknown variable"));
    }

    #[test]
    fn test_parse_with_suggestion() {
        let mut lib = Library::new("Test");
        lib.variables
            .push(PromptVariable::with_options("Hair", vec!["blonde", "red"]));

        let result = lib.parse_prompt("@Hiar"); // Typo

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
    fn test_search_variables() {
        let mut lib = Library::new("Test");
        lib.variables
            .push(PromptVariable::with_options("Hair", vec!["blonde", "red"]));
        lib.variables
            .push(PromptVariable::with_options("Eyes", vec!["blue", "green"]));
        lib.variables
            .push(PromptVariable::with_options("Hairband", vec!["pink"]));

        let results = lib.search_variables("hair");
        assert_eq!(results.len(), 2); // Hair and Hairband
        // Hair should score higher (exact match)
        assert_eq!(results[0].variable_name, "Hair");
    }

    #[test]
    fn test_search_options() {
        let mut lib = Library::new("Test");
        lib.variables.push(PromptVariable::with_options(
            "Hair",
            vec!["blonde hair", "red hair"],
        ));
        lib.variables.push(PromptVariable::with_options(
            "Eyes",
            vec!["blue eyes", "green eyes"],
        ));

        let results = lib.search_options("blonde", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].variable_name, "Hair");
        assert_eq!(results[0].matches.len(), 1);
        assert_eq!(results[0].matches[0].text, "blonde hair");
    }

    #[test]
    fn test_saved_prompt() {
        let prompt = SavedPrompt::new("Portrait", "A @Style portrait of @Character");
        assert_eq!(prompt.name, "Portrait");
        assert_eq!(prompt.content, "A @Style portrait of @Character");
        assert!(prompt.slots.is_empty());
    }

    #[test]
    fn test_saved_prompt_with_slots() {
        let mut slots = HashMap::new();
        slots.insert(
            "style".to_string(),
            SlotValue::Pick(vec!["oil painting".to_string()]),
        );
        slots.insert(
            "desc".to_string(),
            SlotValue::Text("wise wizard".to_string()),
        );

        let prompt = SavedPrompt::with_slots("Portrait", "A {{ style }} of {{ desc }}", slots);
        assert_eq!(prompt.name, "Portrait");
        assert_eq!(prompt.slots.len(), 2);
    }

    #[test]
    fn test_slot_value_types() {
        let text_val = SlotValue::Text("hello".to_string());
        let pick_val = SlotValue::Pick(vec!["a".to_string(), "b".to_string()]);

        assert!(matches!(text_val, SlotValue::Text(_)));
        assert!(matches!(pick_val, SlotValue::Pick(_)));
    }

    #[test]
    fn test_has_prompt_named() {
        let mut lib = Library::new("Test");
        lib.prompts.push(SavedPrompt::new("Portrait", "content"));

        assert!(lib.has_prompt_named("Portrait"));
        assert!(!lib.has_prompt_named("Landscape"));
    }

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
