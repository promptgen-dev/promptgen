//! Shared test utilities for promptgen tests.

#![allow(dead_code)]

use promptgen_core::{
    EvalContext, Library, RenderResult, Workspace, WorkspaceBuilder, parse_pack, parse_template,
    render,
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

/// Evaluate a template source against a library with slot overrides.
pub fn eval_with_slots(
    library: &Library,
    source: &str,
    slots: &[(&str, &str)],
    seed: Option<u64>,
) -> RenderResult {
    let ast = parse_template(source).expect("Template should parse");
    let ws = workspace(library.clone());
    let mut ctx = EvalContext::with_seed(&ws, seed.unwrap_or(42));
    for (name, value) in slots {
        ctx.set_slot(*name, (*value).to_string());
    }
    render(&ast, &mut ctx).expect("Template should render")
}
