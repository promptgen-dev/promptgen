//! Fuzzy search functionality for workspaces.
//!
//! Provides fuzzy matching for groups and options across all libraries
//! in a workspace.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::workspace::Workspace;

/// Result of a fuzzy search for a group.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupSearchResult {
    /// ID of the library containing this group
    pub library_id: String,
    /// Display name of the library
    pub library_name: String,
    /// Name of the matched group
    pub group_name: String,
    /// All options in this group
    pub options: Vec<String>,
    /// Raw fuzzy match score (higher is better)
    pub score: i64,
    /// Indices of matched characters in the group name
    pub match_indices: Vec<usize>,
}

/// A single option match within a group.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionMatch {
    /// The option text
    pub text: String,
    /// Raw fuzzy match score (higher is better)
    pub score: i64,
    /// Indices of matched characters in the option text
    pub match_indices: Vec<usize>,
}

/// Result of a fuzzy search for options.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionSearchResult {
    /// ID of the library containing this group
    pub library_id: String,
    /// Display name of the library
    pub library_name: String,
    /// Name of the group containing matched options
    pub group_name: String,
    /// Matched options with their scores
    pub matches: Vec<OptionMatch>,
}

/// Unified search result that can contain either groups or options.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SearchResult {
    /// Group search results
    Groups(Vec<GroupSearchResult>),
    /// Option search results
    Options(Vec<OptionSearchResult>),
}

impl Workspace {
    /// Search for groups matching the query across all libraries.
    ///
    /// Returns all groups if query is empty. Results are sorted by score (highest first).
    /// Search is case-insensitive.
    ///
    /// # Example
    ///
    /// ```
    /// # use promptgen_core::workspace::WorkspaceBuilder;
    /// # use promptgen_core::library::Library;
    /// let workspace = WorkspaceBuilder::new()
    ///     .add_library(Library::new("My Library"))
    ///     .build();
    /// let results = workspace.search_groups("hair");
    /// ```
    pub fn search_groups(&self, query: &str) -> Vec<GroupSearchResult> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let query = query.trim();

        let mut results = Vec::new();

        for library in self.libraries() {
            for group in &library.groups {
                let group_name = &group.name;

                if query.is_empty() {
                    // Return all groups with score 0 when query is empty
                    results.push(GroupSearchResult {
                        library_id: library.id.clone(),
                        library_name: library.name.clone(),
                        group_name: group_name.to_string(),
                        options: group.options.clone(),
                        score: 0,
                        match_indices: vec![],
                    });
                } else if let Some((score, indices)) = matcher.fuzzy_indices(group_name, query) {
                    results.push(GroupSearchResult {
                        library_id: library.id.clone(),
                        library_name: library.name.clone(),
                        group_name: group_name.to_string(),
                        options: group.options.clone(),
                        score,
                        match_indices: indices,
                    });
                }
            }
        }

        // Sort by score descending (highest first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        results
    }

    /// Search for options matching the query, optionally filtered to a specific group.
    ///
    /// Returns all options if query is empty. Results are sorted by best match score within each group.
    /// Search is case-insensitive.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query
    /// * `group_filter` - Optional group name to filter results to
    ///
    /// # Example
    ///
    /// ```
    /// # use promptgen_core::workspace::WorkspaceBuilder;
    /// # use promptgen_core::library::Library;
    /// let workspace = WorkspaceBuilder::new()
    ///     .add_library(Library::new("My Library"))
    ///     .build();
    ///
    /// // Search all options
    /// let results = workspace.search_options("blonde", None);
    ///
    /// // Search within a specific group
    /// let results = workspace.search_options("blonde", Some("Hair"));
    /// ```
    pub fn search_options(&self, query: &str, group_filter: Option<&str>) -> Vec<OptionSearchResult> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let query = query.trim();

        let mut results = Vec::new();

        for library in self.libraries() {
            for group in &library.groups {
                let group_name = &group.name;

                // Skip if group filter is specified and doesn't match
                if let Some(filter) = group_filter
                    && !group_name.eq_ignore_ascii_case(filter)
                {
                    continue;
                }

                let mut matches = Vec::new();

                for option in &group.options {
                    if query.is_empty() {
                        // Return all options with score 0 when query is empty
                        matches.push(OptionMatch {
                            text: option.clone(),
                            score: 0,
                            match_indices: vec![],
                        });
                    } else if let Some((score, indices)) = matcher.fuzzy_indices(option, query) {
                        matches.push(OptionMatch {
                            text: option.clone(),
                            score,
                            match_indices: indices,
                        });
                    }
                }

                if !matches.is_empty() {
                    // Sort matches by score descending
                    matches.sort_by(|a, b| b.score.cmp(&a.score));

                    results.push(OptionSearchResult {
                        library_id: library.id.clone(),
                        library_name: library.name.clone(),
                        group_name: group_name.to_string(),
                        matches,
                    });
                }
            }
        }

        // Sort result groups by their best match score
        results.sort_by(|a, b| {
            let a_best = a.matches.first().map(|m| m.score).unwrap_or(0);
            let b_best = b.matches.first().map(|m| m.score).unwrap_or(0);
            b_best.cmp(&a_best)
        });

        results
    }

    /// Unified search with syntax parsing.
    ///
    /// Supports the following query syntax:
    /// - Plain text (e.g., `blue`) - Search options across all groups
    /// - `@group` or `@group_query` - Search for groups by name, show all options
    /// - `@group/option` - Search for options within groups matching "group"
    /// - `@/option` - Search for options across all groups (same as plain text)
    ///
    /// # Example
    ///
    /// ```
    /// # use promptgen_core::workspace::WorkspaceBuilder;
    /// # use promptgen_core::library::Library;
    /// # use promptgen_core::search::SearchResult;
    /// let workspace = WorkspaceBuilder::new()
    ///     .add_library(Library::new("My Library"))
    ///     .build();
    ///
    /// // Search options across all groups
    /// let results = workspace.search("blue");
    ///
    /// // Search groups by name
    /// let results = workspace.search("@hair");
    ///
    /// // Search options in groups matching "Hair"
    /// let results = workspace.search("@Hair/blonde");
    ///
    /// // Search options across all groups (same as plain text)
    /// let results = workspace.search("@/blue");
    /// ```
    pub fn search(&self, query: &str) -> SearchResult {
        let query = query.trim();

        // Check if query starts with @
        if let Some(rest) = query.strip_prefix('@') {
            // Check for / to determine if searching options within a group
            if let Some(slash_pos) = rest.find('/') {
                let group_part = &rest[..slash_pos];
                let option_part = &rest[slash_pos + 1..];

                if group_part.is_empty() {
                    // @/option - search all options (same as plain text)
                    SearchResult::Options(self.search_options(option_part, None))
                } else {
                    // @group/option - search options in groups matching group_part
                    // First find matching groups, then search their options
                    SearchResult::Options(self.search_options_in_matching_groups(group_part, option_part))
                }
            } else {
                // @group - search groups by name
                SearchResult::Groups(self.search_groups(rest))
            }
        } else {
            // No @ prefix - search options across all groups
            SearchResult::Options(self.search_options(query, None))
        }
    }

    /// Search for options within groups that match a fuzzy group filter.
    ///
    /// This is used for the `@group/option` syntax where we first fuzzy-match
    /// group names, then search for options within those matched groups.
    pub fn search_options_in_matching_groups(
        &self,
        group_query: &str,
        option_query: &str,
    ) -> Vec<OptionSearchResult> {
        let group_matcher = SkimMatcherV2::default().ignore_case();
        let option_matcher = SkimMatcherV2::default().ignore_case();
        let group_query = group_query.trim();
        let option_query = option_query.trim();

        let mut results = Vec::new();

        for library in self.libraries() {
            for group in &library.groups {
                let group_name = &group.name;

                // First check if the group name matches the group query
                let group_matches = group_query.is_empty()
                    || group_matcher.fuzzy_match(group_name, group_query).is_some();

                if !group_matches {
                    continue;
                }

                // Now search options within this matching group
                let mut matches = Vec::new();

                for option in &group.options {
                    if option_query.is_empty() {
                        matches.push(OptionMatch {
                            text: option.clone(),
                            score: 0,
                            match_indices: vec![],
                        });
                    } else if let Some((score, indices)) = option_matcher.fuzzy_indices(option, option_query) {
                        matches.push(OptionMatch {
                            text: option.clone(),
                            score,
                            match_indices: indices,
                        });
                    }
                }

                if !matches.is_empty() {
                    matches.sort_by(|a, b| b.score.cmp(&a.score));

                    results.push(OptionSearchResult {
                        library_id: library.id.clone(),
                        library_name: library.name.clone(),
                        group_name: group_name.to_string(),
                        matches,
                    });
                }
            }
        }

        // Sort result groups by their best match score
        results.sort_by(|a, b| {
            let a_best = a.matches.first().map(|m| m.score).unwrap_or(0);
            let b_best = b.matches.first().map(|m| m.score).unwrap_or(0);
            b_best.cmp(&a_best)
        });

        results
    }
}
