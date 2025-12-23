//! Prompt evaluation module.
//!
//! Evaluates prompts against a Library to produce resolved prompts.
//!
//! Key features:
//! - Variable resolution (finds variables by name)
//! - InlineOptions evaluation (random selection from {a|b|c})
//! - Lazy parsing of option text for nested grammar
//! - Cycle detection for circular references

use std::collections::HashMap;

use rand::prelude::*;

use crate::ast::{LibraryRef, Node, OptionItem, PickOperator, PickSlot, Prompt, SlotKind};
use crate::library::Library;
use crate::parser::parse_prompt;

/// Context for evaluating a prompt against a library.
pub struct EvalContext<'a, R: Rng = StdRng> {
    /// The library containing variables.
    pub library: &'a Library,
    /// Random number generator for selecting options.
    pub rng: R,
    /// Overrides for slots (slot name -> list of values).
    /// For `| one` slots, provide a single-element vec.
    /// For `| many` slots, provide multiple values.
    pub slot_overrides: HashMap<String, Vec<String>>,
    /// Stack of variable names being evaluated (for cycle detection).
    eval_stack: Vec<String>,
}

impl<'a> EvalContext<'a, StdRng> {
    /// Create a new context with the given library and OS random.
    /// Note: This will not work in WASM environments. Use `with_seed` instead.
    #[cfg(not(target_arch = "wasm32"))]
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

    /// Add a slot override with a single value.
    /// For `| one` slots or textarea slots.
    pub fn set_slot(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.slot_overrides.insert(name.into(), vec![value.into()]);
    }

    /// Add a slot override with multiple values.
    /// For `| many` slots.
    pub fn set_slot_values(&mut self, name: impl Into<String>, values: Vec<String>) {
        self.slot_overrides.insert(name.into(), values);
    }

    /// Add multiple slot overrides (single values each).
    pub fn set_slots(&mut self, overrides: impl IntoIterator<Item = (String, String)>) {
        for (name, value) in overrides {
            self.slot_overrides.insert(name, vec![value]);
        }
    }
}

/// Record of which option was chosen from a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenOption {
    /// The variable name that was referenced.
    pub variable_name: String,
    /// The text of the option that was selected.
    pub option_text: String,
    /// The index of the option in the variable.
    pub option_index: usize,
}

/// Result of rendering a prompt.
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// The final rendered prompt text.
    pub text: String,
    /// Options that were chosen during rendering (for provenance/reproducibility).
    pub chosen_options: Vec<ChosenOption>,
    /// Slot values that were used (slot name -> list of values).
    pub slot_values: HashMap<String, Vec<String>>,
}

/// Error that can occur during rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("variable not found: {0}")]
    VariableNotFound(String),

    #[error("variable has no options: {0}")]
    EmptyVariable(String),

    #[error("circular reference detected: {0}")]
    CircularReference(String),

    #[error("parse error in option text: {0}")]
    OptionParseError(String),

    #[error("slot '{slot}' expects exactly one value, but got {count}")]
    TooManyValuesForOne { slot: String, count: usize },

    #[error("slot '{slot}' allows at most {max} values, but got {count}")]
    TooManyValuesForMany {
        slot: String,
        max: u32,
        count: usize,
    },

    #[error("Slots may not reference other slots: {0}")]
    SlotReferencesSlot(String),
}

/// Render a parsed prompt AST using the given context.
pub fn render<R: Rng>(
    ast: &Prompt,
    ctx: &mut EvalContext<'_, R>,
) -> Result<RenderResult, RenderError> {
    let mut output = String::new();
    let mut chosen_options = Vec::new();
    let slot_values = ctx.slot_overrides.clone();

    for (node, _span) in &ast.nodes {
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

        Node::SlotBlock(slot_block) => {
            let slot_name = &slot_block.label.0;

            match &slot_block.kind.0 {
                SlotKind::Textarea => {
                    // Textarea slot: check for override, otherwise return empty string
                    if let Some(values) = ctx.slot_overrides.get(slot_name).cloned() {
                        // For textarea, join all values (typically just one)
                        // Each value can contain grammar - parse and evaluate
                        let mut result = String::new();
                        for (i, value) in values.iter().enumerate() {
                            if i > 0 {
                                result.push_str(", ");
                            }
                            let evaluated = eval_text_with_grammar(value, ctx, chosen_options)?;
                            result.push_str(&evaluated);
                        }
                        Ok(result)
                    } else {
                        // No value provided - render as empty string per spec
                        Ok(String::new())
                    }
                }
                SlotKind::Pick(pick) => {
                    // Pick slot: check for override first
                    if let Some(values) = ctx.slot_overrides.get(slot_name).cloned() {
                        // Validate and render the pick slot values
                        eval_pick_slot_value(slot_name, &values, pick, ctx, chosen_options)
                    } else {
                        // No value provided - render as empty string per spec
                        Ok(String::new())
                    }
                }
            }
        }

        Node::LibraryRef(lib_ref) => {
            let (text, chosen) = resolve_library_ref(lib_ref, ctx, chosen_options)?;
            chosen_options.push(chosen);
            Ok(text)
        }

        Node::InlineOptions(options) => eval_inline_options(options, ctx, chosen_options),
    }
}

/// Parse and evaluate text that may contain grammar.
/// Slot values may not contain slot blocks (would cause infinite recursion).
fn eval_text_with_grammar<R: Rng>(
    text: &str,
    ctx: &mut EvalContext<'_, R>,
    chosen_options: &mut Vec<ChosenOption>,
) -> Result<String, RenderError> {
    let ast = parse_prompt(text).map_err(|e| RenderError::OptionParseError(e.to_string()))?;

    // Check for slot blocks in the parsed AST - slots may not reference other slots
    for (node, _span) in &ast.nodes {
        if let Node::SlotBlock(slot_block) = node {
            return Err(RenderError::SlotReferencesSlot(slot_block.label.0.clone()));
        }
    }

    let mut output = String::new();
    for (node, _span) in &ast.nodes {
        let result = eval_node(node, ctx, chosen_options)?;
        output.push_str(&result);
    }

    Ok(output)
}

/// Evaluate a pick slot value with validation based on operators.
///
/// Validates the values array against the `one` or `many(max=N)` constraints,
/// evaluates any grammar in each value, and joins the results with the
/// appropriate separator.
fn eval_pick_slot_value<R: Rng>(
    slot_name: &str,
    values: &[String],
    pick: &PickSlot,
    ctx: &mut EvalContext<'_, R>,
    chosen_options: &mut Vec<ChosenOption>,
) -> Result<String, RenderError> {
    // Determine cardinality and separator from operators
    let (is_one, max, separator) = extract_pick_constraints(pick);

    let count = values.len();

    // Validate count constraints
    if is_one && count > 1 {
        return Err(RenderError::TooManyValuesForOne {
            slot: slot_name.to_string(),
            count,
        });
    }

    if let Some(max_val) = max
        && count > max_val as usize
    {
        return Err(RenderError::TooManyValuesForMany {
            slot: slot_name.to_string(),
            max: max_val,
            count,
        });
    }

    // Evaluate each value (may contain grammar like @Color or {a|b})
    let mut evaluated: Vec<String> = Vec::with_capacity(count);
    for value in values {
        let result = eval_text_with_grammar(value, ctx, chosen_options)?;
        evaluated.push(result);
    }

    // Join with the appropriate separator
    Ok(evaluated.join(&separator))
}

/// Extract cardinality constraints and separator from pick operators.
/// Returns (is_one, max_for_many, separator)
fn extract_pick_constraints(pick: &PickSlot) -> (bool, Option<u32>, String) {
    let mut is_one = false;
    let mut max: Option<u32> = None;
    let mut separator = ", ".to_string(); // Default separator

    for (op, _span) in &pick.operators {
        match op {
            PickOperator::One => {
                is_one = true;
            }
            PickOperator::Many(spec) => {
                max = spec.max;
                if let Some(sep) = &spec.sep {
                    separator = sep.clone();
                }
            }
        }
    }

    (is_one, max, separator)
}

/// Resolve a library reference to a random option.
fn resolve_library_ref<R: Rng>(
    lib_ref: &LibraryRef,
    ctx: &mut EvalContext<'_, R>,
    chosen_options: &mut Vec<ChosenOption>,
) -> Result<(String, ChosenOption), RenderError> {
    let variable_name = &lib_ref.variable;

    // Check for circular reference
    if ctx.eval_stack.contains(variable_name) {
        let chain = ctx.eval_stack.join(" -> ");
        return Err(RenderError::CircularReference(format!(
            "{} -> {}",
            chain, variable_name
        )));
    }

    // Find the variable in the library (ignore any library qualifier in single-library mode)
    let variable = ctx
        .library
        .find_variable(variable_name)
        .ok_or_else(|| RenderError::VariableNotFound(variable_name.clone()))?;

    if variable.options.is_empty() {
        return Err(RenderError::EmptyVariable(variable_name.clone()));
    }

    // Pick a random option
    let idx = ctx.rng.random_range(0..variable.options.len());
    let option_text = &variable.options[idx];

    // Push to eval stack for cycle detection
    ctx.eval_stack.push(variable_name.clone());

    // Parse and evaluate the option (lazy evaluation for nested grammar)
    let evaluated_text = eval_text_with_grammar(option_text, ctx, chosen_options)?;

    // Pop from eval stack
    ctx.eval_stack.pop();

    let chosen = ChosenOption {
        variable_name: variable_name.clone(),
        option_text: evaluated_text.clone(),
        option_index: idx,
    };

    Ok((evaluated_text, chosen))
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
            eval_text_with_grammar(text, ctx, chosen_options)
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
    use crate::library::PromptVariable;

    fn make_test_library() -> Library {
        let mut lib = Library::new("Test Library");

        lib.variables.push(PromptVariable::with_options(
            "Hair",
            vec!["blonde hair", "red hair", "black hair"],
        ));

        lib.variables.push(PromptVariable::with_options(
            "Eyes",
            vec!["blue eyes", "green eyes"],
        ));

        lib.variables.push(PromptVariable::with_options(
            "Color",
            vec!["red", "blue", "green"],
        ));

        lib
    }

    #[test]
    fn test_render_plain_text() {
        let lib = make_test_library();
        let ast = parse_prompt("Hello, world!").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello, world!");
        assert!(result.chosen_options.is_empty());
    }

    #[test]
    fn test_render_library_ref() {
        let lib = make_test_library();
        let ast = parse_prompt("A girl with @Hair").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        assert!(result.text.starts_with("A girl with "));
        assert!(
            result.text.contains("blonde hair")
                || result.text.contains("red hair")
                || result.text.contains("black hair")
        );
        assert_eq!(result.chosen_options.len(), 1);
        assert_eq!(result.chosen_options[0].variable_name, "Hair");
    }

    #[test]
    fn test_render_quoted_library_ref() {
        let mut lib = Library::new("Test");
        lib.variables.push(PromptVariable::with_options(
            "Eye Color",
            vec!["amber", "violet"],
        ));

        let ast = parse_prompt(r#"@"Eye Color""#).unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        assert!(result.text == "amber" || result.text == "violet");
    }

    #[test]
    fn test_render_deterministic_with_seed() {
        let lib = make_test_library();
        let ast = parse_prompt("@Hair and @Eyes").unwrap();

        let mut ctx1 = EvalContext::with_seed(&lib, 12345);
        let result1 = render(&ast, &mut ctx1).unwrap();

        let mut ctx2 = EvalContext::with_seed(&lib, 12345);
        let result2 = render(&ast, &mut ctx2).unwrap();

        assert_eq!(result1.text, result2.text);
    }

    #[test]
    fn test_render_inline_options() {
        let lib = make_test_library();
        let ast = parse_prompt("{hot|cold} weather").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        assert!(result.text == "hot weather" || result.text == "cold weather");
    }

    #[test]
    fn test_render_slot_with_override() {
        let lib = make_test_library();
        let ast = parse_prompt("Hello {{ Name }}!").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);
        ctx.set_slot("Name", "Alice");

        let result = render(&ast, &mut ctx).unwrap();
        assert_eq!(result.text, "Hello Alice!");
    }

    #[test]
    fn test_render_slot_without_override() {
        let lib = make_test_library();
        let ast = parse_prompt("Hello {{ Name }}!").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        // Empty slots render to empty string per spec
        assert_eq!(result.text, "Hello !");
    }

    #[test]
    fn test_render_slot_with_grammar() {
        let lib = make_test_library();
        let ast = parse_prompt("A hero: {{ character }}").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);
        ctx.set_slot("character", "@Hair warrior");

        let result = render(&ast, &mut ctx).unwrap();
        assert!(result.text.starts_with("A hero: "));
        assert!(result.text.contains("hair warrior"));
    }

    #[test]
    fn test_render_comments_not_included() {
        let lib = make_test_library();
        let ast = parse_prompt("Hello # this is a comment\nWorld").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        assert!(!result.text.contains("this is a comment"));
        assert!(!result.text.contains('#'));
    }

    #[test]
    fn test_render_variable_not_found_error() {
        let lib = make_test_library();
        let ast = parse_prompt("@NonExistent").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx);
        assert!(matches!(result, Err(RenderError::VariableNotFound(_))));
    }

    #[test]
    fn test_render_empty_variable_error() {
        let mut lib = Library::new("Test");
        lib.variables.push(PromptVariable::new("Empty", vec![]));

        let ast = parse_prompt("@Empty").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx);
        assert!(matches!(result, Err(RenderError::EmptyVariable(_))));
    }

    #[test]
    fn test_render_nested_grammar_in_options() {
        let mut lib = Library::new("Test");
        lib.variables.push(PromptVariable::with_options(
            "Color",
            vec!["red", "blue", "green"],
        ));
        lib.variables.push(PromptVariable::with_options(
            "FancyEyes",
            vec!["@Color eyes", "sparkling eyes"],
        ));

        let ast = parse_prompt("@FancyEyes").unwrap();

        // Test multiple times to cover both options
        let mut found_color_eyes = false;
        let mut found_sparkling = false;

        for seed in 0..50 {
            let mut ctx = EvalContext::with_seed(&lib, seed);
            let result = render(&ast, &mut ctx).unwrap();

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
        lib.variables
            .push(PromptVariable::with_options("A", vec!["@B"]));
        lib.variables
            .push(PromptVariable::with_options("B", vec!["@A"]));

        let ast = parse_prompt("@A").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx);
        assert!(matches!(result, Err(RenderError::CircularReference(_))));
    }

    #[test]
    fn test_render_mixed_prompt() {
        let lib = make_test_library();
        let ast = parse_prompt("A {big|small} creature with @Hair and @Eyes").unwrap();
        let mut ctx = EvalContext::with_seed(&lib, 42);

        let result = render(&ast, &mut ctx).unwrap();
        assert!(result.text.contains("creature with"));
        assert!(result.text.contains(" and "));
        // Should have 2 chosen options (Hair and Eyes)
        assert_eq!(result.chosen_options.len(), 2);
    }
}
