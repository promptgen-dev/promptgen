//! Fuzzy search functionality for libraries.
//!
//! Provides fuzzy matching for variables and options within a library.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::library::Library;

/// Result of a fuzzy search for a variable.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariableSearchResult {
    /// Name of the matched variable
    pub variable_name: String,
    /// All options in this variable
    pub options: Vec<String>,
    /// Raw fuzzy match score (higher is better)
    pub score: i64,
    /// Indices of matched characters in the variable name
    pub match_indices: Vec<usize>,
}

/// A single option match within a variable.
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
    /// Name of the variable containing matched options
    pub variable_name: String,
    /// Matched options with their scores
    pub matches: Vec<OptionMatch>,
}

/// Unified search result that can contain either variables or options.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SearchResult {
    /// Variable search results
    Variables(Vec<VariableSearchResult>),
    /// Option search results
    Options(Vec<OptionSearchResult>),
}

impl Library {
    /// Search for variables matching the query.
    ///
    /// Returns all variables if query is empty. Results are sorted by score (highest first).
    /// Search is case-insensitive.
    ///
    /// # Example
    ///
    /// ```
    /// # use promptgen_core::library::Library;
    /// let library = Library::new("My Library");
    /// let results = library.search_variables("hair");
    /// ```
    pub fn search_variables(&self, query: &str) -> Vec<VariableSearchResult> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let query = query.trim();

        let mut results = Vec::new();

        for variable in &self.variables {
            let variable_name = &variable.name;

            if query.is_empty() {
                // Return all variables with score 0 when query is empty
                results.push(VariableSearchResult {
                    variable_name: variable_name.to_string(),
                    options: variable.options.clone(),
                    score: 0,
                    match_indices: vec![],
                });
            } else if let Some((score, indices)) = matcher.fuzzy_indices(variable_name, query) {
                results.push(VariableSearchResult {
                    variable_name: variable_name.to_string(),
                    options: variable.options.clone(),
                    score,
                    match_indices: indices,
                });
            }
        }

        // Sort by score descending (highest first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        results
    }

    /// Search for options matching the query, optionally filtered to a specific variable.
    ///
    /// Returns all options if query is empty. Results are sorted by best match score within each variable.
    /// Search is case-insensitive.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query
    /// * `variable_filter` - Optional variable name to filter results to
    ///
    /// # Example
    ///
    /// ```
    /// # use promptgen_core::library::Library;
    /// let library = Library::new("My Library");
    ///
    /// // Search all options
    /// let results = library.search_options("blonde", None);
    ///
    /// // Search within a specific variable
    /// let results = library.search_options("blonde", Some("Hair"));
    /// ```
    pub fn search_options(&self, query: &str, variable_filter: Option<&str>) -> Vec<OptionSearchResult> {
        let matcher = SkimMatcherV2::default().ignore_case();
        let query = query.trim();

        let mut results = Vec::new();

        for variable in &self.variables {
            let variable_name = &variable.name;

            // Skip if variable filter is specified and doesn't match
            if let Some(filter) = variable_filter
                && !variable_name.eq_ignore_ascii_case(filter)
            {
                continue;
            }

            let mut matches = Vec::new();

            for option in &variable.options {
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
                    variable_name: variable_name.to_string(),
                    matches,
                });
            }
        }

        // Sort result variables by their best match score
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
    /// - Plain text (e.g., `blue`) - Search options across all variables
    /// - `@variable` or `@variable_query` - Search for variables by name, show all options
    /// - `@variable/option` - Search for options within variables matching "variable"
    /// - `@/option` - Search for options across all variables (same as plain text)
    ///
    /// # Example
    ///
    /// ```
    /// # use promptgen_core::library::Library;
    /// # use promptgen_core::search::SearchResult;
    /// let library = Library::new("My Library");
    ///
    /// // Search options across all variables
    /// let results = library.search("blue");
    ///
    /// // Search variables by name
    /// let results = library.search("@hair");
    ///
    /// // Search options in variables matching "Hair"
    /// let results = library.search("@Hair/blonde");
    ///
    /// // Search options across all variables (same as plain text)
    /// let results = library.search("@/blue");
    /// ```
    pub fn search(&self, query: &str) -> SearchResult {
        let query = query.trim();

        // Check if query starts with @
        if let Some(rest) = query.strip_prefix('@') {
            // Check for / to determine if searching options within a variable
            if let Some(slash_pos) = rest.find('/') {
                let variable_part = &rest[..slash_pos];
                let option_part = &rest[slash_pos + 1..];

                if variable_part.is_empty() {
                    // @/option - search all options (same as plain text)
                    SearchResult::Options(self.search_options(option_part, None))
                } else {
                    // @variable/option - search options in variables matching variable_part
                    // First find matching variables, then search their options
                    SearchResult::Options(self.search_options_in_matching_variables(variable_part, option_part))
                }
            } else {
                // @variable - search variables by name
                SearchResult::Variables(self.search_variables(rest))
            }
        } else {
            // No @ prefix - search options across all variables
            SearchResult::Options(self.search_options(query, None))
        }
    }

    /// Search for options within variables that match a fuzzy variable filter.
    ///
    /// This is used for the `@variable/option` syntax where we first fuzzy-match
    /// variable names, then search for options within those matched variables.
    pub fn search_options_in_matching_variables(
        &self,
        variable_query: &str,
        option_query: &str,
    ) -> Vec<OptionSearchResult> {
        let variable_matcher = SkimMatcherV2::default().ignore_case();
        let option_matcher = SkimMatcherV2::default().ignore_case();
        let variable_query = variable_query.trim();
        let option_query = option_query.trim();

        let mut results = Vec::new();

        for variable in &self.variables {
            let variable_name = &variable.name;

            // First check if the variable name matches the variable query
            let variable_matches = variable_query.is_empty()
                || variable_matcher.fuzzy_match(variable_name, variable_query).is_some();

            if !variable_matches {
                continue;
            }

            // Now search options within this matching variable
            let mut matches = Vec::new();

            for option in &variable.options {
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
                    variable_name: variable_name.to_string(),
                    matches,
                });
            }
        }

        // Sort result variables by their best match score
        results.sort_by(|a, b| {
            let a_best = a.matches.first().map(|m| m.score).unwrap_or(0);
            let b_best = b.matches.first().map(|m| m.score).unwrap_or(0);
            b_best.cmp(&a_best)
        });

        results
    }
}
