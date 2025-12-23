//! Tests for deterministic rendering.
//!
//! Verifies that using the same seed produces identical results.

mod common;

use common::{empty_lib, eval, lib};
use std::collections::HashSet;

// ============================================================================
// Determinism Tests
// ============================================================================

#[test]
fn same_seed_produces_same_result() {
    let lib = lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
      - red hair
      - black hair
      - brown hair
"#);

    let result1 = eval(&lib, "@Hair", Some(12345));
    let result2 = eval(&lib, "@Hair", Some(12345));

    assert_eq!(
        result1.text, result2.text,
        "Same seed should produce same result"
    );
}

#[test]
fn different_seeds_produce_different_results_eventually() {
    let lib = lib(r#"
variables:
  - name: Color
    options:
      - red
      - green
      - blue
      - yellow
      - purple
"#);

    // Try many seeds, should eventually get different results
    let mut results: HashSet<String> = HashSet::new();
    for seed in 0..100 {
        let result = eval(&lib, "@Color", Some(seed));
        results.insert(result.text);
    }

    // With 5 options and 100 trials, we should see multiple different results
    assert!(
        results.len() > 1,
        "Different seeds should produce different results. Got: {:?}",
        results
    );
}

#[test]
fn inline_options_are_deterministic() {
    let lib = empty_lib();

    let result1 = eval(&lib, "{a|b|c|d|e}", Some(999));
    let result2 = eval(&lib, "{a|b|c|d|e}", Some(999));

    assert_eq!(
        result1.text, result2.text,
        "Inline options with same seed should match"
    );
}

#[test]
fn complex_prompt_is_deterministic() {
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
"#);

    let prompt = "@Hair, @Eyes, {happy|sad} expression";
    let result1 = eval(&lib, prompt, Some(42));
    let result2 = eval(&lib, prompt, Some(42));

    assert_eq!(
        result1.text, result2.text,
        "Complex prompt with same seed should match"
    );
}

#[test]
fn same_options_produce_different_results_in_single_prompt() {
    let lib = lib(r#"
variables:
  - name: Color
    options:
      - red
      - green
      - blue
"#);

    // With multiple references to the same variable, we should potentially get different choices
    let mut found_different = false;
    for seed in 0..100 {
        let result = eval(&lib, "@Color and @Color", Some(seed));
        let parts: Vec<&str> = result.text.split(" and ").collect();
        if parts.len() == 2 && parts[0] != parts[1] {
            found_different = true;
            break;
        }
    }

    assert!(
        found_different,
        "Multiple references to same variable should sometimes produce different choices"
    );
}

#[test]
fn nested_grammar_is_deterministic() {
    let lib = lib(r#"
variables:
  - name: Size
    options:
      - big
      - small
  - name: Thing
    options:
      - "@Size dog"
      - "@Size cat"
"#);

    let result1 = eval(&lib, "@Thing", Some(777));
    let result2 = eval(&lib, "@Thing", Some(777));

    assert_eq!(
        result1.text, result2.text,
        "Nested grammar with same seed should match"
    );
}
