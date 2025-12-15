//! Tests for fuzzy search functionality.

mod common;

use common::{lib, workspace};
use promptgen_core::search::SearchResult;
use promptgen_core::WorkspaceBuilder;

fn test_library() -> promptgen_core::Library {
    lib(r#"
groups:
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
// Group Search Tests
// ============================================================================

#[test]
fn search_groups_empty_query_returns_all() {
    let ws = workspace(test_library());
    let results = ws.search_groups("");
    assert_eq!(results.len(), 3);
}

#[test]
fn search_groups_finds_exact_match() {
    let ws = workspace(test_library());
    let results = ws.search_groups("Hair");
    assert!(!results.is_empty());
    assert_eq!(results[0].group_name, "Hair");
}

#[test]
fn search_groups_case_insensitive() {
    let ws = workspace(test_library());
    let results = ws.search_groups("hair");
    assert!(!results.is_empty());
    assert_eq!(results[0].group_name, "Hair");
}

#[test]
fn search_groups_fuzzy_match() {
    let ws = workspace(test_library());
    let results = ws.search_groups("hr"); // fuzzy for "Hair"
    assert!(!results.is_empty());
    // Should find Hair and Hair Color
    let names: Vec<&str> = results.iter().map(|r| r.group_name.as_str()).collect();
    assert!(names.contains(&"Hair") || names.contains(&"Hair Color"));
}

#[test]
fn search_groups_includes_match_indices() {
    let ws = workspace(test_library());
    let results = ws.search_groups("Hair");
    assert!(!results.is_empty());
    // Exact match should have indices
    assert!(!results[0].match_indices.is_empty());
}

#[test]
fn search_groups_sorted_by_score() {
    let ws = workspace(test_library());
    let results = ws.search_groups("Hair");
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
    let ws = workspace(test_library());
    let results = ws.search_options("", None);
    // Should return results from all groups
    assert!(results.len() >= 2);
}

#[test]
fn search_options_finds_match() {
    let ws = workspace(test_library());
    let results = ws.search_options("blonde", None);
    assert!(!results.is_empty());

    // Should find blonde options
    let all_matches: Vec<&str> = results
        .iter()
        .flat_map(|r| r.matches.iter().map(|m| m.text.as_str()))
        .collect();
    assert!(all_matches.iter().any(|m| m.contains("blonde")));
}

#[test]
fn search_options_with_group_filter() {
    let ws = workspace(test_library());
    let results = ws.search_options("blonde", Some("Hair"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].group_name, "Hair");
}

#[test]
fn search_options_group_filter_case_insensitive() {
    let ws = workspace(test_library());
    let results = ws.search_options("blonde", Some("hair"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].group_name, "Hair");
}

// ============================================================================
// Unified Search Tests
// ============================================================================

#[test]
fn unified_search_groups_with_at_prefix() {
    let ws = workspace(test_library());
    let result = ws.search("@Hair");
    match result {
        SearchResult::Groups(groups) => {
            assert!(!groups.is_empty());
            assert_eq!(groups[0].group_name, "Hair");
        }
        SearchResult::Options(_) => panic!("Expected groups result"),
    }
}

#[test]
fn unified_search_options_with_group() {
    let ws = workspace(test_library());
    let result = ws.search("@Hair/blonde");
    match result {
        SearchResult::Options(options) => {
            assert!(!options.is_empty());
            assert_eq!(options[0].group_name, "Hair");
            assert!(options[0].matches.iter().any(|m| m.text.contains("blonde")));
        }
        SearchResult::Groups(_) => panic!("Expected options result"),
    }
}

#[test]
fn unified_search_options_all_groups() {
    let ws = workspace(test_library());
    let result = ws.search("@/blonde");
    match result {
        SearchResult::Options(options) => {
            assert!(!options.is_empty());
            // Should find blonde in multiple groups
            let all_matches: Vec<&str> = options
                .iter()
                .flat_map(|r| r.matches.iter().map(|m| m.text.as_str()))
                .collect();
            assert!(all_matches.iter().any(|m| m.contains("blonde")));
        }
        SearchResult::Groups(_) => panic!("Expected options result"),
    }
}

#[test]
fn unified_search_no_prefix_defaults_to_options() {
    let ws = workspace(test_library());
    // Plain text search defaults to searching options across all groups
    let result = ws.search("blonde");
    match result {
        SearchResult::Options(options) => {
            assert!(!options.is_empty());
            // Should find "blonde hair" in Hair group
            let hair_result = options.iter().find(|r| r.group_name == "Hair");
            assert!(hair_result.is_some());
        }
        SearchResult::Groups(_) => panic!("Expected options result"),
    }
}

// ============================================================================
// Multi-Library Search Tests
// ============================================================================

#[test]
fn search_across_multiple_libraries() {
    let lib1 = lib(r#"
groups:
  - name: Style
    options: [modern]
"#);

    let lib2 = lib(r#"
groups:
  - name: Style
    options: [vintage]
"#);

    let ws = WorkspaceBuilder::new()
        .add_library(lib1)
        .add_library(lib2)
        .build();

    let results = ws.search_groups("Style");
    assert_eq!(results.len(), 2);
}
