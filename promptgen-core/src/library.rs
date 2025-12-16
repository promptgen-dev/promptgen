//! Library data structures for PromptGen.
//!
//! A Library contains reusable prompt groups and templates that can be
//! evaluated to produce final prompts.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ast::{Node, Template};

/// Generate a new unique ID.
pub fn new_id() -> String {
    cuid::cuid1().expect("failed to generate cuid")
}

/// Target engine hint for a template.
/// Determines how the final prompt should be formatted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EngineHint {
    #[default]
    StableDiffusion,
}

/// A library is a container for prompt groups and templates.
#[derive(Debug, Clone)]
pub struct Library {
    pub id: String,
    pub name: String,
    pub description: String,
    pub groups: Vec<PromptGroup>,
    pub templates: Vec<PromptTemplate>,
}

impl Library {
    /// Create a new library with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            description: String::new(),
            groups: Vec::new(),
            templates: Vec::new(),
        }
    }

    /// Create a new library with a specific ID (useful for testing or imports).
    pub fn with_id(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            groups: Vec::new(),
            templates: Vec::new(),
        }
    }

    /// Find a group by name.
    pub fn find_group(&self, name: &str) -> Option<&PromptGroup> {
        self.groups.iter().find(|g| g.name == name)
    }

    /// Find a template by name.
    pub fn find_template(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.iter().find(|t| t.name == name)
    }
}

/// A prompt group is a collection of related prompt options.
/// Groups are identified by their unique name within a library.
///
/// For example, a group named "Hair" can be referenced as `@Hair` in templates.
/// Group names with spaces require quoted syntax: `@"Eye Color"`.
#[derive(Debug, Clone)]
pub struct PromptGroup {
    /// Unique name for this group within the library.
    /// Examples: "Hair", "Eye Color", "My Character"
    pub name: String,
    /// Options stored as strings, parsed lazily at render time.
    /// Options can contain nested grammar (e.g., `@Color eyes`).
    pub options: Vec<String>,
}

impl PromptGroup {
    /// Create a new group with the given name and options.
    pub fn new(name: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            name: name.into(),
            options,
        }
    }

    /// Create a new group with string options.
    pub fn with_options(name: impl Into<String>, options: Vec<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            options: options.into_iter().map(Into::into).collect(),
        }
    }
}

/// A prompt template that can be evaluated against a library.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub engine_hint: EngineHint,
    pub ast: Template,
}

impl PromptTemplate {
    /// Create a new template with the given name and AST.
    pub fn new(name: impl Into<String>, ast: Template) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            description: String::new(),
            engine_hint: EngineHint::default(),
            ast,
        }
    }

    /// Create a new template with a specific ID.
    pub fn with_id(id: impl Into<String>, name: impl Into<String>, ast: Template) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            engine_hint: EngineHint::default(),
            ast,
        }
    }

    /// Extract all slots from this template.
    /// Returns slots defined by `{{ label }}` or `{{ label: pick(...) }}` syntax.
    pub fn slots(&self) -> Vec<TemplateSlot> {
        let mut slots = Vec::new();

        for (node, _span) in &self.ast.nodes {
            if let Node::SlotBlock(slot_block) = node {
                let kind = match &slot_block.kind.0 {
                    crate::ast::SlotKind::Textarea => TemplateSlotKind::Freeform,
                    crate::ast::SlotKind::Pick(_) => TemplateSlotKind::Pick,
                };
                slots.push(TemplateSlot {
                    name: slot_block.label.0.clone(),
                    kind,
                });
            }
        }

        slots
    }

    /// Extract all library references from this template.
    /// Useful for validation (checking all referenced groups exist).
    pub fn referenced_groups(&self) -> Vec<crate::ast::LibraryRef> {
        let mut refs = Vec::new();

        for (node, _span) in &self.ast.nodes {
            if let Node::LibraryRef(lib_ref) = node {
                refs.push(lib_ref.clone());
            }
        }

        refs
    }
}

/// A slot in a template that can be filled with a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSlot {
    pub name: String,
    pub kind: TemplateSlotKind,
}

/// The kind of slot in a template (legacy representation).
/// Note: This will be replaced by ast::SlotKind in a future version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSlotKind {
    /// A freeform/textarea slot.
    Freeform,
    /// A pick slot with structured selection.
    Pick,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_template;

    #[test]
    fn test_library_new() {
        let lib = Library::new("My Library");
        assert_eq!(lib.name, "My Library");
        assert!(lib.description.is_empty());
        assert!(lib.groups.is_empty());
        assert!(lib.templates.is_empty());
        assert!(!lib.id.is_empty());
    }

    #[test]
    fn test_library_find_group() {
        let mut lib = Library::new("Test");
        lib.groups.push(PromptGroup::new("Hair", vec![]));
        lib.groups.push(PromptGroup::new("Eyes", vec![]));

        assert!(lib.find_group("Hair").is_some());
        assert!(lib.find_group("Eyes").is_some());
        assert!(lib.find_group("Nose").is_none());
    }

    #[test]
    fn test_group_with_options() {
        let group = PromptGroup::with_options(
            "Hair",
            vec!["blonde hair", "red hair", "black hair"],
        );
        assert_eq!(group.name, "Hair");
        assert_eq!(group.options.len(), 3);
        assert_eq!(group.options[0], "blonde hair");
    }

    #[test]
    fn test_template_slots_freeform() {
        let ast = parse_template("Hello {{ Name }}, welcome to {{ Place }}!").unwrap();
        let template = PromptTemplate::new("greeting", ast);

        let slots = template.slots();
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].name, "Name");
        assert_eq!(slots[0].kind, TemplateSlotKind::Freeform);
        assert_eq!(slots[1].name, "Place");
        assert_eq!(slots[1].kind, TemplateSlotKind::Freeform);
    }

    #[test]
    fn test_template_referenced_groups() {
        let ast = parse_template(r#"@Hair and @"Eye Color""#).unwrap();
        let template = PromptTemplate::new("test", ast);

        let refs = template.referenced_groups();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].group, "Hair");
        assert_eq!(refs[0].library, None);
        assert_eq!(refs[1].group, "Eye Color");
        assert_eq!(refs[1].library, None);
    }

    #[test]
    fn test_template_referenced_groups_qualified() {
        let ast = parse_template(r#"@"MyLib:Hair""#).unwrap();
        let template = PromptTemplate::new("test", ast);

        let refs = template.referenced_groups();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].group, "Hair");
        assert_eq!(refs[0].library, Some("MyLib".to_string()));
    }
}
