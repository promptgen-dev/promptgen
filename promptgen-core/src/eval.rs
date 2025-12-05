//! Template evaluation module.
//!
//! Evaluates templates against a library to produce resolved prompts.

use std::collections::HashMap;

use rand::prelude::*;

use crate::ast::{Expr, Node, Op, TagQuery};
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

/// Record of which option was chosen for a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenOption {
    /// The query that was evaluated.
    pub query: TagQuery,
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
    /// Slot values that were used (both overrides and assigned).
    pub slot_values: HashMap<String, String>,
}

/// Error that can occur during rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("group not found: {0}")]
    GroupNotFound(String),

    #[error("tag not found: {0}")]
    TagNotFound(String),

    #[error("group has no options: {0}")]
    EmptyGroup(String),

    #[error("query matched no options: {0:?}")]
    EmptyQueryResult(TagQuery),
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

            Node::TagQuery(query) => {
                let chosen = pick_from_query(query, ctx)?;
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

/// Pick a random option from groups matching a tag query.
///
/// This collects all options from groups that have ANY of the include tags,
/// then removes options from groups that have ANY of the exclude tags.
fn pick_from_query<R: Rng>(
    query: &TagQuery,
    ctx: &mut EvalContext<'_, R>,
) -> Result<ChosenOption, RenderError> {
    // Collect all matching groups
    let matching_groups: Vec<&PromptGroup> = ctx
        .library
        .groups
        .iter()
        .filter(|group| {
            // Include if group has ANY of the include tags
            let has_include = query.include.iter().any(|tag| group.tags.contains(tag));

            // Exclude if group has ANY of the exclude tags
            let has_exclude = query.exclude.iter().any(|tag| group.tags.contains(tag));

            has_include && !has_exclude
        })
        .collect();

    if matching_groups.is_empty() {
        return Err(RenderError::EmptyQueryResult(query.clone()));
    }

    // Collect all options from matching groups
    let all_options: Vec<&PromptOption> = matching_groups
        .iter()
        .flat_map(|group| group.options.iter())
        .collect();

    if all_options.is_empty() {
        return Err(RenderError::EmptyQueryResult(query.clone()));
    }

    // Uniform random selection
    let idx = ctx.rng.random_range(0..all_options.len());
    let option = all_options[idx];

    Ok(ChosenOption {
        query: query.clone(),
        option_text: option.text.clone(),
    })
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
            // A literal in an expression block is interpreted as a tag query
            // For now, treat the literal as a single include tag (backwards compatible)
            let query = TagQuery::new(text.clone());
            let chosen = pick_from_query(&query, ctx)?;
            Ok((chosen.option_text.clone(), Some(chosen), None))
        }

        Expr::Query(query) => {
            let chosen = pick_from_query(query, ctx)?;
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

        lib.groups.push(PromptGroup::new(
            vec!["Hair".to_string()],
            vec![
                PromptOption::new("blonde hair"),
                PromptOption::new("red hair"),
                PromptOption::new("black hair"),
            ],
        ));

        lib.groups.push(PromptGroup::new(
            vec!["Eyes".to_string()],
            vec![
                PromptOption::new("blue eyes"),
                PromptOption::new("green eyes"),
            ],
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
        assert_eq!(result.chosen_options[0].query.include, vec!["Hair"]);
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
    fn test_render_tag_not_found_error() {
        let lib = make_test_library();
        let ast = parse_template("{NonExistent}").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        // When no groups match the tag query, we get EmptyQueryResult
        assert!(matches!(result, Err(RenderError::EmptyQueryResult(_))));
    }

    #[test]
    fn test_render_empty_group_error() {
        let mut lib = make_test_library();
        lib.groups.push(PromptGroup::new(
            vec!["Empty".to_string(), "empty-tag".to_string()],
            vec![],
        ));

        let ast = parse_template("{empty-tag}").unwrap();
        let template = PromptTemplate::new("test", ast);
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&template, &mut ctx);
        // When groups match but have no options, we get EmptyQueryResult
        assert!(matches!(result, Err(RenderError::EmptyQueryResult(_))));
    }

}
