//! Tests for pre-defined templates from the library.
//!
//! These tests verify that templates defined in libraries render correctly
//! and produce expected results.

mod common;

use common::{eval_template, lib};

// ============================================================================
// Template Rendering Tests
// ============================================================================

#[test]
fn basic_character_template_renders() {
    let lib = lib(r#"
groups:
  - tags: [Quality]
    options: [masterpiece, high quality]
  - tags: [Hair]
    options: [blonde hair, red hair]
  - tags: [Eyes]
    options: [blue eyes, green eyes]
  - tags: [Expression]
    options: [smiling, serious]
templates:
  - name: Basic Character
    source: "{Quality}, {Hair}, {Eyes}, {Expression}"
"#);
    let result = eval_template(&lib, "Basic Character", None);

    // Should have 4 selections (Quality, Hair, Eyes, Expression)
    assert_eq!(result.chosen_options.len(), 4);
    // Should contain commas separating parts
    assert!(result.text.contains(", "));
}

#[test]
fn freeform_scene_template_with_overrides() {
    let lib = lib(r#"
groups:
  - tags: [Quality]
    options: [masterpiece]
  - tags: [Background]
    options: [simple background]
  - tags: [Lighting]
    options: [soft lighting]
templates:
  - name: Freeform Scene
    source: "{Quality}, {{ Subject }}, {{ Action }}, {Background}, {Lighting}"
"#);
    let mut ctx = promptgen_core::EvalContext::with_seed(&lib, 42);
    ctx.set_slot("Subject", "a majestic dragon");
    ctx.set_slot("Action", "breathing fire");

    let template = lib.find_template("Freeform Scene").unwrap();
    let result = promptgen_core::render(template, &mut ctx).unwrap();

    assert!(result.text.contains("a majestic dragon"));
    assert!(result.text.contains("breathing fire"));
}

#[test]
fn eyes_exclusion_template_works() {
    let lib = lib(r#"
groups:
  - tags: [Eyes]
    options: [blue eyes, green eyes]
  - tags: [Eyes, anime]
    options: [sparkling eyes, chibi eyes]
templates:
  - name: Eyes with Exclusion
    source: "{Eyes - anime}"
"#);
    for seed in 0..20 {
        let result = eval_template(&lib, "Eyes with Exclusion", Some(seed));

        // Should never get anime eye options
        assert!(
            !result.text.contains("sparkling") && !result.text.contains("chibi"),
            "Seed {}: Got anime eyes '{}' despite exclusion",
            seed,
            result.text
        );
    }
}

#[test]
fn expression_block_template_records_assignment() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options: [blonde hair, red hair, black hair]
  - tags: [Eyes]
    options: [blue eyes, green eyes]
templates:
  - name: Expression Block Test
    source: |
      [[ "Hair" | some | assign("chosen_hair") ]], {Eyes}
"#);
    let result = eval_template(&lib, "Expression Block Test", None);

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
    let lib = lib(r#"
groups: []
templates:
  - name: Freeform Scene
    source: "{{ Subject }} doing {{ Action }}"
"#);
    let template = lib.find_template("Freeform Scene").unwrap();

    let slots = template.slots();

    // Should have Subject and Action as freeform slots
    let slot_names: Vec<_> = slots.iter().map(|s| s.name.as_str()).collect();
    assert!(slot_names.contains(&"Subject"));
    assert!(slot_names.contains(&"Action"));
}

#[test]
fn template_referenced_tags_extracted() {
    let lib = lib(r#"
groups: []
templates:
  - name: Basic Character
    source: "{Quality}, {Hair}, {Eyes}, {Expression}"
"#);
    let template = lib.find_template("Basic Character").unwrap();

    let tags = template.referenced_tags();

    assert!(tags.contains(&"Quality".to_string()));
    assert!(tags.contains(&"Hair".to_string()));
    assert!(tags.contains(&"Eyes".to_string()));
    assert!(tags.contains(&"Expression".to_string()));
}

#[test]
fn exclusion_template_shows_excluded_tags() {
    let lib = lib(r#"
groups: []
templates:
  - name: Eyes with Exclusion
    source: "{Eyes - anime}"
"#);
    let template = lib.find_template("Eyes with Exclusion").unwrap();

    let tags = template.referenced_tags();

    // Should include both the main tag and excluded tags
    assert!(tags.contains(&"Eyes".to_string()));
    assert!(tags.contains(&"anime".to_string()));
}
