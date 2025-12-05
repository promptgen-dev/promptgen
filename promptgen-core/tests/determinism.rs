//! Tests for deterministic rendering.
//!
//! Verifies that:
//! - Same seed produces same results
//! - Different seeds produce variation

mod common;

use common::eval_template;
use std::collections::HashSet;

// ============================================================================
// Determinism Tests
// ============================================================================

#[test]
fn same_seed_produces_same_result() {
    let result1 = eval_template("Full Character", &[], Some(12345));
    let result2 = eval_template("Full Character", &[], Some(12345));

    assert_eq!(result1.text, result2.text);
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

