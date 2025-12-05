//! Tests for library structure and loading.
//!
//! Verifies that libraries load correctly with expected groups, templates,
//! and tag configurations.

mod common;

use common::lib;

// ============================================================================
// Library Loading Tests
// ============================================================================

#[test]
fn library_loads_groups() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options: [blonde, red]
  - tags: [Eyes]
    options: [blue, green]
  - tags: [Outfit]
    options: [casual, formal]
"#);
    assert_eq!(lib.groups.len(), 3, "Should have 3 groups");
}

#[test]
fn library_loads_templates() {
    let lib = lib(r#"
groups: []
templates:
  - name: Template One
    source: "hello"
  - name: Template Two
    source: "world"
"#);
    assert_eq!(lib.templates.len(), 2, "Should have 2 templates");
}

// ============================================================================
// Group Tag Tests
// ============================================================================

#[test]
fn groups_have_correct_tags() {
    let lib = lib(r#"
groups:
  - tags: [Hair, hair-color, appearance, Hair Color]
    options: [blonde hair, red hair]
"#);

    // Hair group should have multiple tags
    let hair = lib.find_group("Hair").expect("Hair group should exist");
    assert!(hair.tags.contains(&"Hair".to_string()));
    assert!(hair.tags.contains(&"hair-color".to_string()));
    assert!(hair.tags.contains(&"appearance".to_string()));
    assert!(hair.tags.contains(&"Hair Color".to_string()));
}

#[test]
fn groups_findable_by_any_tag() {
    let lib = lib(r#"
groups:
  - tags: [Hair, hair-color, appearance, Hair Color]
    options: [blonde hair, red hair]
"#);

    // Should find same group via different tags
    let by_hair = lib.find_group("Hair");
    let by_hair_color = lib.find_group("hair-color");
    let by_appearance = lib.find_group("appearance");
    let by_hair_color_pretty = lib.find_group("Hair Color");

    assert!(by_hair.is_some());
    assert!(by_hair_color.is_some());
    assert!(by_appearance.is_some());
    assert!(by_hair_color_pretty.is_some());
}

#[test]
fn find_groups_by_tag_returns_all_matching() {
    let lib = lib(r#"
groups:
  - tags: [Eyes]
    options: [blue eyes, green eyes]
  - tags: [Eyes, anime]
    options: [sparkling eyes]
  - tags: [Eyes, realistic]
    options: [detailed iris]
  - tags: [Hair]
    options: [blonde]
"#);

    let eye_groups = lib.find_groups_by_tag("Eyes");
    assert_eq!(eye_groups.len(), 3, "Should find all 3 Eyes groups");

    let anime_groups = lib.find_groups_by_tag("anime");
    assert_eq!(anime_groups.len(), 1, "Should find 1 anime group");
}

#[test]
fn library_metadata_preserved() {
    // Test that library id/name/description work
    let yaml = r#"
id: my-custom-id
name: My Custom Library
description: A test library
groups: []
"#;
    let lib = promptgen_core::parse_pack(yaml).unwrap();

    assert_eq!(lib.id, "my-custom-id");
    assert_eq!(lib.name, "My Custom Library");
    assert_eq!(lib.description, "A test library");
}
