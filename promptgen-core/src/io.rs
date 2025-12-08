//! Library I/O module for loading and saving libraries to disk.
//!
//! This module provides YAML-based serialization for libraries, groups, and templates.
//! Templates are stored as source text and re-parsed on load.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{LibraryRef, Node, OptionItem};
use crate::library::{EngineHint, Library, PromptGroup, PromptTemplate, new_id};
use crate::parser::parse_template;

/// Error type for I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("failed to read file: {0}")]
    ReadFile(#[from] std::io::Error),

    #[error("failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("failed to parse template '{name}': {message}")]
    TemplateParse { name: String, message: String },

    #[error("duplicate group name: '{0}'")]
    DuplicateGroupName(String),
}

// ============================================================================
// Data Transfer Objects (DTOs) for YAML serialization
// ============================================================================

/// DTO for PromptGroup.
/// Groups are identified by their unique name.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupDto {
    /// Unique name for this group.
    pub name: String,
    /// Options as strings (may contain nested grammar).
    #[serde(default)]
    pub options: Vec<String>,
}

/// DTO for PromptTemplate.
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateDto {
    #[serde(default = "new_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub engine_hint: EngineHint,
    /// The template source text (will be parsed into AST on load).
    pub source: String,
}

/// DTO for a complete library pack (single-file format).
#[derive(Debug, Serialize, Deserialize)]
pub struct PackDto {
    #[serde(default = "new_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub groups: Vec<GroupDto>,
    #[serde(default)]
    pub templates: Vec<TemplateDto>,
}

// ============================================================================
// Conversion: DTO -> Domain types
// ============================================================================

impl From<GroupDto> for PromptGroup {
    fn from(dto: GroupDto) -> Self {
        PromptGroup {
            name: dto.name,
            options: dto.options,
        }
    }
}

impl TemplateDto {
    /// Convert to PromptTemplate, parsing the source text.
    pub fn try_into_template(self) -> Result<PromptTemplate, IoError> {
        let ast = parse_template(&self.source).map_err(|e| IoError::TemplateParse {
            name: self.name.clone(),
            message: e.to_string(),
        })?;

        Ok(PromptTemplate {
            id: self.id,
            name: self.name,
            description: self.description,
            engine_hint: self.engine_hint,
            ast,
        })
    }
}

// ============================================================================
// Conversion: Domain types -> DTO
// ============================================================================

impl From<&PromptGroup> for GroupDto {
    fn from(group: &PromptGroup) -> Self {
        GroupDto {
            name: group.name.clone(),
            options: group.options.clone(),
        }
    }
}

impl From<&PromptTemplate> for TemplateDto {
    fn from(template: &PromptTemplate) -> Self {
        TemplateDto {
            id: template.id.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            engine_hint: template.engine_hint.clone(),
            source: template_to_source(&template.ast),
        }
    }
}

impl From<&Library> for PackDto {
    fn from(library: &Library) -> Self {
        PackDto {
            id: library.id.clone(),
            name: library.name.clone(),
            description: library.description.clone(),
            groups: library.groups.iter().map(Into::into).collect(),
            templates: library.templates.iter().map(Into::into).collect(),
        }
    }
}

/// Reconstruct source text from a parsed template AST.
fn template_to_source(template: &crate::ast::Template) -> String {
    let mut source = String::new();

    for (node, _span) in &template.nodes {
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

        Node::Slot(name) => {
            output.push_str("{{ ");
            output.push_str(name);
            output.push_str(" }}");
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

    let needs_quotes = lib_ref.library.is_some()
        || lib_ref.group.contains(' ')
        || lib_ref.group.contains(':');

    if needs_quotes {
        output.push('"');
        if let Some(lib) = &lib_ref.library {
            output.push_str(lib);
            output.push(':');
        }
        output.push_str(&lib_ref.group);
        output.push('"');
    } else {
        output.push_str(&lib_ref.group);
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

// ============================================================================
// Library I/O (single YAML file)
// ============================================================================

/// Load a library from a YAML file.
///
/// The file should contain the complete library: metadata, groups, and templates.
pub fn load_library(path: &Path) -> Result<Library, IoError> {
    load_pack(path)
}

/// Save a library to a YAML file.
///
/// Writes the complete library (metadata, groups, templates) to a single file.
pub fn save_library(library: &Library, path: &Path) -> Result<(), IoError> {
    save_pack(library, path)
}

// ============================================================================
// Pack format (single-file) I/O
// ============================================================================

/// Load a library from a pack file (single YAML file).
pub fn load_pack(path: &Path) -> Result<Library, IoError> {
    let content = fs::read_to_string(path)?;
    let pack: PackDto = serde_yaml_ng::from_str(&content)?;

    let mut templates = Vec::new();
    for template_dto in pack.templates {
        templates.push(template_dto.try_into_template()?);
    }

    Ok(Library {
        id: pack.id,
        name: pack.name,
        description: pack.description,
        groups: pack.groups.into_iter().map(Into::into).collect(),
        templates,
    })
}

/// Save a library as a pack file (single YAML file).
pub fn save_pack(library: &Library, path: &Path) -> Result<(), IoError> {
    let pack: PackDto = library.into();
    let content = serde_yaml_ng::to_string(&pack)?;
    fs::write(path, content)?;
    Ok(())
}

/// Parse a library from a YAML string (pack format).
pub fn parse_pack(yaml: &str) -> Result<Library, IoError> {
    let pack: PackDto = serde_yaml_ng::from_str(yaml)?;

    // Check for duplicate group names
    let mut seen_names = std::collections::HashSet::new();
    for group in &pack.groups {
        if !seen_names.insert(&group.name) {
            return Err(IoError::DuplicateGroupName(group.name.clone()));
        }
    }

    let mut templates = Vec::new();
    for template_dto in pack.templates {
        templates.push(template_dto.try_into_template()?);
    }

    Ok(Library {
        id: pack.id,
        name: pack.name,
        description: pack.description,
        groups: pack.groups.into_iter().map(Into::into).collect(),
        templates,
    })
}

/// Serialize a library to a YAML string (pack format).
pub fn serialize_pack(library: &Library) -> Result<String, IoError> {
    let pack: PackDto = library.into();
    Ok(serde_yaml_ng::to_string(&pack)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TEST_LIBRARY_YAML: &str = r#"
id: test-lib-id
name: Test Library
description: A test library
groups:
  - name: Hair
    options:
      - blonde hair
      - red hair
templates:
  - id: tmpl-id
    name: Character
    description: A character template
    source: "@Hair with blue eyes"
"#;

    fn make_test_library() -> Library {
        parse_pack(TEST_LIBRARY_YAML).expect("TEST_LIBRARY_YAML should be valid")
    }

    #[test]
    fn test_pack_round_trip() {
        let lib = make_test_library();

        let yaml = serialize_pack(&lib).unwrap();
        let loaded = parse_pack(&yaml).unwrap();

        assert_eq!(loaded.id, lib.id);
        assert_eq!(loaded.name, lib.name);
        assert_eq!(loaded.description, lib.description);
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.groups[0].name, "Hair");
        assert_eq!(loaded.groups[0].options.len(), 2);
        assert_eq!(loaded.templates.len(), 1);
        assert_eq!(loaded.templates[0].name, "Character");
    }

    #[test]
    fn test_library_file_round_trip() {
        let lib = make_test_library();
        let dir = tempdir().unwrap();
        let lib_path = dir.path().join("my-library.yml");

        save_library(&lib, &lib_path).unwrap();
        let loaded = load_library(&lib_path).unwrap();

        assert_eq!(loaded.id, lib.id);
        assert_eq!(loaded.name, lib.name);
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.templates.len(), 1);
    }

    #[test]
    fn test_pack_file_round_trip() {
        let lib = make_test_library();
        let dir = tempdir().unwrap();
        let pack_path = dir.path().join("library.promptgen-pack.yml");

        save_pack(&lib, &pack_path).unwrap();
        let loaded = load_pack(&pack_path).unwrap();

        assert_eq!(loaded.id, lib.id);
        assert_eq!(loaded.name, lib.name);
    }

    #[test]
    fn test_ids_auto_generated_when_missing() {
        let yaml = r#"
name: Minimal Library
groups:
  - name: Colors
    options:
      - red
      - blue
templates:
  - name: Simple
    source: "Pick a {red|blue}"
"#;

        let lib = parse_pack(yaml).unwrap();

        // Library and Template IDs should be auto-generated
        assert!(!lib.id.is_empty());
        assert!(!lib.templates[0].id.is_empty());
        assert_eq!(lib.groups[0].name, "Colors");
        assert_eq!(lib.groups[0].options[0], "red");
    }

    #[test]
    fn test_template_source_reconstruction() {
        let source = r#"@Hair with {{ EyeColor }} and {red|blue|green}"#;
        let ast = parse_template(source).unwrap();
        let reconstructed = template_to_source(&ast);

        // Parse the reconstructed source and verify it works
        let reparsed = parse_template(&reconstructed).unwrap();
        assert_eq!(reparsed.nodes.len(), ast.nodes.len());
    }

    #[test]
    fn test_template_source_reconstruction_qualified_ref() {
        let source = r#"@"MyLib:Hair Color" with @Eyes"#;
        let ast = parse_template(source).unwrap();
        let reconstructed = template_to_source(&ast);

        // Verify the qualified reference is preserved
        assert!(reconstructed.contains(r#"@"MyLib:Hair Color""#));
        assert!(reconstructed.contains("@Eyes"));
    }

    #[test]
    fn test_template_source_reconstruction_inline_options() {
        let source = r#"A {big|small} {red|blue|green} car"#;
        let ast = parse_template(source).unwrap();
        let reconstructed = template_to_source(&ast);

        assert_eq!(reconstructed, source);
    }

    #[test]
    fn test_template_source_reconstruction_slot() {
        let source = r#"Hello {{ Name }}, welcome!"#;
        let ast = parse_template(source).unwrap();
        let reconstructed = template_to_source(&ast);

        assert_eq!(reconstructed, source);
    }

    #[test]
    fn test_template_source_reconstruction_comment() {
        let source = "# This is a comment";
        let ast = parse_template(source).unwrap();
        let reconstructed = template_to_source(&ast);

        assert_eq!(reconstructed, source);
    }

    #[test]
    fn test_duplicate_group_name_error() {
        let yaml = r#"
name: Test Library
groups:
  - name: Color
    options:
      - red
  - name: Color
    options:
      - blue
"#;

        let result = parse_pack(yaml);
        assert!(matches!(result, Err(IoError::DuplicateGroupName(name)) if name == "Color"));
    }
}
