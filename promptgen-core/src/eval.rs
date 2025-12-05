//! Template evaluation module.
//!
//! Evaluates templates against a library to produce resolved prompts.

use std::collections::HashMap;

use rand::prelude::*;

use crate::ast::{Expr, Node, Op};
use crate::library::{Library, PromptGroup, PromptOption, PromptTemplate};

/// Context for evaluating a template.
pub struct EvalContext<'a, R: Rng = StdRng> {
    /// The library containing groups and their options.
    pub library: &'a Library,
    /// Random number generator for selecting options.
    pub rng: R,
    /// Overrides for freeform slots (slot name -> value).
    pub slot_overrides: HashMap<String, String>,
}

impl<'a> EvalContext<'a, StdRng> {
    /// Create a new context with the given library and a random seed.
    pub fn new(library: &'a Library) -> Self {
        Self {
            library,
            rng: StdRng::from_os_rng(),
            slot_overrides: HashMap::new(),
        }
    }

    /// Create a new context with a specific seed for deterministic evaluation.
    pub fn with_seed(library: &'a Library, seed: u64) -> Self {
        Self {
            library,
            rng: StdRng::seed_from_u64(seed),
            slot_overrides: HashMap::new(),
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
    pub group_name: String,
    pub option_text: String,
}

/// Result of rendering a template.
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// The final rendered prompt text.
    pub text: String,
    /// Options that were chosen during rendering (for provenance).
    pub chosen_options: Vec<ChosenOption>,
    /// Slot values that were used (both overrides and assigned).
    pub slot_values: HashMap<String, String>,
}

/// Error that can occur during rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("group not found: {0}")]
    GroupNotFound(String),

    #[error("group has no options: {0}")]
    EmptyGroup(String),
}

/// Render a template using the given context.
pub fn render<R: Rng>(
    template: &PromptTemplate,
    ctx: &mut EvalContext<'_, R>,
) -> Result<RenderResult, RenderError> {
    let mut output = String::new();
    let mut chosen_options = Vec::new();
    let mut slot_values = ctx.slot_overrides.clone();

    for (node, _span) in &template.ast.nodes {
        match node {
            Node::Text(text) => {
                output.push_str(text);
            }

            Node::Comment(_) => {
                // Comments are not included in output
            }

            Node::GroupRef(group_name) => {
                let chosen = pick_from_group(group_name, ctx)?;
                output.push_str(&chosen.option_text);
                chosen_options.push(chosen);
            }

            Node::FreeformSlot(slot_name) => {
                if let Some(value) = ctx.slot_overrides.get(slot_name) {
                    output.push_str(value);
                } else {
                    // Leave the slot placeholder as-is if no override provided
                    output.push_str("{{ ");
                    output.push_str(slot_name);
                    output.push_str(" }}");
                }
            }

            Node::ExprBlock(expr) => {
                let (text, maybe_chosen, maybe_assign) = eval_expr(expr, ctx)?;
                output.push_str(&text);

                if let Some(chosen) = maybe_chosen {
                    chosen_options.push(chosen);
                }

                if let Some((slot_name, value)) = maybe_assign {
                    slot_values.insert(slot_name, value);
                }
            }
        }
    }

    Ok(RenderResult {
        text: output,
        chosen_options,
        slot_values,
    })
}

/// Pick a random option from a group using weighted selection.
fn pick_from_group<R: Rng>(
    group_name: &str,
    ctx: &mut EvalContext<'_, R>,
) -> Result<ChosenOption, RenderError> {
    let group = ctx
        .library
        .find_group(group_name)
        .ok_or_else(|| RenderError::GroupNotFound(group_name.to_string()))?;

    pick_option_from_group(group, ctx)
}

/// Pick a weighted random option from a group.
fn pick_option_from_group<R: Rng>(
    group: &PromptGroup,
    ctx: &mut EvalContext<'_, R>,
) -> Result<ChosenOption, RenderError> {
    if group.options.is_empty() {
        return Err(RenderError::EmptyGroup(group.name.clone()));
    }

    let option = weighted_choice(&group.options, &mut ctx.rng);

    Ok(ChosenOption {
        group_name: group.name.clone(),
        option_text: option.text.clone(),
    })
}

/// Weighted random selection from a slice of options.
fn weighted_choice<'a, R: Rng>(options: &'a [PromptOption], rng: &mut R) -> &'a PromptOption {
    let total_weight: f32 = options.iter().map(|o| o.weight).sum();

    if total_weight <= 0.0 {
        // Fallback to uniform selection if weights are invalid
        return &options[rng.random_range(0..options.len())];
    }

    let mut pick = rng.random_range(0.0..total_weight);

    for option in options {
        pick -= option.weight;
        if pick <= 0.0 {
            return option;
        }
    }

    // Fallback (shouldn't happen with valid weights)
    options.last().unwrap()
}

/// Result of evaluating an expression: (output_text, maybe_chosen_option, maybe_assignment).
type ExprResult = (String, Option<ChosenOption>, Option<(String, String)>);

/// Evaluate an expression, returning the output text, any chosen option, and any assignment.
fn eval_expr<R: Rng>(
    expr: &Expr,
    ctx: &mut EvalContext<'_, R>,
) -> Result<ExprResult, RenderError> {
    match expr {
        Expr::Literal(text) => {
            // A literal in an expression block refers to a group name
            let chosen = pick_from_group(text, ctx)?;
            Ok((chosen.option_text.clone(), Some(chosen), None))
        }

        Expr::GroupRef(name) => {
            let chosen = pick_from_group(name, ctx)?;
            Ok((chosen.option_text.clone(), Some(chosen), None))
        }

        Expr::Pipeline(base, ops) => {
            // Evaluate base expression
            let (text, chosen, _) = eval_expr(base, ctx)?;

            // Apply operations to find any assignment
            let assignment = ops.iter().find_map(|op| {
                if let Op::Assign(slot_name) = op {
                    Some((slot_name.clone(), text.clone()))
                } else {
                    None
                }
            });

            Ok((text, chosen, assignment))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_template;

    fn make_test_library() -> Library {
        let mut lib = Library::with_id("test-lib", "Test Library");

        let mut hair = PromptGroup::new("Hair");
        hair.add_text("blonde hair");
        hair.add_text("red hair");
        hair.add_text("black hair");
        lib.groups.push(hair);

        let mut eyes = PromptGroup::new("Eyes");
        eyes.add_text("blue eyes");
        eyes.add_text("green eyes");
        lib.groups.push(eyes);

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
    fn test_render_group_ref() {
        let lib = make_test_library();
        let ast = parse_template("A girl with {Hair}").unwrap();
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
    fn test_render_deterministic_with_seed() {
        let lib = make_test_library();
        let ast = parse_template("{Hair} and {Eyes}").unwrap();
        let template = PromptTemplate::new("test", ast);

        let mut ctx1 = EvalContext::with_seed(&lib, 12345);
        let result1 = render(&template, &mut ctx1).unwrap();

        let mut ctx2 = EvalContext::with_seed(&lib, 12345);
        let result2 = render(&template, &mut ctx2).unwrap();

        assert_eq!(result1.text, result2.text);
    }

    #[test]
    fn test_render_freeform_slot_with_override() {
        let lib = make_test_library();
        let ast = parse_template("Hello {{ Name }}!").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);
        ctx.set_slot("Name", "Alice");

        let result = render(&template, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello Alice!");
    }

    #[test]
    fn test_render_freeform_slot_without_override() {
        let lib = make_test_library();
        let ast = parse_template("Hello {{ Name }}!").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello {{ Name }}!");
    }

    #[test]
    fn test_render_expr_block() {
        let lib = make_test_library();
        let ast = parse_template(r#"[[ "Hair" | some ]]"#).unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(
            result.text == "blonde hair"
                || result.text == "red hair"
                || result.text == "black hair"
        );
        assert_eq!(result.chosen_options.len(), 1);
    }

    #[test]
    fn test_render_expr_block_with_assign() {
        let lib = make_test_library();
        let ast = parse_template(r#"[[ "Hair" | some | assign("hair_choice") ]]"#).unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(result.slot_values.contains_key("hair_choice"));
        assert_eq!(result.slot_values["hair_choice"], result.text);
    }

    #[test]
    fn test_render_comments_not_included() {
        let lib = make_test_library();
        let ast = parse_template("Hello # this is a comment\nWorld").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx).unwrap();
        assert!(!result.text.contains("this is a comment"));
        assert!(result.text.contains("Hello"));
        assert!(result.text.contains("World"));
    }

    #[test]
    fn test_render_group_not_found_error() {
        let lib = make_test_library();
        let ast = parse_template("{NonExistent}").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        assert!(matches!(result, Err(RenderError::GroupNotFound(_))));
    }

    #[test]
    fn test_render_empty_group_error() {
        let mut lib = make_test_library();
        lib.groups.push(PromptGroup::new("Empty"));

        let ast = parse_template("{Empty}").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        assert!(matches!(result, Err(RenderError::EmptyGroup(_))));
    }

    #[test]
    fn test_weighted_selection() {
        let mut lib = Library::with_id("test", "Test");
        let mut group = PromptGroup::new("Weighted");
        group.options.push(PromptOption::with_weight("common", 100.0));
        group.options.push(PromptOption::with_weight("rare", 1.0));
        lib.groups.push(group);

        let ast = parse_template("{Weighted}").unwrap();
        let template = PromptTemplate::new("test", ast);

        let mut common_count = 0;
        for seed in 0..100 {
            let mut ctx = EvalContext::with_seed(&lib, seed);
            let result = render(&template, &mut ctx).unwrap();
            if result.text == "common" {
                common_count += 1;
            }
        }

        // With 100:1 weight ratio, "common" should be selected most of the time
        assert!(common_count > 80, "common was selected {} times", common_count);
    }
}
