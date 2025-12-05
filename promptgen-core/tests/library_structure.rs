//! Tests for library structure and loading.
//!
//! Verifies that the kitchen sink library loads correctly with all expected
//! groups, templates, tags, and weighted options.

mod common;

use common::load_test_library;

// ============================================================================
// Library Loading Tests
// ============================================================================

#[test]
fn library_loads_all_groups() {
    let lib = load_test_library();
    assert_eq!(lib.groups.len(), 12, "Should have 12 groups");
}

#[test]
fn library_loads_all_templates() {
    let lib = load_test_library();
    assert_eq!(lib.templates.len(), 5, "Should have 5 templates");
}

// ============================================================================
// Group Tag Tests
// ============================================================================

#[test]
fn groups_have_correct_tags() {
    let lib = load_test_library();

    // Hair group should have multiple tags
    let hair = lib.find_group("Hair").expect("Hair group should exist");
    assert!(hair.tags.contains(&"Hair".to_string()));
    assert!(hair.tags.contains(&"hair-color".to_string()));
    assert!(hair.tags.contains(&"appearance".to_string()));
}

#[test]
fn groups_findable_by_any_tag() {
    let lib = load_test_library();

    // Should find same group via different tags
    let by_hair = lib.find_group("Hair");
    let by_hair_color = lib.find_group("hair-color");
    let by_appearance = lib.find_group("appearance");
    let by_hair_color_pretty = lib.find_group("Hair Color");

    assert!(by_hair.is_some());
    assert!(by_hair_color.is_some());
    // appearance might match multiple groups, just check it finds something
    assert!(by_appearance.is_some());
    assert!(by_hair_color_pretty.is_some());
}

// ============================================================================
// Weighted Options Tests
// ============================================================================

#[test]
fn weighted_options_loaded_correctly() {
    let lib = load_test_library();

    let style = lib.find_group("Style").expect("Style group should exist");
    let photorealistic = style.options.iter().find(|o| o.text == "photorealistic");
    let oil_painting = style.options.iter().find(|o| o.text == "oil painting");

    assert_eq!(photorealistic.unwrap().weight, 5.0);
    assert_eq!(oil_painting.unwrap().weight, 1.0);
}
