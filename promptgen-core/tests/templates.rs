//! Tests for saved prompts from the library.
//!
//! Tests that prompts stored in libraries render correctly.

mod common;

use common::{eval_prompt, lib};

// ============================================================================
// Prompt Rendering Tests
// ============================================================================

#[test]
fn basic_character_prompt_renders() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
      - red hair
  - name: Eyes
    options:
      - blue eyes
      - green eyes
prompts:
  - name: Character
    content: "@Hair, @Eyes"
"#);

    let result = eval_prompt(&lib, "Character", Some(42));

    // Should produce a valid combination
    assert!(result.text.contains("hair"));
    assert!(result.text.contains("eyes"));
}

#[test]
fn prompt_with_slots_renders() {
    let lib = lib(r#"
variables: []
prompts:
  - name: Greeting
    content: "Hello {{ Name }}"
"#);

    let result = eval_prompt(&lib, "Greeting", Some(42));

    // Without slot override, the slot renders to empty string per spec
    assert_eq!(result.text, "Hello ");
}

#[test]
fn prompt_with_inline_options_renders() {
    let lib = lib(r#"
variables: []
prompts:
  - name: Mood
    content: "Feeling {happy|sad|excited} today"
"#);

    let result = eval_prompt(&lib, "Mood", Some(42));

    assert!(result.text.starts_with("Feeling "));
    assert!(result.text.ends_with(" today"));
}

#[test]
fn complex_prompt_renders() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
  - name: Eyes
    options:
      - blue eyes
  - name: Expression
    options:
      - smiling
      - serious
prompts:
  - name: Portrait
    content: "A person with @Hair and @Eyes, @Expression, {realistic|artistic} style"
"#);

    let result = eval_prompt(&lib, "Portrait", Some(42));

    // Should contain all the expected parts
    assert!(result.text.contains("blonde hair"));
    assert!(result.text.contains("blue eyes"));
    assert!(result.text.contains("smiling") || result.text.contains("serious"));
    assert!(result.text.contains("style"));
}

#[test]
fn prompt_chosen_options_are_tracked() {
    let lib = lib(r#"
variables:
  - name: Color
    options:
      - red
      - blue
prompts:
  - name: Simple
    content: "@Color"
"#);

    let result = eval_prompt(&lib, "Simple", Some(42));

    assert_eq!(result.chosen_options.len(), 1);
    assert_eq!(result.chosen_options[0].variable_name, "Color");
    assert!(result.chosen_options[0].option_text == "red" || result.chosen_options[0].option_text == "blue");
}

#[test]
fn prompt_with_nested_grammar_renders() {
    let lib = lib(r#"
variables:
  - name: Size
    options:
      - big
      - small
  - name: Animal
    options:
      - "@Size dog"
      - "@Size cat"
prompts:
  - name: Pet
    content: "My pet is a @Animal"
"#);

    let result = eval_prompt(&lib, "Pet", Some(42));

    // Should resolve nested grammar
    let valid_options = [
        "My pet is a big dog",
        "My pet is a small dog",
        "My pet is a big cat",
        "My pet is a small cat",
    ];
    assert!(
        valid_options.contains(&result.text.as_str()),
        "Result '{}' should be one of {:?}",
        result.text,
        valid_options
    );
}
