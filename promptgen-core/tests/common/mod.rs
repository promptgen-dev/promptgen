//! Shared test utilities for promptgen tests.

#![allow(dead_code)]

use promptgen_core::{
    EvalContext, Library, RenderError, RenderResult, Workspace, WorkspaceBuilder, parse_pack,
    parse_template, render,
};

// ============================================================================
// Inline Library Helpers
// ============================================================================

/// Create an empty test library with no variables or templates.
///
/// Useful for tests that don't need library features.
pub fn empty_lib() -> Library {
    lib("variables: []")
}

/// Create a small test library from inline YAML.
///
/// The YAML should contain just variables and optionally templates.
/// The library name and id are auto-generated.
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
    let full_yaml = format!("id: test\nname: test\n{}", yaml);
    parse_pack(&full_yaml).expect("Test library YAML should be valid")
}

/// Create a workspace from a library.
pub fn workspace(library: Library) -> Workspace {
    WorkspaceBuilder::new().add_library(library).build()
}

/// Evaluate a template source against a library.
pub fn eval(library: &Library, source: &str, seed: Option<u64>) -> RenderResult {
    eval_with_slots(library, source, &[], seed)
}

/// Evaluate a pre-defined template from the library by name.
pub fn eval_template(library: &Library, name: &str, seed: Option<u64>) -> RenderResult {
    let template = library
        .find_template(name)
        .unwrap_or_else(|| panic!("Template '{}' should exist", name));
    let ws = workspace(library.clone());
    let mut ctx = EvalContext::with_seed(&ws, seed.unwrap_or(42));
    render(&template.ast, &mut ctx).expect("Template should render")
}

/// Evaluate a template source against a library with slot overrides (single values).
pub fn eval_with_slots(
    library: &Library,
    source: &str,
    slots: &[(&str, &str)],
    seed: Option<u64>,
) -> RenderResult {
    try_eval_with_slots(library, source, slots, seed).expect("Template should render")
}

/// Evaluate a template source against a library with slot overrides (multiple values per slot).
/// Use this for `| many` slots where you need to provide multiple values.
pub fn eval_with_slot_values(
    library: &Library,
    source: &str,
    slots: &[(&str, Vec<&str>)],
    seed: Option<u64>,
) -> RenderResult {
    try_eval_with_slot_values(library, source, slots, seed).expect("Template should render")
}

/// Try to evaluate a template source against a library with slot overrides (single values).
/// Returns Result to allow testing error cases.
pub fn try_eval_with_slots(
    library: &Library,
    source: &str,
    slots: &[(&str, &str)],
    seed: Option<u64>,
) -> Result<RenderResult, RenderError> {
    let ast = parse_template(source).expect("Template should parse");
    let ws = workspace(library.clone());
    let mut ctx = EvalContext::with_seed(&ws, seed.unwrap_or(42));
    for (name, value) in slots {
        ctx.set_slot(*name, (*value).to_string());
    }
    render(&ast, &mut ctx)
}

/// Try to evaluate a template source against a library with slot overrides (multiple values per slot).
/// Returns Result to allow testing error cases.
/// Use this for `| many` slots or to test validation errors on `| one` slots with multiple values.
pub fn try_eval_with_slot_values(
    library: &Library,
    source: &str,
    slots: &[(&str, Vec<&str>)],
    seed: Option<u64>,
) -> Result<RenderResult, RenderError> {
    let ast = parse_template(source).expect("Template should parse");
    let ws = workspace(library.clone());
    let mut ctx = EvalContext::with_seed(&ws, seed.unwrap_or(42));
    for (name, values) in slots {
        ctx.set_slot_values(*name, values.iter().map(|s| s.to_string()).collect());
    }
    render(&ast, &mut ctx)
}
