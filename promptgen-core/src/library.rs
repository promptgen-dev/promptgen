//! Library data structures for PromptGen.
//!
//! A Library contains reusable prompt groups and templates that can be
//! evaluated to produce final prompts.

use crate::ast::{Node, Template};

/// Generate a new CUID for use as an ID.
pub fn new_id() -> String {
    cuid::cuid1().expect("CUID generation should not fail")
}

/// Target engine hint for a template.
/// Determines how the final prompt should be formatted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
/// For example, a "Hair" group might contain options like "blonde hair", "red hair", etc.
#[derive(Debug, Clone)]
pub struct PromptGroup {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub options: Vec<PromptOption>,
}

impl PromptGroup {
    /// Create a new group with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            tags: Vec::new(),
            options: Vec::new(),
        }
    }

    /// Create a new group with a specific ID.
    pub fn with_id(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            tags: Vec::new(),
            options: Vec::new(),
        }
    }

    /// Add an option to this group.
    pub fn add_option(&mut self, option: PromptOption) {
        self.options.push(option);
    }

    /// Create and add a simple text option.
    pub fn add_text(&mut self, text: impl Into<String>) {
        self.options.push(PromptOption::new(text));
    }
}

/// A single option within a prompt group.
#[derive(Debug, Clone)]
pub struct PromptOption {
    pub id: String,
    pub text: String,
    pub weight: f32,
}

impl PromptOption {
    /// Create a new option with the given text and default weight of 1.0.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            text: text.into(),
            weight: 1.0,
        }
    }

    /// Create a new option with the given text and weight.
    pub fn with_weight(text: impl Into<String>, weight: f32) -> Self {
        Self {
            id: new_id(),
            text: text.into(),
            weight,
        }
    }

    /// Create a new option with a specific ID.
    pub fn with_id(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            weight: 1.0,
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

    /// Extract all group references from this template.
    /// Useful for validation (checking all referenced groups exist).
    pub fn referenced_groups(&self) -> Vec<String> {
        let mut groups = Vec::new();

        for (node, _span) in &self.ast.nodes {
            match node {
                Node::GroupRef(name) => {
                    groups.push(name.clone());
                }
                Node::ExprBlock(expr) => {
                    if let Some(group) = extract_group_from_expr(expr) {
                        groups.push(group);
                    }
                }
                _ => {}
            }
        }

        groups
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

/// Extract the group name from an expression (from a literal that represents a group name).
fn extract_group_from_expr(expr: &crate::ast::Expr) -> Option<String> {
    use crate::ast::Expr;

    match expr {
        Expr::Literal(name) => Some(name.clone()),
        Expr::GroupRef(name) => Some(name.clone()),
        Expr::Pipeline(base, _) => extract_group_from_expr(base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_template;

    #[test]
    fn test_new_id_generates_unique_ids() {
        let id1 = new_id();
        let id2 = new_id();
        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
    }

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
        lib.groups.push(PromptGroup::new("Hair"));
        lib.groups.push(PromptGroup::new("Eyes"));

        assert!(lib.find_group("Hair").is_some());
        assert!(lib.find_group("Eyes").is_some());
        assert!(lib.find_group("Nose").is_none());
    }

    #[test]
    fn test_prompt_group_add_options() {
        let mut group = PromptGroup::new("Hair");
        group.add_text("blonde hair");
        group.add_text("red hair");
        group.add_option(PromptOption::with_weight("black hair", 2.0));

        assert_eq!(group.options.len(), 3);
        assert_eq!(group.options[0].text, "blonde hair");
        assert_eq!(group.options[0].weight, 1.0);
        assert_eq!(group.options[2].weight, 2.0);
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
    fn test_template_referenced_groups() {
        let ast = parse_template("{Hair} and {Eyes} with [[ \"Outfit\" | some ]]").unwrap();
        let template = PromptTemplate::new("test", ast);

        let groups = template.referenced_groups();
        assert_eq!(groups.len(), 3);
        assert!(groups.contains(&"Hair".to_string()));
        assert!(groups.contains(&"Eyes".to_string()));
        assert!(groups.contains(&"Outfit".to_string()));
    }
}
