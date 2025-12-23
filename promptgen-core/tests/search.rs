//! Tests for fuzzy search functionality.

mod common;

use common::lib;
use promptgen_core::search::SearchResult;

fn test_library() -> promptgen_core::Library {
    lib(r#"
variables:
  - name: Hair
    options:
      - blonde hair
      - red hair
      - black hair
  - name: Eyes
    options:
      - blue eyes
      - green eyes
      - brown eyes
  - name: Hair Color
    options:
      - platinum blonde
      - strawberry blonde
"#)
}

// ============================================================================
// Variable Search Tests
// ============================================================================

#[test]
fn search_variables_empty_query_returns_all() {
    let library = test_library();
    let results = library.search_variables("");
    assert_eq!(results.len(), 3);
}

#[test]
fn search_variables_finds_exact_match() {
    let library = test_library();
    let results = library.search_variables("Hair");
    assert!(!results.is_empty());
    assert_eq!(results[0].variable_name, "Hair");
}

#[test]
fn search_variables_case_insensitive() {
    let library = test_library();
    let results = library.search_variables("hair");
    assert!(!results.is_empty());
    assert_eq!(results[0].variable_name, "Hair");
}

#[test]
fn search_variables_fuzzy_match() {
    let library = test_library();
    let results = library.search_variables("hr"); // fuzzy for "Hair"
    assert!(!results.is_empty());
    // Should find Hair and Hair Color
    let names: Vec<&str> = results.iter().map(|r| r.variable_name.as_str()).collect();
    assert!(names.contains(&"Hair") || names.contains(&"Hair Color"));
}

#[test]
fn search_variables_includes_match_indices() {
    let library = test_library();
    let results = library.search_variables("Hair");
    assert!(!results.is_empty());
    // Exact match should have indices
    assert!(!results[0].match_indices.is_empty());
}

#[test]
fn search_variables_sorted_by_score() {
    let library = test_library();
    let results = library.search_variables("Hair");
    // Results should be sorted by score descending
    for i in 1..results.len() {
        assert!(results[i - 1].score >= results[i].score);
    }
}

// ============================================================================
// Option Search Tests
// ============================================================================

#[test]
fn search_options_empty_query_returns_all() {
    let library = test_library();
    let results = library.search_options("", None);
    // Should return results from all variables
    assert!(results.len() >= 2);
}

#[test]
fn search_options_finds_match() {
    let library = test_library();
    let results = library.search_options("blonde", None);
    assert!(!results.is_empty());

    // Should find blonde options
    let all_matches: Vec<&str> = results
        .iter()
        .flat_map(|r| r.matches.iter().map(|m| m.text.as_str()))
        .collect();
    assert!(all_matches.iter().any(|m| m.contains("blonde")));
}

#[test]
fn search_options_with_variable_filter() {
    let library = test_library();
    let results = library.search_options("blonde", Some("Hair"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].variable_name, "Hair");
}

#[test]
fn search_options_variable_filter_case_insensitive() {
    let library = test_library();
    let results = library.search_options("blonde", Some("hair"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].variable_name, "Hair");
}

// ============================================================================
// Unified Search Tests
// ============================================================================

#[test]
fn unified_search_variables_with_at_prefix() {
    let library = test_library();
    let result = library.search("@Hair");
    match result {
        SearchResult::Variables(variables) => {
            assert!(!variables.is_empty());
            assert_eq!(variables[0].variable_name, "Hair");
        }
        SearchResult::Options(_) => panic!("Expected variables result"),
    }
}

#[test]
fn unified_search_options_with_variable() {
    let library = test_library();
    let result = library.search("@Hair/blonde");
    match result {
        SearchResult::Options(options) => {
            assert!(!options.is_empty());
            assert_eq!(options[0].variable_name, "Hair");
            assert!(options[0].matches.iter().any(|m| m.text.contains("blonde")));
        }
        SearchResult::Variables(_) => panic!("Expected options result"),
    }
}

#[test]
fn unified_search_options_all_variables() {
    let library = test_library();
    let result = library.search("@/blonde");
    match result {
        SearchResult::Options(options) => {
            assert!(!options.is_empty());
            // Should find blonde in multiple variables
            let all_matches: Vec<&str> = options
                .iter()
                .flat_map(|r| r.matches.iter().map(|m| m.text.as_str()))
                .collect();
            assert!(all_matches.iter().any(|m| m.contains("blonde")));
        }
        SearchResult::Variables(_) => panic!("Expected options result"),
    }
}

#[test]
fn unified_search_no_prefix_defaults_to_options() {
    let library = test_library();
    // Plain text search defaults to searching options across all variables
    let result = library.search("blonde");
    match result {
        SearchResult::Options(options) => {
            assert!(!options.is_empty());
            // Should find "blonde hair" in Hair variable
            let hair_result = options.iter().find(|r| r.variable_name == "Hair");
            assert!(hair_result.is_some());
        }
        SearchResult::Variables(_) => panic!("Expected options result"),
    }
}
