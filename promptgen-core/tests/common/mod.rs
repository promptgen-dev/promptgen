//! Shared test utilities for promptgen tests.
//!
//! Provides helper functions for:
//! - Creating small inline test libraries from YAML
//! - Evaluating templates against libraries

#![allow(dead_code)]

use promptgen_core::{
    EvalContext, Library, PromptTemplate, RenderResult, parse_pack, parse_template, render,
};

// ============================================================================
// Inline Library Helpers
// ============================================================================

/// Create a small test library from inline YAML.
///
/// The YAML should contain just groups and optionally templates.
/// The library name and id are auto-generated.
///
/// # Example
/// ```ignore
/// let lib = lib(r#"
/// groups:
///   - tags: [Color]
///     options:
///       - red
///       - blue
/// "#);
/// ```
pub fn lib(yaml: &str) -> Library {
    let full_yaml = format!("id: test\nname: test\n{}", yaml);
    parse_pack(&full_yaml).expect("Test library YAML should be valid")
}

/// Evaluate a template source against a library.
///
/// # Example
/// ```ignore
/// let lib = lib(r#"
/// groups:
///   - tags: [Color]
///     options: [red, blue, green]
/// "#);
/// let result = eval(&lib, "{Color}", None);
/// assert!(["red", "blue", "green"].contains(&result.text.as_str()));
/// ```
pub fn eval(library: &Library, source: &str, seed: Option<u64>) -> RenderResult {
    eval_with_slots(library, source, &[], seed)
}

/// Evaluate a pre-defined template from the library by name.
pub fn eval_template(library: &Library, name: &str, seed: Option<u64>) -> RenderResult {
    let template = library
        .find_template(name)
        .unwrap_or_else(|| panic!("Template '{}' should exist", name));
    let mut ctx = EvalContext::with_seed(library, seed.unwrap_or(42));
    render(template, &mut ctx).expect("Template should render")
}

/// Evaluate a template source against a library with slot overrides.
pub fn eval_with_slots(
    library: &Library,
    source: &str,
    slots: &[(&str, &str)],
    seed: Option<u64>,
) -> RenderResult {
    let ast = parse_template(source).expect("Template should parse");
    let template = PromptTemplate::new("test", ast);
    let mut ctx = EvalContext::with_seed(library, seed.unwrap_or(42));
    for (name, value) in slots {
        ctx.set_slot(*name, *value);
    }
    render(&template, &mut ctx).expect("Template should render")
}
