//! Tests for deterministic rendering.
//!
//! Verifies that:
//! - Same seed produces same results
//! - Different seeds produce variation

mod common;

use common::{eval, eval_template};
use std::collections::HashSet;

// ============================================================================
// Determinism Tests
// ============================================================================

#[test]
fn same_seed_produces_same_result() {
    let result1 = eval_template("Full Character", &[], Some(12345));
    let result2 = eval_template("Full Character", &[], Some(12345));

    println!("Result 1: {}", result1.text);
    println!("Result 2: {}", result2.text);
    assert_eq!(result1.text, result2.text);
}

#[test]
fn same_options_produce_different_results_in_single_prompt() {
    // append {Hair} to a string 100 times with the same seed,
    // and verify that we get different hair results
    let hair = "{Hair} ".repeat(100);
    let result = eval(&hair, Some(42));
    let hairs: HashSet<_> = result.text.split_whitespace().collect();
    assert!(hairs.len() > 1, "Expected variation in hair results");
}

#[test]
fn different_seeds_usually_produce_different_results() {
    let mut results = Vec::new();
    for seed in 0..10 {
        let result = eval_template("Full Character", &[], Some(seed));
        results.push(result.text);
    }

    // With so many random choices, we should get at least a few unique results
    let unique: HashSet<_> = results.iter().collect();
    assert!(
        unique.len() > 1,
        "Expected variation across seeds, got {} unique results",
        unique.len()
    );
}
