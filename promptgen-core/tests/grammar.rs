//! Grammar and syntax tests for the template language.
//!
//! Tests all the different syntactic constructs:
//! - Tag queries: `{Tag}`, `{Tag - exclude}`
//! - Freeform slots: `{{ SlotName }}`
//! - Expression blocks: `[[ "Tag" | op ]]`
//! - Comments: `# comment`

mod common;

use common::{eval, eval_with_slots, try_eval};

// ============================================================================
// Basic Tag Query Tests: {Tag}
// ============================================================================

#[test]
fn simple_tag_query_renders() {
    let result = eval("{Hair}", None);

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
    let result = eval("{Hair}, {Eyes}", None);

    // Should contain a comma separating two selections
    assert!(result.text.contains(", "));
    assert_eq!(result.chosen_options.len(), 2);
}

#[test]
fn tag_query_via_alias_works() {
    let result1 = eval("{Hair}", Some(123));
    let result2 = eval("{hair-color}", Some(123));

    // Same seed, same underlying group = same result
    assert_eq!(result1.text, result2.text);
}

#[test]
fn tag_with_spaces_works() {
    let result = eval("{Hair Color}", Some(42));

    let valid_options = ["blonde hair", "red hair", "black hair", "brown hair"];
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
    // {Eyes - anime} should exclude AnimeEyes group
    // Run many times to ensure anime eyes never appear
    for seed in 0..50 {
        let result = eval("{Eyes - anime}", Some(seed));

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
    // {Eyes - anime - realistic} should only get base Eyes options
    for seed in 0..50 {
        let result = eval("{Eyes - anime - realistic}", Some(seed));

        // Should only get options from the base Eyes group
        let valid = ["blue eyes", "green eyes", "brown eyes", "heterochromia"];
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
    let result = eval_with_slots(
        "A {{ Subject }} in the scene",
        &[("Subject", "small cat")],
        None,
    );
    assert_eq!(result.text, "A small cat in the scene");
}

#[test]
fn freeform_slot_without_override_preserved() {
    let result = eval("A {{ Subject }} in the scene", None);
    assert_eq!(result.text, "A {{ Subject }} in the scene");
}

#[test]
fn multiple_freeform_slots() {
    let result = eval_with_slots(
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
    let result = eval(r#"[[ "Hair" ]]"#, None);

    let valid_options = ["blonde hair", "red hair", "black hair", "brown hair"];
    assert!(valid_options.contains(&result.text.as_str()));
}

#[test]
fn expression_block_with_some_operator() {
    let result = eval(r#"[[ "Style" | some ]]"#, None);

    let valid = [
        "photorealistic",
        "anime style",
        "oil painting",
        "watercolor",
    ];
    assert!(valid.contains(&result.text.as_str()));
}

#[test]
fn expression_block_with_assign_records_value() {
    let result = eval(r#"[[ "Hair" | some | assign("hair_choice") ]]"#, None);

    assert!(result.slot_values.contains_key("hair_choice"));
    assert_eq!(result.slot_values["hair_choice"], result.text);
}

// ============================================================================
// Comment Tests: # comment
// ============================================================================

#[test]
fn comments_not_in_output() {
    let result = eval("{Hair} # this selects hair\n{Eyes}", None);

    assert!(!result.text.contains("this selects hair"));
    assert!(!result.text.contains("#"));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn nonexistent_tag_returns_error() {
    let result = try_eval("{NonExistentTag}", None);
    assert!(result.is_err());
}

#[test]
fn empty_template_renders_empty() {
    let result = eval("", None);
    assert_eq!(result.text, "");
}

#[test]
fn plain_text_only_renders_unchanged() {
    let result = eval("Just some plain text without any tags", None);
    assert_eq!(result.text, "Just some plain text without any tags");
    assert!(result.chosen_options.is_empty());
}
