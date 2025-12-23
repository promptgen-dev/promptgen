//! Shared test utilities for promptgen tests.

#![allow(dead_code)]

use promptgen_core::{
    EvalContext, Library, RenderError, RenderResult, parse_library, parse_prompt, render,
};

// ============================================================================
// Inline Library Helpers
// ============================================================================

/// Create an empty test library with no variables or prompts.
///
/// Useful for tests that don't need library features.
pub fn empty_lib() -> Library {
    lib("variables: []")
}

/// Create a small test library from inline YAML.
///
/// The YAML should contain just variables and optionally prompts.
/// The library name is auto-generated.
///
/// # Example
/// ```ignore
/// let lib = lib(r#"
/// variables:
///   - name: Color
///     options:
///       - red
///       - blue
/// "#);
/// ```
pub fn lib(yaml: &str) -> Library {
    let full_yaml = format!("name: test\n{}", yaml);
    parse_library(&full_yaml).expect("Test library YAML should be valid")
}

/// Evaluate a prompt source against a library.
pub fn eval(library: &Library, source: &str, seed: Option<u64>) -> RenderResult {
    eval_with_slots(library, source, &[], seed)
}

/// Evaluate a saved prompt from the library by name.
pub fn eval_prompt(library: &Library, name: &str, seed: Option<u64>) -> RenderResult {
    let prompt = library
        .prompts
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("Prompt '{}' should exist", name));
    let ast = parse_prompt(&prompt.content).expect("Prompt content should parse");
    let mut ctx = EvalContext::with_seed(library, seed.unwrap_or(42));
    render(&ast, &mut ctx).expect("Prompt should render")
}

/// Evaluate a prompt source against a library with slot overrides (single values).
pub fn eval_with_slots(
    library: &Library,
    source: &str,
    slots: &[(&str, &str)],
    seed: Option<u64>,
) -> RenderResult {
    try_eval_with_slots(library, source, slots, seed).expect("prompt should render")
}

/// Evaluate a prompt source against a library with slot overrides (multiple values per slot).
/// Use this for `| many` slots where you need to provide multiple values.
pub fn eval_with_slot_values(
    library: &Library,
    source: &str,
    slots: &[(&str, Vec<&str>)],
    seed: Option<u64>,
) -> RenderResult {
    try_eval_with_slot_values(library, source, slots, seed).expect("prompt should render")
}

/// Try to evaluate a prompt source against a library with slot overrides (single values).
/// Returns Result to allow testing error cases.
pub fn try_eval_with_slots(
    library: &Library,
    source: &str,
    slots: &[(&str, &str)],
    seed: Option<u64>,
) -> Result<RenderResult, RenderError> {
    let ast = parse_prompt(source).expect("prompt should parse");
    let mut ctx = EvalContext::with_seed(library, seed.unwrap_or(42));
    for (name, value) in slots {
        ctx.set_slot(*name, (*value).to_string());
    }
    render(&ast, &mut ctx)
}

/// Try to evaluate a prompt source against a library with slot overrides (multiple values per slot).
/// Returns Result to allow testing error cases.
/// Use this for `| many` slots or to test validation errors on `| one` slots with multiple values.
pub fn try_eval_with_slot_values(
    library: &Library,
    source: &str,
    slots: &[(&str, Vec<&str>)],
    seed: Option<u64>,
) -> Result<RenderResult, RenderError> {
    let ast = parse_prompt(source).expect("prompt should parse");
    let mut ctx = EvalContext::with_seed(library, seed.unwrap_or(42));
    for (name, values) in slots {
        ctx.set_slot_values(*name, values.iter().map(|s| s.to_string()).collect());
    }
    render(&ast, &mut ctx)
}
