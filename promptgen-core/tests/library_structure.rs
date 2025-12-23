//! Tests for library structure and loading.
//!
//! Tests that libraries can be loaded from YAML and have correct structure.

mod common;

use common::{empty_lib, lib};

// ============================================================================
// Library Loading Tests
// ============================================================================

#[test]
fn library_loads_variables() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde
      - red
  - name: Eyes
    options:
      - blue
      - green
"#);

    assert_eq!(lib.variables.len(), 2);
    assert!(lib.find_variable("Hair").is_some());
    assert!(lib.find_variable("Eyes").is_some());
}

#[test]
fn library_loads_prompts() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde
prompts:
  - name: Character
    content: "@Hair style"
"#);

    assert_eq!(lib.prompts.len(), 1);
    let prompt = lib.prompts.iter().find(|p| p.name == "Character");
    assert!(prompt.is_some());
    assert_eq!(prompt.unwrap().content, "@Hair style");
}

#[test]
fn variable_options_are_loaded() {
    let lib = lib(r#"
variables:
  - name: Colors
    options:
      - red
      - green
      - blue
"#);

    let variable = lib.find_variable("Colors").unwrap();
    assert_eq!(variable.options.len(), 3);
    assert!(variable.options.contains(&"red".to_string()));
    assert!(variable.options.contains(&"green".to_string()));
    assert!(variable.options.contains(&"blue".to_string()));
}

#[test]
fn empty_library_loads() {
    let lib = empty_lib();

    assert!(lib.variables.is_empty());
    assert!(lib.prompts.is_empty());
}

#[test]
fn variable_with_spaces_in_name() {
    let lib = lib(r#"
variables:
  - name: "Hair Color"
    options:
      - blonde
      - brunette
"#);

    let variable = lib.find_variable("Hair Color");
    assert!(variable.is_some());
    assert_eq!(variable.unwrap().options.len(), 2);
}

#[test]
fn prompt_extracts_library_refs() {
    use promptgen_core::parse_prompt;

    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde
prompts:
  - name: Test
    content: "@Hair and @Eyes"
"#);

    let prompt = lib.prompts.iter().find(|p| p.name == "Test").unwrap();
    let ast = parse_prompt(&prompt.content).unwrap();
    let refs = lib.get_references(&ast);

    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].variable, "Hair");
    assert_eq!(refs[1].variable, "Eyes");
}

#[test]
fn prompt_extracts_slots() {
    use promptgen_core::parse_prompt;

    let lib = lib(r#"
variables: []
prompts:
  - name: Greeting
    content: "Hello {{ Name }}, welcome to {{ Place }}"
"#);

    let prompt = lib.prompts.iter().find(|p| p.name == "Greeting").unwrap();
    let ast = parse_prompt(&prompt.content).unwrap();
    let slots = lib.get_slot_definitions(&ast);

    assert_eq!(slots.len(), 2);
    assert!(slots.iter().any(|s| s.label == "Name"));
    assert!(slots.iter().any(|s| s.label == "Place"));
}

// ============================================================================
// Library Metadata
// ============================================================================

#[test]
fn library_has_metadata() {
    let lib = lib(r#"
variables:
  - name: Test
    options:
      - value
"#);

    // The common::lib helper sets name to "test"
    assert_eq!(lib.name, "test");
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
#[should_panic(expected = "DuplicateVariableName")]
fn duplicate_variable_names_rejected() {
    // This should panic because the common::lib helper uses expect()
    lib(r#"
variables:
  - name: Hair
    options:
      - blonde
  - name: Hair
    options:
      - red
"#);
}
