//! Grammar and syntax tests for the template language.
//!
//! Tests all the different syntactic constructs:
//! - Tag queries: `{Tag}`, `{Tag - exclude}`
//! - Freeform slots: `{{ SlotName }}`
//! - Expression blocks: `[[ "Tag" | op ]]`
//! - Comments: `# comment`

mod common;

use common::{eval, eval_with_slots, lib};

// ============================================================================
// Basic Tag Query Tests: {Tag}
// ============================================================================

#[test]
fn simple_tag_query_renders() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options:
      - blonde hair
      - red hair
      - black hair
      - brown hair
"#);
    let result = eval(&lib, "{Hair}", None);

    let valid_options = ["blonde hair", "red hair", "black hair", "brown hair"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn multiple_tag_queries_render() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options: [blonde, red, black]
  - tags: [Eyes]
    options: [blue, green, brown]
"#);
    let result = eval(&lib, "{Hair}, {Eyes}", None);

    assert!(result.text.contains(", "));
    assert_eq!(result.chosen_options.len(), 2);
}

#[test]
fn tag_query_via_alias_works() {
    let lib = lib(r#"
groups:
  - tags: [Hair, hair-color]
    options:
      - blonde hair
      - red hair
"#);
    let result1 = eval(&lib, "{Hair}", Some(123));
    let result2 = eval(&lib, "{hair-color}", Some(123));

    // Same seed, same underlying group = same result
    assert_eq!(result1.text, result2.text);
}

#[test]
fn tag_with_spaces_works() {
    let lib = lib(r#"
groups:
  - tags: [Hair Color]
    options:
      - blonde hair
      - red hair
"#);
    let result = eval(&lib, "{Hair Color}", Some(42));

    let valid_options = ["blonde hair", "red hair"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

// ============================================================================
// Tag Exclusion Tests: {Tag - exclude}
// ============================================================================

#[test]
fn tag_exclusion_filters_groups() {
    let lib = lib(r#"
groups:
  - tags: [Eyes]
    options: [blue eyes, green eyes]
  - tags: [Eyes, anime]
    options: [sparkling eyes, chibi eyes]
"#);
    // {Eyes - anime} should exclude the anime-tagged group
    for seed in 0..50 {
        let result = eval(&lib, "{Eyes - anime}", Some(seed));

        assert!(
            !result.text.contains("sparkling") && !result.text.contains("chibi"),
            "Seed {}: Got anime eyes '{}' despite exclusion",
            seed,
            result.text
        );
    }
}

#[test]
fn multiple_exclusions_work() {
    let lib = lib(r#"
groups:
  - tags: [Eyes]
    options: [blue eyes, green eyes]
  - tags: [Eyes, anime]
    options: [sparkling eyes]
  - tags: [Eyes, realistic]
    options: [detailed iris]
"#);
    // {Eyes - anime - realistic} should only get base Eyes options
    for seed in 0..50 {
        let result = eval(&lib, "{Eyes - anime - realistic}", Some(seed));

        let valid = ["blue eyes", "green eyes"];
        assert!(
            valid.contains(&result.text.as_str()),
            "Seed {}: Got '{}' which isn't from base Eyes group",
            seed,
            result.text
        );
    }
}

// ============================================================================
// Freeform Slot Tests: {{ SlotName }}
// ============================================================================

#[test]
fn freeform_slot_with_override() {
    let lib = lib("groups: []");
    let result = eval_with_slots(
        &lib,
        "A {{ Subject }} in the scene",
        &[("Subject", "small cat")],
        None,
    );
    assert_eq!(result.text, "A small cat in the scene");
}

#[test]
fn freeform_slot_without_override_preserved() {
    let lib = lib("groups: []");
    let result = eval(&lib, "A {{ Subject }} in the scene", None);
    assert_eq!(result.text, "A {{ Subject }} in the scene");
}

#[test]
fn multiple_freeform_slots() {
    let lib = lib("groups: []");
    let result = eval_with_slots(
        &lib,
        "{{ Subject }} doing {{ Action }}",
        &[("Subject", "a knight"), ("Action", "fighting a dragon")],
        None,
    );
    assert_eq!(result.text, "a knight doing fighting a dragon");
}

// ============================================================================
// Expression Block Tests: [[ expr ]]
// ============================================================================

#[test]
fn expression_block_selects_from_group() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options: [blonde hair, red hair, black hair]
"#);
    let result = eval(&lib, r#"[[ "Hair" ]]"#, None);

    let valid_options = ["blonde hair", "red hair", "black hair"];
    assert!(valid_options.contains(&result.text.as_str()));
}

#[test]
fn expression_block_with_some_operator() {
    let lib = lib(r#"
groups:
  - tags: [Style]
    options: [photorealistic, anime style, oil painting]
"#);
    let result = eval(&lib, r#"[[ "Style" | some ]]"#, None);

    let valid = ["photorealistic", "anime style", "oil painting"];
    assert!(valid.contains(&result.text.as_str()));
}

#[test]
fn expression_block_with_assign_records_value() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options: [blonde, red, black]
"#);
    let result = eval(&lib, r#"[[ "Hair" | some | assign("hair_choice") ]]"#, None);

    assert!(result.slot_values.contains_key("hair_choice"));
    assert_eq!(result.slot_values["hair_choice"], result.text);
}

// ============================================================================
// Comment Tests: # comment
// ============================================================================

#[test]
fn comments_not_in_output() {
    let lib = lib(r#"
groups:
  - tags: [Hair]
    options: [blonde]
  - tags: [Eyes]
    options: [blue]
"#);
    let result = eval(&lib, "{Hair} # this selects hair\n{Eyes}", None);

    assert!(!result.text.contains("this selects hair"));
    assert!(!result.text.contains("#"));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn empty_template_renders_empty() {
    let lib = lib("groups: []");
    let result = eval(&lib, "", None);
    assert_eq!(result.text, "");
}

#[test]
fn plain_text_only_renders_unchanged() {
    let lib = lib("groups: []");
    let result = eval(&lib, "Just some plain text without any tags", None);
    assert_eq!(result.text, "Just some plain text without any tags");
    assert!(result.chosen_options.is_empty());
}
