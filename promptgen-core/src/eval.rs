//! Template evaluation module.
//!
//! Evaluates templates against a library to produce resolved prompts.
//!
//! Key features:
//! - LibraryRef resolution (finds groups by name)
//! - InlineOptions evaluation (random selection from {a|b|c})
//! - Lazy parsing of option text for nested grammar
//! - Cycle detection for circular references

use std::collections::HashMap;

use rand::prelude::*;

use crate::ast::{LibraryRef, Node, OptionItem};
use crate::library::{Library, PromptTemplate};
use crate::parser::parse_template;

/// Context for evaluating a template.
pub struct EvalContext<'a, R: Rng = StdRng> {
    /// The library containing groups and their options.
    pub library: &'a Library,
    /// Random number generator for selecting options.
    pub rng: R,
    /// Overrides for freeform slots (slot name -> value).
    pub slot_overrides: HashMap<String, String>,
    /// Stack of group names being evaluated (for cycle detection).
    eval_stack: Vec<String>,
}

impl<'a> EvalContext<'a, StdRng> {
    /// Create a new context with the given library and a random seed.
    pub fn new(library: &'a Library) -> Self {
        Self {
            library,
            rng: StdRng::from_os_rng(),
            slot_overrides: HashMap::new(),
            eval_stack: Vec::new(),
        }
    }

    /// Create a new context with a specific seed for deterministic evaluation.
    pub fn with_seed(library: &'a Library, seed: u64) -> Self {
        Self {
            library,
            rng: StdRng::seed_from_u64(seed),
            slot_overrides: HashMap::new(),
            eval_stack: Vec::new(),
        }
    }
}

impl<'a, R: Rng> EvalContext<'a, R> {
    /// Create a new context with a custom RNG.
    pub fn with_rng(library: &'a Library, rng: R) -> Self {
        Self {
            library,
            rng,
            slot_overrides: HashMap::new(),
            eval_stack: Vec::new(),
        }
    }

    /// Add a slot override.
    pub fn set_slot(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.slot_overrides.insert(name.into(), value.into());
    }

    /// Add multiple slot overrides.
    pub fn set_slots(&mut self, overrides: impl IntoIterator<Item = (String, String)>) {
        self.slot_overrides.extend(overrides);
    }
}

/// Record of which option was chosen from a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenOption {
    /// The group name that was referenced.
    pub group_name: String,
    /// The library name (if qualified reference).
    pub library_name: Option<String>,
    /// The text of the option that was selected.
    pub option_text: String,
}

/// Result of rendering a template.
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// The final rendered prompt text.
    pub text: String,
    /// Options that were chosen during rendering (for provenance).
    pub chosen_options: Vec<ChosenOption>,
    /// Slot values that were used.
    pub slot_values: HashMap<String, String>,
}

/// Error that can occur during rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("group not found: {0}")]
    GroupNotFound(String),

    #[error("group has no options: {0}")]
    EmptyGroup(String),

    #[error("circular reference detected: {0}")]
    CircularReference(String),

    #[error("parse error in option text: {0}")]
    OptionParseError(String),

    #[error("ambiguous group reference '{0}' found in multiple libraries")]
    AmbiguousGroup(String),
}

/// Render a template using the given context.
pub fn render<R: Rng>(
    template: &PromptTemplate,
    ctx: &mut EvalContext<'_, R>,
) -> Result<RenderResult, RenderError> {
    let mut output = String::new();
    let mut chosen_options = Vec::new();
    let slot_values = ctx.slot_overrides.clone();

    for (node, _span) in &template.ast.nodes {
        let text = eval_node(node, ctx, &mut chosen_options)?;
        output.push_str(&text);
    }

    Ok(RenderResult {
        text: output,
        chosen_options,
        slot_values,
    })
}

/// Evaluate a single node, returning the output text.
fn eval_node<R: Rng>(
    node: &Node,
    ctx: &mut EvalContext<'_, R>,
    chosen_options: &mut Vec<ChosenOption>,
) -> Result<String, RenderError> {
    match node {
        Node::Text(text) => Ok(text.clone()),

        Node::Comment(_) => Ok(String::new()),

        Node::Slot(slot_name) => {
            if let Some(value) = ctx.slot_overrides.get(slot_name).cloned() {
                // Slot values can contain grammar - parse and evaluate
                eval_slot_value(&value, ctx, chosen_options)
            } else {
                // Leave the slot placeholder as-is if no override provided
                Ok(format!("{{{{ {} }}}}", slot_name))
            }
        }

        Node::LibraryRef(lib_ref) => {
            let (text, chosen) = resolve_library_ref(lib_ref, ctx)?;
            chosen_options.push(chosen);
            Ok(text)
        }

        Node::InlineOptions(options) => eval_inline_options(options, ctx, chosen_options),
    }
}

/// Evaluate a slot value, which may contain grammar.
fn eval_slot_value<R: Rng>(
    value: &str,
    ctx: &mut EvalContext<'_, R>,
    chosen_options: &mut Vec<ChosenOption>,
) -> Result<String, RenderError> {
    // Parse the slot value as a template
    let ast = parse_template(value).map_err(|e| RenderError::OptionParseError(e.to_string()))?;

    let mut output = String::new();
    for (node, _span) in &ast.nodes {
        let text = eval_node(node, ctx, chosen_options)?;
        output.push_str(&text);
    }

    Ok(output)
}

/// Resolve a library reference to a random option.
fn resolve_library_ref<R: Rng>(
    lib_ref: &LibraryRef,
    ctx: &mut EvalContext<'_, R>,
) -> Result<(String, ChosenOption), RenderError> {
    let group_name = &lib_ref.group;

    // Check for circular reference
    if ctx.eval_stack.contains(group_name) {
        let chain = ctx.eval_stack.join(" -> ");
        return Err(RenderError::CircularReference(format!(
            "{} -> {}",
            chain, group_name
        )));
    }

    // Find the group
    // TODO: Handle lib_ref.library for multi-library support
    let group = ctx
        .library
        .find_group(group_name)
        .ok_or_else(|| RenderError::GroupNotFound(group_name.clone()))?;

    if group.options.is_empty() {
        return Err(RenderError::EmptyGroup(group_name.clone()));
    }

    // Pick a random option
    let idx = ctx.rng.random_range(0..group.options.len());
    let option_text = &group.options[idx];

    // Push to eval stack for cycle detection
    ctx.eval_stack.push(group_name.clone());

    // Parse and evaluate the option (lazy evaluation for nested grammar)
    let evaluated_text = eval_option_text(option_text, ctx)?;

    // Pop from eval stack
    ctx.eval_stack.pop();

    let chosen = ChosenOption {
        group_name: group_name.clone(),
        library_name: lib_ref.library.clone(),
        option_text: evaluated_text.clone(),
    };

    Ok((evaluated_text, chosen))
}

/// Evaluate option text, which may contain nested grammar.
fn eval_option_text<R: Rng>(
    option_text: &str,
    ctx: &mut EvalContext<'_, R>,
) -> Result<String, RenderError> {
    // Parse the option text as a template
    let ast =
        parse_template(option_text).map_err(|e| RenderError::OptionParseError(e.to_string()))?;

    let mut output = String::new();
    let mut temp_chosen = Vec::new();

    for (node, _span) in &ast.nodes {
        let text = eval_node(node, ctx, &mut temp_chosen)?;
        output.push_str(&text);
    }

    Ok(output)
}

/// Evaluate inline options {a|b|c}.
fn eval_inline_options<R: Rng>(
    options: &[OptionItem],
    ctx: &mut EvalContext<'_, R>,
    chosen_options: &mut Vec<ChosenOption>,
) -> Result<String, RenderError> {
    if options.is_empty() {
        return Ok(String::new());
    }

    // Pick a random option
    let idx = ctx.rng.random_range(0..options.len());
    let option = &options[idx];

    match option {
        OptionItem::Text(text) => {
            // Plain text option - but it might still contain grammar like @Hair
            // Parse and evaluate it
            eval_option_text(text, ctx)
        }
        OptionItem::Nested(nodes) => {
            // Already-parsed nested nodes
            let mut output = String::new();
            for (node, _span) in nodes {
                let text = eval_node(node, ctx, chosen_options)?;
                output.push_str(&text);
            }
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::PromptGroup;

    fn make_test_library() -> Library {
        let mut lib = Library::with_id("test-lib", "Test Library");

        lib.groups.push(PromptGroup::with_options(
            "Hair",
            vec!["blonde hair", "red hair", "black hair"],
        ));

        lib.groups.push(PromptGroup::with_options(
            "Eyes",
            vec!["blue eyes", "green eyes"],
        ));

        lib.groups.push(PromptGroup::with_options(
            "Color",
            vec!["red", "blue", "green"],
        ));

        lib
    }

    #[test]
    fn test_render_plain_text() {
        let lib = make_test_library();
        let ast = parse_template("Hello, world!").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello, world!");
        assert!(result.chosen_options.is_empty());
    }

    #[test]
    fn test_render_library_ref() {
        let lib = make_test_library();
        let ast = parse_template("A girl with @Hair").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(result.text.starts_with("A girl with "));
        assert!(
            result.text.contains("blonde hair")
                || result.text.contains("red hair")
                || result.text.contains("black hair")
        );
        assert_eq!(result.chosen_options.len(), 1);
        assert_eq!(result.chosen_options[0].group_name, "Hair");
    }

    #[test]
    fn test_render_quoted_library_ref() {
        let mut lib = make_test_library();
        lib.groups
            .push(PromptGroup::with_options("Eye Color", vec!["amber", "violet"]));

        let ast = parse_template(r#"@"Eye Color""#).unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(result.text == "amber" || result.text == "violet");
    }

    #[test]
    fn test_render_deterministic_with_seed() {
        let lib = make_test_library();
        let ast = parse_template("@Hair and @Eyes").unwrap();
        let template = PromptTemplate::new("test", ast);

        let mut ctx1 = EvalContext::with_seed(&lib, 12345);
        let result1 = render(&template, &mut ctx1).unwrap();

        let mut ctx2 = EvalContext::with_seed(&lib, 12345);
        let result2 = render(&template, &mut ctx2).unwrap();

        assert_eq!(result1.text, result2.text);
    }

    #[test]
    fn test_render_inline_options() {
        let lib = make_test_library();
        let ast = parse_template("{hot|cold} weather").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(result.text == "hot weather" || result.text == "cold weather");
    }

    #[test]
    fn test_render_slot_with_override() {
        let lib = make_test_library();
        let ast = parse_template("Hello {{ Name }}!").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);
        ctx.set_slot("Name", "Alice");

        let result = render(&template, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello Alice!");
    }

    #[test]
    fn test_render_slot_without_override() {
        let lib = make_test_library();
        let ast = parse_template("Hello {{ Name }}!").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello {{ Name }}!");
    }

    #[test]
    fn test_render_slot_with_grammar() {
        let lib = make_test_library();
        let ast = parse_template("A hero: {{ character }}").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);
        ctx.set_slot("character", "@Hair warrior");

        let result = render(&template, &mut ctx).unwrap();
        assert!(result.text.starts_with("A hero: "));
        assert!(result.text.contains("hair warrior"));
    }

    #[test]
    fn test_render_comments_not_included() {
        let lib = make_test_library();
        let ast = parse_template("Hello # this is a comment\nWorld").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(!result.text.contains("this is a comment"));
        assert!(!result.text.contains('#'));
    }

    #[test]
    fn test_render_group_not_found_error() {
        let lib = make_test_library();
        let ast = parse_template("@NonExistent").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        assert!(matches!(result, Err(RenderError::GroupNotFound(_))));
    }

    #[test]
    fn test_render_empty_group_error() {
        let mut lib = make_test_library();
        lib.groups.push(PromptGroup::new("Empty", vec![]));

        let ast = parse_template("@Empty").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        assert!(matches!(result, Err(RenderError::EmptyGroup(_))));
    }

    #[test]
    fn test_render_nested_grammar_in_options() {
        let mut lib = make_test_library();
        // Create a group with nested @Color reference
        lib.groups.push(PromptGroup::with_options(
            "FancyEyes",
            vec!["@Color eyes", "sparkling eyes"],
        ));

        let ast = parse_template("@FancyEyes").unwrap();
        let template = PromptTemplate::new("test", ast);

        // Test multiple times to cover both options
        let mut found_color_eyes = false;
        let mut found_sparkling = false;

        for seed in 0..50 {
            let mut ctx = EvalContext::with_seed(&lib, seed);
            let result = render(&template, &mut ctx).unwrap();

            if result.text.contains(" eyes") && !result.text.contains("sparkling") {
                found_color_eyes = true;
            }
            if result.text == "sparkling eyes" {
                found_sparkling = true;
            }

            if found_color_eyes && found_sparkling {
                break;
            }
        }

        assert!(found_color_eyes, "Should have found color eyes option");
        assert!(found_sparkling, "Should have found sparkling eyes option");
    }

    #[test]
    fn test_render_cycle_detection() {
        let mut lib = Library::new("Test");

        // Create a cycle: A references B, B references A
        lib.groups
            .push(PromptGroup::with_options("A", vec!["@B"]));
        lib.groups
            .push(PromptGroup::with_options("B", vec!["@A"]));

        let ast = parse_template("@A").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        assert!(matches!(result, Err(RenderError::CircularReference(_))));
    }

    #[test]
    fn test_render_mixed_template() {
        let lib = make_test_library();
        let ast = parse_template("A {big|small} creature with @Hair and @Eyes").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(result.text.contains("creature with"));
        assert!(result.text.contains(" and "));
        // Should have 2 chosen options (Hair and Eyes)
        assert_eq!(result.chosen_options.len(), 2);
    }
}
