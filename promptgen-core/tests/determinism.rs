//! Tests for deterministic rendering and weighted selection.
//!
//! Verifies that:
//! - Same seed produces same results
//! - Different seeds produce variation
//! - Weighted options are selected proportionally

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

// ============================================================================
// Weighted Selection Tests
// ============================================================================

#[test]
fn weighted_selection_favors_higher_weights() {
    let mut masterpiece_count = 0;
    let mut normal_count = 0;

    for seed in 0..200 {
        let result = eval("{Quality}", Some(seed));

        if result.text.contains("masterpiece") {
            masterpiece_count += 1;
        } else if result.text == "normal quality" {
            normal_count += 1;
        }
    }

    // masterpiece has weight 10, normal has weight 1
    // So masterpiece should appear much more often
    assert!(
        masterpiece_count > normal_count * 3,
        "masterpiece ({}) should appear much more than normal ({})",
        masterpiece_count,
        normal_count
    );
}
