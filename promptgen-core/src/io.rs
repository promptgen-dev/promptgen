//! Library I/O module for loading and saving libraries to disk.
//!
//! This module provides YAML-based serialization for libraries, groups, and templates.
//! Templates are stored as source text and re-parsed on load.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::library::{EngineHint, Library, PromptGroup, PromptOption, PromptTemplate, new_id};
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
}

// ============================================================================
// Data Transfer Objects (DTOs) for YAML serialization
// ============================================================================

/// DTO for PromptGroup.
/// Groups are identified by their tags - at least one tag is required.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupDto {
    /// Tags that identify this group.
    pub tags: Vec<String>,
    #[serde(default)]
    pub options: Vec<OptionDto>,
}

/// DTO for PromptOption.
#[derive(Debug, Serialize, Deserialize)]
pub struct OptionDto {
    pub text: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

/// DTO for PromptTemplate (templates/*.yml).
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateDto {
    #[serde(default = "new_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
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
            tags: dto.tags,
            options: dto.options.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<OptionDto> for PromptOption {
    fn from(dto: OptionDto) -> Self {
        PromptOption {
            text: dto.text,
            weight: dto.weight,
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
            tags: self.tags,
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
            tags: group.tags.clone(),
            options: group.options.iter().map(Into::into).collect(),
        }
    }
}

impl From<&PromptOption> for OptionDto {
    fn from(option: &PromptOption) -> Self {
        OptionDto {
            text: option.text.clone(),
            weight: option.weight,
        }
    }
}

impl From<&PromptTemplate> for TemplateDto {
    fn from(template: &PromptTemplate) -> Self {
        TemplateDto {
            id: template.id.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            tags: template.tags.clone(),
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
    use crate::ast::Node;

    let mut source = String::new();

    for (node, _span) in &template.nodes {
        match node {
            Node::Text(text) => source.push_str(text),
            Node::TagQuery(query) => {
                source.push('{');
                source.push_str(&tag_query_to_source(query));
                source.push('}');
            }
            Node::FreeformSlot(name) => {
                source.push_str("{{ ");
                source.push_str(name);
                source.push_str(" }}");
            }
            Node::Comment(text) => {
                source.push_str("# ");
                source.push_str(text);
            }
            Node::ExprBlock(expr) => {
                source.push_str("[[ ");
                source.push_str(&expr_to_source(expr));
                source.push_str(" ]]");
            }
        }
    }

    source
}

/// Convert a TagQuery back to source string like "eyes - anime - crazy"
fn tag_query_to_source(query: &crate::ast::TagQuery) -> String {
    let mut parts = query.include.clone();
    for exclude in &query.exclude {
        parts.push(format!("- {}", exclude));
    }
    parts.join(" ")
}

fn expr_to_source(expr: &crate::ast::Expr) -> String {
    use crate::ast::Expr;

    match expr {
        Expr::Literal(s) => format!("\"{}\"", s),
        Expr::Query(query) => format!("\"{}\"", tag_query_to_source(query)),
        Expr::Pipeline(base, ops) => {
            let mut result = expr_to_source(base);
            for op in ops {
                result.push_str(" | ");
                result.push_str(&op_to_source(op));
            }
            result
        }
    }
}

fn op_to_source(op: &crate::ast::Op) -> String {
    use crate::ast::Op;

    match op {
        Op::Some => "some".to_string(),
        Op::ExcludeGroup(name) => format!("excludeGroup(\"{}\")", name),
        Op::Assign(name) => format!("assign(\"{}\")", name),
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
  - tags: [Hair, appearance]
    options:
      - text: blonde hair
      - text: red hair
templates:
  - id: tmpl-id
    name: Character
    description: A character template
    tags: [character]
    source: "{Hair} with blue eyes"
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
        assert_eq!(loaded.groups[0].tags[0], "Hair");
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
  - tags: [Colors]
    options:
      - text: red
      - text: blue
templates:
  - name: Simple
    source: "Pick a {Colors}"
"#;

        let lib = parse_pack(yaml).unwrap();

        // Library and Template IDs should be auto-generated
        assert!(!lib.id.is_empty());
        assert!(!lib.templates[0].id.is_empty());
        assert_eq!(lib.groups[0].tags[0], "Colors");
        assert_eq!(lib.groups[0].options[0].text, "red");
    }

    #[test]
    fn test_template_source_reconstruction() {
        let source = r#"{Hair} with {{ EyeColor }} and [[ "Outfit" | some | assign("outfit") ]]"#;
        let ast = parse_template(source).unwrap();
        let reconstructed = template_to_source(&ast);

        // Parse the reconstructed source and verify it works
        let reparsed = parse_template(&reconstructed).unwrap();
        assert_eq!(reparsed.nodes.len(), ast.nodes.len());
    }

    #[test]
    fn test_weighted_options_preserved() {
        let yaml = r#"
name: Weighted Test
groups:
  - tags: [Rarity]
    options:
      - text: common
        weight: 10.0
      - text: rare
        weight: 1.0
templates: []
"#;

        let lib = parse_pack(yaml).unwrap();
        assert_eq!(lib.groups[0].options[0].weight, 10.0);
        assert_eq!(lib.groups[0].options[1].weight, 1.0);
    }
}
