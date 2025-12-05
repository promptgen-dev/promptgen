//! Library data structures for PromptGen.
//!
//! A Library contains reusable prompt groups and templates that can be
//! evaluated to produce final prompts.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ast::{Node, Template};

/// Generate a new CUID for use as an ID.
pub fn new_id() -> String {
    cuid::cuid1().expect("CUID generation should not fail")
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

    /// Find a group by tag.
    /// Returns the first group that has the given tag.
    pub fn find_group(&self, tag: &str) -> Option<&PromptGroup> {
        self.groups
            .iter()
            .find(|g| g.tags.contains(&tag.to_string()))
    }

    /// Find all groups that have the given tag.
    pub fn find_groups_by_tag(&self, tag: &str) -> Vec<&PromptGroup> {
        self.groups
            .iter()
            .filter(|g| g.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Find a template by name.
    pub fn find_template(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.iter().find(|t| t.name == name)
    }
}

/// A prompt group is a collection of related prompt options.
/// Groups are identified by their tags.
/// For example, a group with tags `["Hair", "appearance"]` can be referenced as `{Hair}` or `{appearance}`.
#[derive(Debug, Clone)]
pub struct PromptGroup {
    /// Tags that identify this group.
    /// Tags can be any string: "Hair", "hair-color", "Character Hair", etc.
    pub tags: Vec<String>,
    pub options: Vec<PromptOption>,
}

impl PromptGroup {
    /// Create a new group with the given tags and options.
    pub fn new(tags: Vec<String>, options: Vec<PromptOption>) -> Self {
        Self { tags, options }
    }
}

/// A single option within a prompt group.
#[derive(Debug, Clone)]
pub struct PromptOption {
    pub text: String,
    pub weight: f32,
}

impl PromptOption {
    /// Create a new option with the given text and default weight of 1.0.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            weight: 1.0,
        }
    }

    /// Create a new option with the given text and weight.
    pub fn with_weight(text: impl Into<String>, weight: f32) -> Self {
        Self {
            text: text.into(),
            weight,
        }
    }
}

/// A prompt template that can be evaluated against a library.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
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
            tags: Vec::new(),
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
            tags: Vec::new(),
            engine_hint: EngineHint::default(),
            ast,
        }
    }

    /// Extract all slots from this template.
    /// Returns both freeform slots (`{{ Name }}`) and assigned slots from expressions.
    pub fn slots(&self) -> Vec<TemplateSlot> {
        let mut slots = Vec::new();

        for (node, _span) in &self.ast.nodes {
            match node {
                Node::FreeformSlot(name) => {
                    slots.push(TemplateSlot {
                        name: name.clone(),
                        kind: SlotKind::Freeform,
                    });
                }
                Node::ExprBlock(expr) => {
                    // Look for assign operations in the expression
                    if let Some(slot) = extract_assigned_slot(expr) {
                        slots.push(slot);
                    }
                }
                _ => {}
            }
        }

        slots
    }

    /// Extract all tag references from this template.
    /// Useful for validation (checking all referenced tags exist).
    /// Returns all tags from both include and exclude parts of queries.
    pub fn referenced_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();

        for (node, _span) in &self.ast.nodes {
            match node {
                Node::TagQuery(query) => {
                    tags.extend(query.include.clone());
                    tags.extend(query.exclude.clone());
                }
                Node::ExprBlock(expr) => {
                    collect_tags_from_expr(expr, &mut tags);
                }
                _ => {}
            }
        }

        tags
    }
}

/// A slot in a template that can be filled with a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSlot {
    pub name: String,
    pub kind: SlotKind,
}

/// The kind of slot in a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotKind {
    /// A freeform slot from `{{ Name }}` syntax.
    Freeform,
    /// An assigned slot from `| assign("name")` in an expression.
    Assigned,
}

/// Extract an assigned slot from an expression if it contains an assign operation.
fn extract_assigned_slot(expr: &crate::ast::Expr) -> Option<TemplateSlot> {
    use crate::ast::{Expr, Op};

    match expr {
        Expr::Pipeline(_, ops) => {
            for op in ops {
                if let Op::Assign(name) = op {
                    return Some(TemplateSlot {
                        name: name.clone(),
                        kind: SlotKind::Assigned,
                    });
                }
            }
            None
        }
        _ => None,
    }
}

/// Collect all tag references from an expression.
fn collect_tags_from_expr(expr: &crate::ast::Expr, tags: &mut Vec<String>) {
    use crate::ast::Expr;

    match expr {
        Expr::Literal(name) => {
            // A literal is interpreted as a tag name
            tags.push(name.clone());
        }
        Expr::Query(query) => {
            tags.extend(query.include.clone());
            tags.extend(query.exclude.clone());
        }
        Expr::Pipeline(base, _) => collect_tags_from_expr(base, tags),
    }
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
        lib.groups.push(PromptGroup::new(vec!["Hair".to_string()], vec![]));
        lib.groups.push(PromptGroup::new(vec!["Eyes".to_string()], vec![]));

        assert!(lib.find_group("Hair").is_some());
        assert!(lib.find_group("Eyes").is_some());
        assert!(lib.find_group("Nose").is_none());
    }

    #[test]
    fn test_template_slots_freeform() {
        let ast = parse_template("Hello {{ Name }}, welcome to {{ Place }}!").unwrap();
        let template = PromptTemplate::new("greeting", ast);

        let slots = template.slots();
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].name, "Name");
        assert_eq!(slots[0].kind, SlotKind::Freeform);
        assert_eq!(slots[1].name, "Place");
        assert_eq!(slots[1].kind, SlotKind::Freeform);
    }

    #[test]
    fn test_template_slots_assigned() {
        let ast = parse_template(r#"[[ "Hair" | some | assign("hair") ]]"#).unwrap();
        let template = PromptTemplate::new("test", ast);

        let slots = template.slots();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].name, "hair");
        assert_eq!(slots[0].kind, SlotKind::Assigned);
    }

    #[test]
    fn test_template_referenced_tags() {
        let ast = parse_template("{Hair} and {Eyes} with [[ \"Outfit\" | some ]]").unwrap();
        let template = PromptTemplate::new("test", ast);

        let tags = template.referenced_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"Hair".to_string()));
        assert!(tags.contains(&"Eyes".to_string()));
        assert!(tags.contains(&"Outfit".to_string()));
    }

    #[test]
    fn test_template_referenced_tags_with_exclusions() {
        let ast = parse_template("{Eyes - anime - crazy}").unwrap();
        let template = PromptTemplate::new("test", ast);

        let tags = template.referenced_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"Eyes".to_string()));
        assert!(tags.contains(&"anime".to_string()));
        assert!(tags.contains(&"crazy".to_string()));
    }
}
