//! Syntax highlighting for promptgen prompts using egui LayoutJob.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, TextStyle};
use promptgen_core::{Node, ParseResult};

use crate::theme::syntax;

/// Resolved syntax colors for the current theme
#[derive(Clone)]
struct SyntaxColors {
    text: Color32,
    reference: Color32,
    slot: Color32,
    option: Color32,
    brace: Color32,
    comment: Color32,
}

impl SyntaxColors {
    fn from_context(ctx: &egui::Context) -> Self {
        Self {
            text: syntax::text(ctx),
            reference: syntax::reference(ctx),
            slot: syntax::slot(ctx),
            option: syntax::option(ctx),
            brace: syntax::brace(ctx),
            comment: syntax::comment(ctx),
        }
    }
}

/// Token types for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// Plain text
    Text,
    /// Library reference (@Name or @"Name")
    Reference,
    /// Slot ({{ name }})
    Slot,
    /// Inline options ({a|b|c})
    Option,
    /// Braces and delimiters
    Brace,
    /// Comment (# ...)
    Comment,
}

impl TokenKind {
    /// Get the color for this token kind from resolved colors
    fn color(self, colors: &SyntaxColors) -> Color32 {
        match self {
            TokenKind::Text => colors.text,
            TokenKind::Reference => colors.reference,
            TokenKind::Slot => colors.slot,
            TokenKind::Option => colors.option,
            TokenKind::Brace => colors.brace,
            TokenKind::Comment => colors.comment,
        }
    }
}

/// Create a highlighted LayoutJob from the editor content and parse result.
pub fn highlight_prompt(
    ctx: &egui::Context,
    text: &str,
    parse_result: Option<&ParseResult>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font_id = TextStyle::Monospace.resolve(&ctx.style());
    let colors = SyntaxColors::from_context(ctx);

    // If we have a successful parse with an AST, use it for accurate highlighting
    if let Some(result) = parse_result
        && let Some(ast) = &result.ast
    {
        highlight_from_ast(&mut job, text, ast, &font_id, &colors);
        return job;
    }

    // Fallback: simple regex-like highlighting for when parsing fails
    highlight_fallback(&mut job, text, &font_id, &colors);
    job
}

/// Highlight using the parsed AST for accurate token boundaries
fn highlight_from_ast(
    job: &mut LayoutJob,
    text: &str,
    ast: &promptgen_core::Prompt,
    font_id: &FontId,
    colors: &SyntaxColors,
) {
    let text_len = text.len();
    let mut last_end = 0;

    for (node, span) in &ast.nodes {
        // Bounds check: if span is out of bounds, fall back to simple highlighting
        if span.start > text_len || span.end > text_len || span.start > span.end {
            // AST is stale, fall back to fallback highlighting for remaining text
            if last_end < text_len {
                highlight_fallback_range(job, &text[last_end..], font_id, colors);
            }
            return;
        }

        // Add any gap before this node as plain text (shouldn't happen normally)
        if span.start > last_end && last_end < text_len {
            let gap_end = span.start.min(text_len);
            append_token(
                job,
                &text[last_end..gap_end],
                TokenKind::Text,
                font_id,
                colors,
            );
        }

        // Get the original source text for this span
        let node_text = &text[span.clone()];

        match node {
            Node::Text(_) => {
                append_token(job, node_text, TokenKind::Text, font_id, colors);
            }
            Node::LibraryRef(_) => {
                // Highlight @ symbol and the reference name
                append_token(job, node_text, TokenKind::Reference, font_id, colors);
            }
            Node::SlotBlock(_) => {
                // Highlight entire slot including {{ }}
                append_token(job, node_text, TokenKind::Slot, font_id, colors);
            }
            Node::InlineOptions(_) => {
                // Highlight inline options with brace coloring for { and }
                highlight_inline_options(job, node_text, font_id, colors);
            }
            Node::Comment(_) => {
                append_token(job, node_text, TokenKind::Comment, font_id, colors);
            }
        }

        last_end = span.end;
    }

    // Add any remaining text after the last node
    if last_end < text_len {
        append_token(job, &text[last_end..], TokenKind::Text, font_id, colors);
    }
}

/// Fallback highlighting for a range when AST is stale
fn highlight_fallback_range(
    job: &mut LayoutJob,
    text: &str,
    font_id: &FontId,
    colors: &SyntaxColors,
) {
    highlight_fallback(job, text, font_id, colors);
}

/// Highlight inline options with colored braces and pipe separators
fn highlight_inline_options(
    job: &mut LayoutJob,
    text: &str,
    font_id: &FontId,
    colors: &SyntaxColors,
) {
    // Text format: {option1|option2|option3}
    if text.starts_with('{') && text.ends_with('}') {
        // Opening brace
        append_token(job, "{", TokenKind::Brace, font_id, colors);

        // Content between braces
        let inner = &text[1..text.len() - 1];
        let parts: Vec<&str> = inner.split('|').collect();

        for (i, part) in parts.iter().enumerate() {
            append_token(job, part, TokenKind::Option, font_id, colors);
            if i < parts.len() - 1 {
                append_token(job, "|", TokenKind::Brace, font_id, colors);
            }
        }

        // Closing brace
        append_token(job, "}", TokenKind::Brace, font_id, colors);
    } else {
        // Fallback if format is unexpected
        append_token(job, text, TokenKind::Option, font_id, colors);
    }
}

/// Fallback highlighting when parsing fails - uses simple pattern matching
fn highlight_fallback(job: &mut LayoutJob, text: &str, font_id: &FontId, colors: &SyntaxColors) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current_text = String::new();

    while i < chars.len() {
        let c = chars[i];

        match c {
            '@' => {
                // Flush current text
                if !current_text.is_empty() {
                    append_token(job, &current_text, TokenKind::Text, font_id, colors);
                    current_text.clear();
                }

                // Check for quoted reference @"..."
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    let start = i;
                    i += 2; // Skip @"
                    while i < chars.len() && chars[i] != '"' {
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1; // Skip closing "
                    }
                    let ref_text: String = chars[start..i].iter().collect();
                    append_token(job, &ref_text, TokenKind::Reference, font_id, colors);
                } else {
                    // Simple reference @Name
                    let start = i;
                    i += 1; // Skip @
                    while i < chars.len()
                        && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-')
                    {
                        i += 1;
                    }
                    let ref_text: String = chars[start..i].iter().collect();
                    append_token(job, &ref_text, TokenKind::Reference, font_id, colors);
                }
                continue;
            }
            '{' => {
                // Flush current text
                if !current_text.is_empty() {
                    append_token(job, &current_text, TokenKind::Text, font_id, colors);
                    current_text.clear();
                }

                // Check for slot {{ ... }}
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    let start = i;
                    i += 2; // Skip {{
                    while i < chars.len() {
                        if i + 1 < chars.len() && chars[i] == '}' && chars[i + 1] == '}' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    let slot_text: String = chars[start..i].iter().collect();
                    append_token(job, &slot_text, TokenKind::Slot, font_id, colors);
                } else {
                    // Inline options { ... }
                    let start = i;
                    let mut depth = 1;
                    i += 1;
                    while i < chars.len() && depth > 0 {
                        if chars[i] == '{' {
                            depth += 1;
                        } else if chars[i] == '}' {
                            depth -= 1;
                        }
                        i += 1;
                    }
                    let opt_text: String = chars[start..i].iter().collect();
                    highlight_inline_options(job, &opt_text, font_id, colors);
                }
                continue;
            }
            '#' => {
                // Flush current text
                if !current_text.is_empty() {
                    append_token(job, &current_text, TokenKind::Text, font_id, colors);
                    current_text.clear();
                }

                // Comment to end of line
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                let comment_text: String = chars[start..i].iter().collect();
                append_token(job, &comment_text, TokenKind::Comment, font_id, colors);
                continue;
            }
            _ => {
                current_text.push(c);
            }
        }
        i += 1;
    }

    // Flush remaining text
    if !current_text.is_empty() {
        append_token(job, &current_text, TokenKind::Text, font_id, colors);
    }
}

/// Append a token with the appropriate styling to the LayoutJob
fn append_token(
    job: &mut LayoutJob,
    text: &str,
    kind: TokenKind,
    font_id: &FontId,
    colors: &SyntaxColors,
) {
    if text.is_empty() {
        return;
    }

    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color: kind.color(colors),
            ..Default::default()
        },
    );
}
