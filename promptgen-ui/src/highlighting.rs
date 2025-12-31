//! Syntax highlighting for promptgen prompts using egui LayoutJob.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, TextStyle};
use promptgen_core::{Node, ParseResult, PickSource, SlotBlock, SlotKind};

use crate::theme::{self, Theme};

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
    /// Get the color for this token kind from the theme
    fn color(self, theme: &Theme) -> Color32 {
        match self {
            TokenKind::Text => theme.text,
            TokenKind::Reference => theme.reference,
            TokenKind::Slot => theme.slot,
            TokenKind::Option => theme.option,
            TokenKind::Brace => theme.brace,
            TokenKind::Comment => theme.comment,
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
    let theme = theme::current(ctx);

    // If we have a successful parse with an AST, use it for accurate highlighting
    if let Some(result) = parse_result
        && let Some(ast) = &result.ast
    {
        highlight_from_ast(&mut job, text, ast, &font_id, &theme);
        return job;
    }

    // No AST available - return plain text with default color
    append_token(&mut job, text, TokenKind::Text, &font_id, &theme);
    job
}

/// Highlight using the parsed AST for accurate token boundaries
fn highlight_from_ast(
    job: &mut LayoutJob,
    text: &str,
    ast: &promptgen_core::Prompt,
    font_id: &FontId,
    theme: &Theme,
) {
    let text_len = text.len();
    let mut last_end = 0;

    for (node, span) in &ast.nodes {
        // Bounds check: if span is out of bounds, append remaining text as plain
        if span.start > text_len || span.end > text_len || span.start > span.end {
            // AST is stale, show remaining text as plain
            if last_end < text_len {
                append_token(job, &text[last_end..], TokenKind::Text, font_id, theme);
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
                theme,
            );
        }

        // Get the original source text for this span
        let node_text = &text[span.clone()];

        match node {
            Node::Text(_) => {
                append_token(job, node_text, TokenKind::Text, font_id, theme);
            }
            Node::LibraryRef(_) => {
                // Highlight @ symbol and the reference name
                append_token(job, node_text, TokenKind::Reference, font_id, theme);
            }
            Node::SlotBlock(slot_block) => {
                // Highlight slot with recursive highlighting for variable references
                highlight_slot_block(job, node_text, slot_block, span.start, font_id, theme);
            }
            Node::InlineOptions(_) => {
                // Highlight inline options with brace coloring for { and }
                highlight_inline_options(job, node_text, font_id, theme);
            }
            Node::Comment(_) => {
                append_token(job, node_text, TokenKind::Comment, font_id, theme);
            }
        }

        last_end = span.end;
    }

    // Add any remaining text after the last node
    if last_end < text_len {
        append_token(job, &text[last_end..], TokenKind::Text, font_id, theme);
    }
}

/// Highlight a slot block with recursive highlighting for variable references inside pick() sources
fn highlight_slot_block(
    job: &mut LayoutJob,
    text: &str,
    slot_block: &SlotBlock,
    block_start: usize,
    font_id: &FontId,
    theme: &Theme,
) {
    // For pick slots, we need to highlight variable references inside
    if let SlotKind::Pick(pick_slot) = &slot_block.kind.0 {
        // Collect all variable reference spans (relative to block_start)
        let mut ref_spans: Vec<(usize, usize)> = Vec::new();
        for (source, span) in &pick_slot.sources {
            if matches!(source, PickSource::VariableRef(_)) {
                // Convert span to be relative to the slot block text
                let rel_start = span.start.saturating_sub(block_start);
                let rel_end = span.end.saturating_sub(block_start);
                if rel_end <= text.len() {
                    ref_spans.push((rel_start, rel_end));
                }
            }
        }

        // Sort spans by start position
        ref_spans.sort_by_key(|(start, _)| *start);

        // Highlight with interleaved slot color and reference color
        let mut last_end = 0;
        for (ref_start, ref_end) in ref_spans {
            // Slot-colored text before this reference
            if ref_start > last_end {
                append_token(
                    job,
                    &text[last_end..ref_start],
                    TokenKind::Slot,
                    font_id,
                    theme,
                );
            }
            // Reference-colored text
            if ref_end > ref_start && ref_end <= text.len() {
                append_token(
                    job,
                    &text[ref_start..ref_end],
                    TokenKind::Reference,
                    font_id,
                    theme,
                );
            }
            last_end = ref_end;
        }
        // Remaining slot-colored text
        if last_end < text.len() {
            append_token(job, &text[last_end..], TokenKind::Slot, font_id, theme);
        }
    } else {
        // Textarea slot - just highlight the whole thing as slot color
        append_token(job, text, TokenKind::Slot, font_id, theme);
    }
}

/// Highlight inline options with colored braces and pipe separators
fn highlight_inline_options(job: &mut LayoutJob, text: &str, font_id: &FontId, theme: &Theme) {
    // Text format: {option1|option2|option3}
    if text.starts_with('{') && text.ends_with('}') {
        // Opening brace
        append_token(job, "{", TokenKind::Brace, font_id, theme);

        // Content between braces
        let inner = &text[1..text.len() - 1];
        let parts: Vec<&str> = inner.split('|').collect();

        for (i, part) in parts.iter().enumerate() {
            append_token(job, part, TokenKind::Option, font_id, theme);
            if i < parts.len() - 1 {
                append_token(job, "|", TokenKind::Brace, font_id, theme);
            }
        }

        // Closing brace
        append_token(job, "}", TokenKind::Brace, font_id, theme);
    } else {
        // Fallback if format is unexpected
        append_token(job, text, TokenKind::Option, font_id, theme);
    }
}

/// Append a token with the appropriate styling to the LayoutJob
fn append_token(job: &mut LayoutJob, text: &str, kind: TokenKind, font_id: &FontId, theme: &Theme) {
    if text.is_empty() {
        return;
    }

    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color: kind.color(theme),
            ..Default::default()
        },
    );
}
