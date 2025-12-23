//! Library I/O module for loading and saving libraries to disk.
//!
//! This module provides YAML-based serialization for libraries, variables, and prompts.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{LibraryRef, Node, OptionItem};
use crate::library::{Library, PromptVariable, SavedPrompt, SlotValue};

/// Error type for I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("failed to read file: {0}")]
    ReadFile(#[from] std::io::Error),

    #[error("failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("duplicate variable name: '{0}'")]
    DuplicateVariableName(String),

    #[error("duplicate prompt name: '{0}'")]
    DuplicatePromptName(String),
}

// ============================================================================
// Data Transfer Objects (DTOs) for YAML serialization
// ============================================================================

/// DTO for PromptVariable.
/// Variables are identified by their unique name.
#[derive(Debug, Serialize, Deserialize)]
pub struct VariableDto {
    /// Unique name for this variable.
    pub name: String,
    /// Options as strings (may contain nested grammar).
    #[serde(default)]
    pub options: Vec<String>,
}

/// DTO for a slot value - either text or picks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SlotValueDto {
    /// Text value (textarea slot).
    Text(String),
    /// List of selected options (pick slot).
    Pick(Vec<String>),
}

impl From<SlotValueDto> for SlotValue {
    fn from(dto: SlotValueDto) -> Self {
        match dto {
            SlotValueDto::Text(s) => SlotValue::Text(s),
            SlotValueDto::Pick(v) => SlotValue::Pick(v),
        }
    }
}

impl From<&SlotValue> for SlotValueDto {
    fn from(val: &SlotValue) -> Self {
        match val {
            SlotValue::Text(s) => SlotValueDto::Text(s.clone()),
            SlotValue::Pick(v) => SlotValueDto::Pick(v.clone()),
        }
    }
}

/// DTO for a saved prompt.
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptDto {
    /// Unique name for this prompt.
    pub name: String,
    /// The prompt content (prompt source).
    pub content: String,
    /// Slot values for reproducibility.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub slots: HashMap<String, SlotValueDto>,
}

/// DTO for a complete library (single-file format).
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryDto {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub variables: Vec<VariableDto>,
    #[serde(default)]
    pub prompts: Vec<PromptDto>,
}

// ============================================================================
// Conversion: DTO -> Domain types
// ============================================================================

impl From<VariableDto> for PromptVariable {
    fn from(dto: VariableDto) -> Self {
        PromptVariable {
            name: dto.name,
            options: dto.options,
        }
    }
}

impl From<PromptDto> for SavedPrompt {
    fn from(dto: PromptDto) -> Self {
        SavedPrompt {
            name: dto.name,
            content: dto.content,
            slots: dto.slots.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

// ============================================================================
// Conversion: Domain types -> DTO
// ============================================================================

impl From<&PromptVariable> for VariableDto {
    fn from(variable: &PromptVariable) -> Self {
        VariableDto {
            name: variable.name.clone(),
            options: variable.options.clone(),
        }
    }
}

impl From<&SavedPrompt> for PromptDto {
    fn from(prompt: &SavedPrompt) -> Self {
        PromptDto {
            name: prompt.name.clone(),
            content: prompt.content.clone(),
            slots: prompt
                .slots
                .iter()
                .map(|(k, v)| (k.clone(), v.into()))
                .collect(),
        }
    }
}

impl From<&Library> for LibraryDto {
    fn from(library: &Library) -> Self {
        LibraryDto {
            name: library.name.clone(),
            description: library.description.clone(),
            variables: library.variables.iter().map(Into::into).collect(),
            prompts: library.prompts.iter().map(Into::into).collect(),
        }
    }
}

/// Reconstruct source text from a parsed prompt AST.
pub fn prompt_to_source(prompt: &crate::ast::Prompt) -> String {
    let mut source = String::new();

    for (node, _span) in &prompt.nodes {
        node_to_source(node, &mut source);
    }

    source
}

/// Convert a single node to its source representation.
fn node_to_source(node: &Node, output: &mut String) {
    match node {
        Node::Text(text) => output.push_str(text),

        Node::Comment(text) => {
            output.push_str("# ");
            output.push_str(text);
        }

        Node::SlotBlock(slot_block) => {
            slot_block_to_source(slot_block, output);
        }

        Node::LibraryRef(lib_ref) => {
            library_ref_to_source(lib_ref, output);
        }

        Node::InlineOptions(options) => {
            output.push('{');
            for (i, option) in options.iter().enumerate() {
                if i > 0 {
                    output.push('|');
                }
                option_item_to_source(option, output);
            }
            output.push('}');
        }
    }
}

/// Convert a library reference to source.
fn library_ref_to_source(lib_ref: &LibraryRef, output: &mut String) {
    output.push('@');

    // In single-library mode, we never need library qualifiers
    // but we still need quotes for names with spaces or colons
    let needs_quotes = lib_ref.variable.contains(' ') || lib_ref.variable.contains(':');

    if needs_quotes {
        output.push('"');
        output.push_str(&lib_ref.variable);
        output.push('"');
    } else {
        output.push_str(&lib_ref.variable);
    }
}

/// Convert an option item to source.
fn option_item_to_source(item: &OptionItem, output: &mut String) {
    match item {
        OptionItem::Text(text) => output.push_str(text),
        OptionItem::Nested(nodes) => {
            for (node, _span) in nodes {
                node_to_source(node, output);
            }
        }
    }
}

/// Convert a slot block to source.
fn slot_block_to_source(slot_block: &crate::ast::SlotBlock, output: &mut String) {
    use crate::ast::{PickOperator, PickSource, SlotKind};

    output.push_str("{{ ");

    // Label - quote if it contains special characters
    let label = &slot_block.label.0;
    let needs_quotes = label.contains(':') || label.contains('"') || label.contains('}');
    if needs_quotes {
        output.push('"');
        output.push_str(label);
        output.push('"');
    } else {
        output.push_str(label);
    }

    // Kind
    match &slot_block.kind.0 {
        SlotKind::Textarea => {
            // Nothing more to add for textarea
        }
        SlotKind::Pick(pick) => {
            output.push_str(": pick(");

            // Sources
            for (i, (source, _span)) in pick.sources.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                match source {
                    PickSource::VariableRef(lib_ref) => {
                        library_ref_to_source(lib_ref, output);
                    }
                    PickSource::Literal { value, quoted } => {
                        if *quoted {
                            // Preserve quotes for quoted literals
                            output.push('"');
                            output.push_str(&value.replace('\\', "\\\\").replace('"', "\\\""));
                            output.push('"');
                        } else {
                            // Bare literals stay bare
                            output.push_str(value);
                        }
                    }
                }
            }

            output.push(')');

            // Operators
            for (op, _span) in &pick.operators {
                match op {
                    PickOperator::One => {
                        output.push_str(" | one");
                    }
                    PickOperator::Many(spec) => {
                        output.push_str(" | many");
                        if spec.max.is_some() || spec.sep.is_some() {
                            output.push('(');
                            let mut first = true;
                            if let Some(max) = spec.max {
                                output.push_str(&format!("max={}", max));
                                first = false;
                            }
                            if let Some(sep) = &spec.sep {
                                if !first {
                                    output.push_str(", ");
                                }
                                output.push_str(&format!("sep=\"{}\"", sep));
                            }
                            output.push(')');
                        }
                    }
                }
            }
        }
    }

    output.push_str(" }}");
}

// ============================================================================
// Library I/O (single YAML file)
// ============================================================================

/// Load a library from a YAML file.
///
/// The file should contain: name, description, variables, and prompts.
pub fn load_library(path: &Path) -> Result<Library, IoError> {
    let content = fs::read_to_string(path)?;
    parse_library(&content)
}

/// Save a library to a YAML file.
///
/// Writes the complete library (metadata, variables, prompts) to a single file.
pub fn save_library(library: &Library, path: &Path) -> Result<(), IoError> {
    let content = serialize_library(library)?;
    fs::write(path, content)?;
    Ok(())
}

/// Parse a library from a YAML string.
pub fn parse_library(yaml: &str) -> Result<Library, IoError> {
    let dto: LibraryDto = serde_yaml_ng::from_str(yaml)?;

    // Check for duplicate variable names
    let mut seen_vars = std::collections::HashSet::new();
    for variable in &dto.variables {
        if !seen_vars.insert(&variable.name) {
            return Err(IoError::DuplicateVariableName(variable.name.clone()));
        }
    }

    // Check for duplicate prompt names
    let mut seen_prompts = std::collections::HashSet::new();
    for prompt in &dto.prompts {
        if !seen_prompts.insert(&prompt.name) {
            return Err(IoError::DuplicatePromptName(prompt.name.clone()));
        }
    }

    Ok(Library {
        name: dto.name,
        description: dto.description,
        variables: dto.variables.into_iter().map(Into::into).collect(),
        prompts: dto.prompts.into_iter().map(Into::into).collect(),
    })
}

/// Serialize a library to a YAML string.
pub fn serialize_library(library: &Library) -> Result<String, IoError> {
    let dto: LibraryDto = library.into();
    Ok(serde_yaml_ng::to_string(&dto)?)
}

// ============================================================================
// Legacy pack format support (for backwards compatibility)
// ============================================================================

/// Load a library from a pack file (legacy format, same as load_library).
pub fn load_pack(path: &Path) -> Result<Library, IoError> {
    load_library(path)
}

/// Save a library as a pack file (legacy format, same as save_library).
pub fn save_pack(library: &Library, path: &Path) -> Result<(), IoError> {
    save_library(library, path)
}

/// Parse a library from a YAML string (legacy pack format).
pub fn parse_pack(yaml: &str) -> Result<Library, IoError> {
    parse_library(yaml)
}

/// Serialize a library to a YAML string (legacy pack format).
pub fn serialize_pack(library: &Library) -> Result<String, IoError> {
    serialize_library(library)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TEST_LIBRARY_YAML: &str = r#"
name: Test Library
description: A test library
variables:
  - name: Hair
    options:
      - blonde hair
      - red hair
prompts:
  - name: Character Portrait
    content: "@Hair with blue eyes"
    slots:
      style:
        - oil painting
      background: "standing in a field"
"#;

    fn make_test_library() -> Library {
        parse_library(TEST_LIBRARY_YAML).expect("TEST_LIBRARY_YAML should be valid")
    }

    #[test]
    fn test_library_round_trip() {
        let lib = make_test_library();

        let yaml = serialize_library(&lib).unwrap();
        let loaded = parse_library(&yaml).unwrap();

        assert_eq!(loaded.name, lib.name);
        assert_eq!(loaded.description, lib.description);
        assert_eq!(loaded.variables.len(), 1);
        assert_eq!(loaded.variables[0].name, "Hair");
        assert_eq!(loaded.variables[0].options.len(), 2);
        assert_eq!(loaded.prompts.len(), 1);
        assert_eq!(loaded.prompts[0].name, "Character Portrait");
    }

    #[test]
    fn test_library_file_round_trip() {
        let lib = make_test_library();
        let dir = tempdir().unwrap();
        let lib_path = dir.path().join("library.yml");

        save_library(&lib, &lib_path).unwrap();
        let loaded = load_library(&lib_path).unwrap();

        assert_eq!(loaded.name, lib.name);
        assert_eq!(loaded.variables.len(), 1);
        assert_eq!(loaded.prompts.len(), 1);
    }

    #[test]
    fn test_prompt_with_slots() {
        let yaml = r#"
name: Test
variables: []
prompts:
  - name: Portrait
    content: "{{ style }} of {{ desc }}"
    slots:
      style:
        - oil painting
        - watercolor
      desc: "a wise wizard"
"#;

        let lib = parse_library(yaml).unwrap();
        assert_eq!(lib.prompts.len(), 1);

        let prompt = &lib.prompts[0];
        assert_eq!(prompt.slots.len(), 2);

        // style is a pick (list)
        assert!(matches!(prompt.slots.get("style"), Some(SlotValue::Pick(v)) if v.len() == 2));

        // desc is text (string)
        assert!(
            matches!(prompt.slots.get("desc"), Some(SlotValue::Text(s)) if s == "a wise wizard")
        );
    }

    #[test]
    fn test_minimal_library() {
        let yaml = r#"
name: Minimal
variables:
  - name: Colors
    options:
      - red
      - blue
"#;

        let lib = parse_library(yaml).unwrap();

        assert_eq!(lib.name, "Minimal");
        assert!(lib.prompts.is_empty());
        assert_eq!(lib.variables.len(), 1);
        assert_eq!(lib.variables[0].name, "Colors");
    }

    #[test]
    fn test_duplicate_variable_name_error() {
        let yaml = r#"
name: Test Library
variables:
  - name: Color
    options:
      - red
  - name: Color
    options:
      - blue
"#;

        let result = parse_library(yaml);
        assert!(matches!(result, Err(IoError::DuplicateVariableName(name)) if name == "Color"));
    }

    #[test]
    fn test_duplicate_prompt_name_error() {
        let yaml = r#"
name: Test Library
variables: []
prompts:
  - name: Portrait
    content: "test"
  - name: Portrait
    content: "another"
"#;

        let result = parse_library(yaml);
        assert!(matches!(result, Err(IoError::DuplicatePromptName(name)) if name == "Portrait"));
    }

    #[test]
    fn test_prompt_source_reconstruction() {
        use crate::parser::parse_prompt;

        let source = r#"@Hair with {{ EyeColor }} and {red|blue|green}"#;
        let ast = parse_prompt(source).unwrap();
        let reconstructed = prompt_to_source(&ast);

        // Parse the reconstructed source and verify it works
        let reparsed = parse_prompt(&reconstructed).unwrap();
        assert_eq!(reparsed.nodes.len(), ast.nodes.len());
    }

    #[test]
    fn test_prompt_source_reconstruction_quoted_ref() {
        use crate::parser::parse_prompt;

        let source = r#"@"Hair Color" with @Eyes"#;
        let ast = parse_prompt(source).unwrap();
        let reconstructed = prompt_to_source(&ast);

        // Verify the quoted reference is preserved
        assert!(reconstructed.contains(r#"@"Hair Color""#));
        assert!(reconstructed.contains("@Eyes"));
    }

    #[test]
    fn test_prompt_source_reconstruction_slot() {
        use crate::parser::parse_prompt;

        let source = r#"Hello {{ Name }}, welcome!"#;
        let ast = parse_prompt(source).unwrap();
        let reconstructed = prompt_to_source(&ast);

        assert_eq!(reconstructed, source);
    }
}
