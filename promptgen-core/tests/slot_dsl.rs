//! Integration tests for the Slot DSL v0.1.
//!
//! Tests the new slot syntax:
//! - Textarea slots: `{{ label }}`
//! - Pick slots: `{{ label: pick(@Group, "literal") }}`
//! - Operators: `| one` and `| many(max=N, sep=",")`
//! - Duplicate label detection

mod common;

use common::{empty_lib, eval, eval_with_slot_values, eval_with_slots, lib, try_eval_with_slot_values};
use promptgen_core::{ParseError, RenderError, parse_template};

// ============================================================================
// Textarea Slot Tests
// ============================================================================

#[test]
fn textarea_slot_basic() {
    let lib = empty_lib();
    let result = eval_with_slots(&lib, "Hello {{ Name }}", &[("Name", "World")], None);
    assert_eq!(result.text, "Hello World");
}

#[test]
fn textarea_slot_with_quoted_label() {
    let lib = empty_lib();
    let result = eval_with_slots(
        &lib,
        r#"{{ "User Name" }}"#,
        &[("User Name", "Alice")],
        None,
    );
    assert_eq!(result.text, "Alice");
}

#[test]
fn textarea_slot_preserves_unset_value() {
    let lib = empty_lib();
    let result = eval(&lib, "Hello {{ Name }}", None);
    assert_eq!(result.text, "Hello {{ Name }}");
}

#[test]
fn multiple_textarea_slots() {
    let lib = empty_lib();
    let result = eval_with_slots(
        &lib,
        "Hello {{ FirstName }} {{ LastName }}!",
        &[("FirstName", "John"), ("LastName", "Doe")],
        None,
    );
    assert_eq!(result.text, "Hello John Doe!");
}

// ============================================================================
// Pick Slot Placeholder Tests
// ============================================================================

#[test]
fn pick_slot_preserves_placeholder_without_value() {
    let lib = empty_lib();
    let result = eval(&lib, "{{ Choice: pick(@Color) }}", None);
    assert_eq!(result.text, "{{ Choice: pick(@Color) }}");
}

#[test]
fn pick_slot_with_one_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(&lib, "{{ Choice: pick(@Color) | one }}", None);
    assert_eq!(result.text, "{{ Choice: pick(@Color) | one }}");
}

#[test]
fn pick_slot_with_many_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(&lib, "{{ Tags: pick(@Tag) | many }}", None);
    assert_eq!(result.text, "{{ Tags: pick(@Tag) | many }}");
}

#[test]
fn pick_slot_with_many_max_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(&lib, "{{ Colors: pick(@Color) | many(max=2) }}", None);
    assert_eq!(result.text, "{{ Colors: pick(@Color) | many(max=2) }}");
}

#[test]
fn pick_slot_with_many_sep_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(
        &lib,
        r#"{{ Words: pick(@Word) | many(sep=" | ") }}"#,
        None,
    );
    assert_eq!(result.text, r#"{{ Words: pick(@Word) | many(sep=" | ") }}"#);
}

#[test]
fn pick_slot_with_many_max_and_sep_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(
        &lib,
        r#"{{ Fruits: pick(@Fruit) | many(max=3, sep="; ") }}"#,
        None,
    );
    assert_eq!(
        result.text,
        r#"{{ Fruits: pick(@Fruit) | many(max=3, sep="; ") }}"#
    );
}

#[test]
fn pick_slot_with_literals_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(
        &lib,
        r#"{{ Choice: pick("yes", "no", "maybe") }}"#,
        None,
    );
    assert_eq!(result.text, r#"{{ Choice: pick("yes", "no", "maybe") }}"#);
}

#[test]
fn pick_slot_with_mixed_sources_preserves_placeholder() {
    let lib = empty_lib();
    let result = eval(&lib, r#"{{ Choice: pick(@Color, "custom") }}"#, None);
    assert_eq!(result.text, r#"{{ Choice: pick(@Color, "custom") }}"#);
}

// ============================================================================
// Pick Slot With Value Tests
// ============================================================================

#[test]
fn pick_slot_renders_with_single_value() {
    let lib = empty_lib();
    let result = eval_with_slots(
        &lib,
        "{{ Choice: pick(@Color) | one }}",
        &[("Choice", "red")],
        None,
    );
    assert_eq!(result.text, "red");
}

#[test]
fn pick_slot_renders_with_multiple_values() {
    let lib = empty_lib();
    // User provides array of values for a many slot
    let result = eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) | many(max=3) }}",
        &[("Colors", vec!["red", "blue"])],
        None,
    );
    assert_eq!(result.text, "red, blue");
}

#[test]
fn pick_slot_value_can_contain_grammar() {
    let lib = lib(r#"
groups:
  - name: Color
    options:
      - red
      - blue
"#);
    // User provides a grammar expression as the slot value
    let result = eval_with_slots(
        &lib,
        "{{ FavoriteColor: pick(@Color) | one }}",
        &[("FavoriteColor", "@Color")],
        Some(42),
    );
    let valid = ["red", "blue"];
    assert!(
        valid.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid
    );
}

#[test]
fn pick_slot_value_with_inline_options() {
    let lib = empty_lib();
    // User provides inline options as the slot value
    let result = eval_with_slots(
        &lib,
        "{{ Style: pick(@Styles) | one }}",
        &[("Style", "{bold|italic}")],
        Some(42),
    );
    let valid = ["bold", "italic"];
    assert!(
        valid.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid
    );
}

// ============================================================================
// Pick Slot Validation Tests - One Operator
// ============================================================================

#[test]
fn pick_slot_one_rejects_multiple_values() {
    let lib = empty_lib();
    // Pass multiple array elements to a | one slot - should error
    let result = try_eval_with_slot_values(
        &lib,
        "{{ Choice: pick(@Color) | one }}",
        &[("Choice", vec!["red", "blue"])],
        None,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        RenderError::TooManyValuesForOne { slot, count } => {
            assert_eq!(slot, "Choice");
            assert_eq!(count, 2);
        }
        other => panic!("Expected TooManyValuesForOne error, got {:?}", other),
    }
}

#[test]
fn pick_slot_one_rejects_three_values() {
    let lib = empty_lib();
    // Pass three array elements to a | one slot - should error
    let result = try_eval_with_slot_values(
        &lib,
        "{{ Color: pick(@Color) | one }}",
        &[("Color", vec!["red", "green", "blue"])],
        None,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        RenderError::TooManyValuesForOne { slot, count } => {
            assert_eq!(slot, "Color");
            assert_eq!(count, 3);
        }
        other => panic!("Expected TooManyValuesForOne error, got {:?}", other),
    }
}

#[test]
fn pick_slot_one_accepts_single_value() {
    let lib = empty_lib();
    let result = eval_with_slots(
        &lib,
        "{{ Choice: pick(@Color) | one }}",
        &[("Choice", "red")],
        None,
    );
    assert_eq!(result.text, "red");
}

// ============================================================================
// Pick Slot Validation Tests - Many Operator with Max
// ============================================================================

#[test]
fn pick_slot_many_max_rejects_exceeding_count() {
    let lib = empty_lib();
    // Pass 3 values to a | many(max=2) slot - should error
    let result = try_eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) | many(max=2) }}",
        &[("Colors", vec!["red", "green", "blue"])],
        None,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        RenderError::TooManyValuesForMany { slot, max, count } => {
            assert_eq!(slot, "Colors");
            assert_eq!(max, 2);
            assert_eq!(count, 3);
        }
        other => panic!("Expected TooManyValuesForMany error, got {:?}", other),
    }
}

#[test]
fn pick_slot_many_max_accepts_at_max() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) | many(max=3) }}",
        &[("Colors", vec!["red", "green", "blue"])],
        None,
    );
    assert_eq!(result.text, "red, green, blue");
}

#[test]
fn pick_slot_many_max_accepts_below_max() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) | many(max=5) }}",
        &[("Colors", vec!["red", "blue"])],
        None,
    );
    assert_eq!(result.text, "red, blue");
}

#[test]
fn pick_slot_many_no_max_accepts_many_values() {
    let lib = empty_lib();
    // Without max specified, should accept any number of values
    let result = eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) | many }}",
        &[("Colors", vec!["red", "green", "blue", "yellow", "purple"])],
        None,
    );
    assert_eq!(result.text, "red, green, blue, yellow, purple");
}

// ============================================================================
// Pick Slot Custom Separator Tests
// ============================================================================

#[test]
fn pick_slot_many_custom_separator_pipe() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        r#"{{ Tags: pick(@Tag) | many(sep=" | ") }}"#,
        &[("Tags", vec!["art", "photo", "landscape"])],
        None,
    );
    assert_eq!(result.text, "art | photo | landscape");
}

#[test]
fn pick_slot_many_custom_separator_semicolon() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        r#"{{ Items: pick(@Item) | many(sep="; ") }}"#,
        &[("Items", vec!["apple", "banana", "cherry"])],
        None,
    );
    assert_eq!(result.text, "apple; banana; cherry");
}

#[test]
fn pick_slot_many_custom_separator_newline() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        r#"{{ Lines: pick(@Line) | many(sep="\n") }}"#,
        &[("Lines", vec!["first", "second", "third"])],
        None,
    );
    assert_eq!(result.text, "first\nsecond\nthird");
}

#[test]
fn pick_slot_many_custom_separator_tab() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        r#"{{ Cols: pick(@Col) | many(sep="\t") }}"#,
        &[("Cols", vec!["A", "B", "C"])],
        None,
    );
    assert_eq!(result.text, "A\tB\tC");
}

#[test]
fn pick_slot_many_max_and_sep_combined() {
    let lib = empty_lib();
    let result = eval_with_slot_values(
        &lib,
        r#"{{ Colors: pick(@Color) | many(max=3, sep=" / ") }}"#,
        &[("Colors", vec!["red", "blue", "green"])],
        None,
    );
    assert_eq!(result.text, "red / blue / green");
}

#[test]
fn pick_slot_many_max_and_sep_rejects_exceeding() {
    let lib = empty_lib();
    let result = try_eval_with_slot_values(
        &lib,
        r#"{{ Colors: pick(@Color) | many(max=2, sep=" / ") }}"#,
        &[("Colors", vec!["red", "blue", "green"])],
        None,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        RenderError::TooManyValuesForMany { slot, max, count } => {
            assert_eq!(slot, "Colors");
            assert_eq!(max, 2);
            assert_eq!(count, 3);
        }
        other => panic!("Expected TooManyValuesForMany error, got {:?}", other),
    }
}

#[test]
fn pick_slot_default_separator_is_comma_space() {
    let lib = empty_lib();
    // Without sep specified, default should be ", "
    let result = eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) | many }}",
        &[("Colors", vec!["red", "blue"])],
        None,
    );
    assert_eq!(result.text, "red, blue");
}

#[test]
fn pick_slot_no_operators_defaults_to_many() {
    let lib = empty_lib();
    // Without any operators, pick should default to many with default separator
    let result = eval_with_slot_values(
        &lib,
        "{{ Colors: pick(@Color) }}",
        &[("Colors", vec!["red", "blue", "green"])],
        None,
    );
    assert_eq!(result.text, "red, blue, green");
}

// ============================================================================
// Parse Error Tests
// ============================================================================

#[test]
fn duplicate_labels_rejected() {
    let src = "{{ Name }} and {{ Name }}";
    let result = parse_template(src);

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::DuplicateLabel { label, .. } => {
            assert_eq!(label, "Name");
        }
        other => panic!("Expected DuplicateLabel error, got {:?}", other),
    }
}

#[test]
fn duplicate_pick_labels_rejected() {
    let src = "{{ Choice: pick(@A) }} and {{ Choice: pick(@B) }}";
    let result = parse_template(src);

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::DuplicateLabel { label, .. } => {
            assert_eq!(label, "Choice");
        }
        other => panic!("Expected DuplicateLabel error, got {:?}", other),
    }
}

#[test]
fn different_labels_allowed() {
    let src = "{{ Name }} and {{ Age }}";
    let result = parse_template(src);
    assert!(result.is_ok());
}

// ============================================================================
// Combination Tests
// ============================================================================

#[test]
fn textarea_and_pick_slots_together() {
    let lib = empty_lib();
    // When both slots have values provided
    let result = eval_with_slots(
        &lib,
        "{{ Name }} likes {{ FavoriteColor: pick(@Color) | one }}",
        &[("Name", "Alice"), ("FavoriteColor", "blue")],
        None,
    );
    assert_eq!(result.text, "Alice likes blue");
}

#[test]
fn textarea_and_pick_slots_partial_values() {
    let lib = empty_lib();
    // When only textarea has value, pick slot shows placeholder
    let result = eval_with_slots(
        &lib,
        "{{ Name }} likes {{ FavoriteColor: pick(@Color) | one }}",
        &[("Name", "Alice")],
        None,
    );
    assert_eq!(
        result.text,
        "Alice likes {{ FavoriteColor: pick(@Color) | one }}"
    );
}

#[test]
fn pick_slot_with_inline_options() {
    let lib = empty_lib();
    // Inline options evaluate, but pick slot preserves placeholder
    let result = eval(&lib, "A {big|small} {{ Choice: pick(@Hair) | one }}", Some(42));

    // Should start with "A big " or "A small "
    let valid_starts = ["A big ", "A small "];
    assert!(
        valid_starts.iter().any(|s| result.text.starts_with(s)),
        "Result should start with 'A big ' or 'A small ', got '{}'",
        result.text
    );
    // Should end with the pick slot placeholder
    assert!(
        result.text.ends_with("{{ Choice: pick(@Hair) | one }}"),
        "Should end with pick slot placeholder, got '{}'",
        result.text
    );
}

#[test]
fn pick_slot_with_library_ref() {
    let lib = lib(r#"
groups:
  - name: Hair
    options:
      - blonde hair
      - red hair
"#);
    // Library refs evaluate, but pick slot preserves placeholder
    let result = eval(
        &lib,
        "@Hair and {{ EyeChoice: pick(@Eyes) | one }}",
        Some(42),
    );

    // Should contain hair from library ref (evaluated)
    let has_hair = result.text.contains("blonde hair") || result.text.contains("red hair");
    assert!(
        has_hair,
        "Result '{}' should contain hair type",
        result.text
    );

    // Should contain the pick slot placeholder (not evaluated)
    assert!(
        result.text.contains("{{ EyeChoice: pick(@Eyes) | one }}"),
        "Result '{}' should contain pick slot placeholder",
        result.text
    );
}

#[test]
fn pick_slot_with_library_ref_and_value() {
    let lib = lib(r#"
groups:
  - name: Hair
    options:
      - blonde hair
      - red hair
"#);
    // Both library refs and pick slots with value provided
    let result = eval_with_slots(
        &lib,
        "@Hair and {{ EyeChoice: pick(@Eyes) | one }}",
        &[("EyeChoice", "blue eyes")],
        Some(42),
    );

    // Should contain hair from library ref
    let has_hair = result.text.contains("blonde hair") || result.text.contains("red hair");
    assert!(
        has_hair,
        "Result '{}' should contain hair type",
        result.text
    );

    // Should contain the provided eye choice
    assert!(
        result.text.contains("blue eyes"),
        "Result '{}' should contain provided eye choice",
        result.text
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn slot_label_with_special_chars_quoted() {
    let lib = empty_lib();
    let result = eval_with_slots(
        &lib,
        r#"{{ "Label:With:Colons" }}"#,
        &[("Label:With:Colons", "Value")],
        None,
    );
    assert_eq!(result.text, "Value");
}

#[test]
fn empty_pick_source_parses_ok() {
    // This should still parse, but might error at runtime
    let result = parse_template("{{ Choice: pick(@Empty) }}");
    assert!(result.is_ok());
}
