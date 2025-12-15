import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

/**
 * Syntax highlighting styles for promptgen templates.
 * Uses CSS variables for theme compatibility with light/dark mode.
 */
export const promptgenHighlightStyle = HighlightStyle.define([
  // Comments: # comment text
  {
    tag: t.comment,
    color: "hsl(var(--muted-foreground))",
    fontStyle: "italic",
  },

  // Library references: @GroupName, @"Group Name"
  {
    tag: t.typeName,
    color: "hsl(var(--primary))",
    fontWeight: "600",
  },

  // Slot names: {{ slot_name }}
  {
    tag: t.definition(t.variableName),
    color: "hsl(142 71% 45%)", // Green for slots
    fontWeight: "500",
  },

  // Inline option strings: {happy|sad|excited}
  {
    tag: t.string,
    color: "hsl(25 95% 53%)", // Orange for options
  },

  // Braces and brackets: { } {{ }}
  {
    tag: t.brace,
    color: "hsl(var(--muted-foreground))",
    fontWeight: "600",
  },

  // Pipe separator in inline options: |
  {
    tag: t.punctuation,
    color: "hsl(var(--muted-foreground))",
    fontWeight: "600",
  },
]);

/**
 * Extension that applies the promptgen highlight style.
 */
export const promptgenHighlighting = syntaxHighlighting(promptgenHighlightStyle);
