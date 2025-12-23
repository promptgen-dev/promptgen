//! Grammar and syntax tests for the prompt language.
//!
//! Tests the new grammar syntax:
//! - Library refs: `@Name` or `@"Name with spaces"` or `@"Lib:Name"`
//! - Inline options: `{a|b|c}`
//! - Slots: `{{ slot name }}`
//! - Comments: `# comment`

mod common;

use common::{empty_lib, eval, eval_with_slots, lib};

// ============================================================================
// Library Reference Tests: @Name
// ============================================================================

#[test]
fn simple_library_ref_renders() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
      - red hair
      - black hair
      - brown hair
"#);
    let result = eval(&lib, "@Hair", None);

    let valid_options = ["blonde hair", "red hair", "black hair", "brown hair"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn quoted_library_ref_renders() {
    let lib = lib(r#"
variables:
  - name: Hair Color
    options:
      - blonde
      - red
"#);
    let result = eval(&lib, r#"@"Hair Color""#, None);

    let valid_options = ["blonde", "red"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn library_ref_with_surrounding_text() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
"#);
    let result = eval(&lib, "A person with @Hair, looking happy", Some(42));

    assert_eq!(result.text, "A person with blonde hair, looking happy");
}

#[test]
fn multiple_library_refs_render() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
  - name: Eyes
    options:
      - blue eyes
"#);
    let result = eval(&lib, "@Hair with @Eyes", Some(42));

    assert_eq!(result.text, "blonde hair with blue eyes");
}

// ============================================================================
// Inline Options Tests: {a|b|c}
// ============================================================================

#[test]
fn inline_options_render() {
    let lib = empty_lib();
    let result = eval(&lib, "{happy|sad}", None);

    let valid_options = ["happy", "sad"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn inline_options_with_text() {
    let lib = empty_lib();
    let result = eval(&lib, "feeling {happy|sad} today", Some(42));

    // With seed 42, we should get deterministic output
    assert!(
        result.text == "feeling happy today" || result.text == "feeling sad today",
        "Result should be valid inline option choice"
    );
}

#[test]
fn inline_options_three_choices() {
    let lib = empty_lib();
    let result = eval(&lib, "{red|green|blue}", None);

    let valid_options = ["red", "green", "blue"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

// ============================================================================
// Slot Tests: {{ Name }}
// ============================================================================

#[test]
fn slot_without_override_renders_empty() {
    let lib = empty_lib();
    let result = eval(&lib, "Hello {{ Name }}", None);

    // Empty slots render to empty string per spec
    assert_eq!(result.text, "Hello ");
}

#[test]
fn slot_with_override_renders() {
    let lib = empty_lib();
    let result = eval_with_slots(&lib, "Hello {{ Name }}", &[("Name", "Alice")], None);

    assert_eq!(result.text, "Hello Alice");
}

#[test]
fn multiple_slots_render() {
    let lib = empty_lib();
    let result = eval_with_slots(
        &lib,
        "{{ Name }} lives in {{ Place }}",
        &[("Name", "Alice"), ("Place", "Wonderland")],
        None,
    );

    assert_eq!(result.text, "Alice lives in Wonderland");
}

#[test]
fn slot_with_grammar_in_value() {
    let lib = lib(r#"
variables:
  - name: Color
    options:
      - red
      - blue
"#);
    let result = eval_with_slots(&lib, "The sky is {{ Color }}", &[("Color", "@Color")], None);

    let valid_options = ["The sky is red", "The sky is blue"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

// ============================================================================
// Comment Tests: # comment
// ============================================================================

#[test]
fn comment_not_included_in_output() {
    let lib = empty_lib();
    let result = eval(&lib, "# This is a comment", None);

    assert_eq!(result.text, "");
}

// ============================================================================
// Nested Grammar Tests
// ============================================================================

#[test]
fn nested_library_ref_in_variable_option() {
    let lib = lib(r#"
variables:
  - name: Color
    options:
      - red
      - blue
  - name: Description
    options:
      - "@Color car"
"#);
    let result = eval(&lib, "@Description", None);

    let valid_options = ["red car", "blue car"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn nested_inline_options_in_variable() {
    let lib = lib(r#"
variables:
  - name: Size
    options:
      - "{big|small} thing"
"#);
    let result = eval(&lib, "@Size", None);

    let valid_options = ["big thing", "small thing"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn nested_inline_options_direct() {
    // Test {a|b|{c|d}} - direct nested inline options in prompt
    let lib = lib(r#"
variables:
  - name: Placeholder
    options:
      - placeholder
"#);
    let result = eval(&lib, "{a|b|{c|d}}", None);

    let valid_options = ["a", "b", "c", "d"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn deeply_nested_inline_options() {
    // Test {a|{b|{c|d}}} - deeply nested inline options
    let lib = lib(r#"
variables:
  - name: Placeholder
    options:
      - placeholder
"#);
    let result = eval(&lib, "{a|{b|{c|d}}}", None);

    let valid_options = ["a", "b", "c", "d"];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

#[test]
fn nested_inline_options_with_surrounding_text() {
    // Test prefix {a|{nested|choice}} suffix
    let lib = lib(r#"
variables:
  - name: Placeholder
    options:
      - placeholder
"#);
    let result = eval(&lib, "prefix {a|{nested|choice}} suffix", None);

    let valid_options = [
        "prefix a suffix",
        "prefix nested suffix",
        "prefix choice suffix",
    ];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}

// ============================================================================
// Mixed Grammar Tests
// ============================================================================

#[test]
fn complex_prompt_with_all_features() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
  - name: Eyes
    options:
      - blue eyes
"#);
    let result = eval_with_slots(
        &lib,
        "@Hair, @Eyes, {happy|cheerful} expression, {{ Extra }}",
        &[("Extra", "high quality")],
        Some(42),
    );

    // Should contain the expected parts
    assert!(result.text.contains("blonde hair"));
    assert!(result.text.contains("blue eyes"));
    assert!(result.text.contains("expression"));
    assert!(result.text.contains("high quality"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn empty_prompt_renders_empty() {
    let lib = empty_lib();
    let result = eval(&lib, "", None);

    assert_eq!(result.text, "");
}

#[test]
fn plain_text_only() {
    let lib = empty_lib();
    let result = eval(&lib, "Just plain text, no grammar", None);

    assert_eq!(result.text, "Just plain text, no grammar");
}
