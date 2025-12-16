//! Tests for library structure and loading.
//!
//! Tests that libraries can be loaded from YAML and have correct structure.

mod common;

use common::{empty_lib, lib};

// ============================================================================
// Library Loading Tests
// ============================================================================

#[test]
fn library_loads_groups() {
    let lib = lib(r#"
groups:
  - name: Hair
    options:
      - blonde
      - red
  - name: Eyes
    options:
      - blue
      - green
"#);

    assert_eq!(lib.groups.len(), 2);
    assert!(lib.find_group("Hair").is_some());
    assert!(lib.find_group("Eyes").is_some());
}

#[test]
fn library_loads_templates() {
    let lib = lib(r#"
groups:
  - name: Hair
    options:
      - blonde
templates:
  - name: Character
    description: A character template
    source: "@Hair style"
"#);

    assert_eq!(lib.templates.len(), 1);
    let tmpl = lib.find_template("Character");
    assert!(tmpl.is_some());
    assert_eq!(tmpl.unwrap().description, "A character template");
}

#[test]
fn group_options_are_loaded() {
    let lib = lib(r#"
groups:
  - name: Colors
    options:
      - red
      - green
      - blue
"#);

    let group = lib.find_group("Colors").unwrap();
    assert_eq!(group.options.len(), 3);
    assert!(group.options.contains(&"red".to_string()));
    assert!(group.options.contains(&"green".to_string()));
    assert!(group.options.contains(&"blue".to_string()));
}

#[test]
fn empty_library_loads() {
    let lib = empty_lib();

    assert!(lib.groups.is_empty());
    assert!(lib.templates.is_empty());
}

#[test]
fn group_with_spaces_in_name() {
    let lib = lib(r#"
groups:
  - name: "Hair Color"
    options:
      - blonde
      - brunette
"#);

    let group = lib.find_group("Hair Color");
    assert!(group.is_some());
    assert_eq!(group.unwrap().options.len(), 2);
}

#[test]
fn template_extracts_library_refs() {
    let lib = lib(r#"
groups:
  - name: Hair
    options:
      - blonde
templates:
  - name: Test
    source: "@Hair and @Eyes"
"#);

    let tmpl = lib.find_template("Test").unwrap();
    let refs = tmpl.referenced_groups();

    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].group, "Hair");
    assert_eq!(refs[1].group, "Eyes");
}

#[test]
fn template_extracts_slots() {
    let lib = lib(r#"
groups: []
templates:
  - name: Greeting
    source: "Hello {{ Name }}, welcome to {{ Place }}"
"#);

    let tmpl = lib.find_template("Greeting").unwrap();
    let slots = tmpl.slots();

    assert_eq!(slots.len(), 2);
    assert!(slots.iter().any(|s| s.name == "Name"));
    assert!(slots.iter().any(|s| s.name == "Place"));
}

// ============================================================================
// Library Metadata
// ============================================================================

#[test]
fn library_has_metadata() {
    let lib = lib(r#"
groups:
  - name: Test
    options:
      - value
"#);

    // The common::lib helper sets id and name to "test"
    assert_eq!(lib.id, "test");
    assert_eq!(lib.name, "test");
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
#[should_panic(expected = "DuplicateGroupName")]
fn duplicate_group_names_rejected() {
    // This should panic because the common::lib helper uses expect()
    lib(r#"
groups:
  - name: Hair
    options:
      - blonde
  - name: Hair
    options:
      - red
"#);
}
