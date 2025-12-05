//! Tests for pre-defined templates from the library.
//!
//! These tests verify that templates defined in the kitchen sink library
//! render correctly and produce expected results.

mod common;

use common::eval_template;

// ============================================================================
// Template Rendering Tests
// ============================================================================

#[test]
fn basic_character_template_renders() {
    let result = eval_template("Basic Character", &[], None);

    // Should have 4 selections (Quality, Hair, Eyes, Expression)
    assert_eq!(result.chosen_options.len(), 4);
    // Should contain commas separating parts
    assert!(result.text.contains(", "));
}

#[test]
fn freeform_scene_template_with_overrides() {
    let result = eval_template(
        "Freeform Scene",
        &[
            ("Subject", "a majestic dragon"),
            ("Action", "breathing fire"),
        ],
        None,
    );

    assert!(result.text.contains("a majestic dragon"));
    assert!(result.text.contains("breathing fire"));
}

#[test]
fn eyes_exclusion_template_works() {
    for seed in 0..20 {
        let result = eval_template("Eyes with Exclusion", &[], Some(seed));

        // Should never get anime eye options
        assert!(
            !result.text.contains("sparkling") && !result.text.contains("chibi"),
            "Got anime eyes despite exclusion"
        );
    }
}

#[test]
fn expression_block_template_records_assignment() {
    let result = eval_template("Expression Block Test", &[], None);

    // Should have recorded the hair choice
    assert!(result.slot_values.contains_key("chosen_hair"));
    // Output should contain the chosen hair and eyes
    assert!(result.text.contains(", "));
}

// ============================================================================
// Template Introspection Tests
// ============================================================================

#[test]
fn template_slots_extracted_correctly() {
    let lib = common::load_test_library();
    let template = lib.find_template("Freeform Scene").unwrap();

    let slots = template.slots();

    // Should have Subject and Action as freeform slots
    let slot_names: Vec<_> = slots.iter().map(|s| s.name.as_str()).collect();
    assert!(slot_names.contains(&"Subject"));
    assert!(slot_names.contains(&"Action"));
}

#[test]
fn template_referenced_tags_extracted() {
    let lib = common::load_test_library();
    let template = lib.find_template("Basic Character").unwrap();

    let tags = template.referenced_tags();

    assert!(tags.contains(&"Quality".to_string()));
    assert!(tags.contains(&"Hair".to_string()));
    assert!(tags.contains(&"Eyes".to_string()));
    assert!(tags.contains(&"Expression".to_string()));
}

#[test]
fn exclusion_template_shows_excluded_tags() {
    let lib = common::load_test_library();
    let template = lib.find_template("Eyes with Exclusion").unwrap();

    let tags = template.referenced_tags();

    // Should include both the main tag and excluded tags
    assert!(tags.contains(&"Eyes".to_string()));
    assert!(tags.contains(&"anime".to_string()));
}
