import { EditorView } from "@codemirror/view";

/**
 * Custom CodeMirror theme that integrates with shadcn/ui and Tailwind CSS.
 * Uses CSS variables for theming to support light/dark mode.
 */
export const editorTheme = EditorView.theme({
  // Root editor container
  "&": {
    height: "100%",
    fontSize: "14px",
    backgroundColor: "transparent",
  },

  // Scroller container
  ".cm-scroller": {
    fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace",
    lineHeight: "1.6",
    overflow: "auto",
  },

  // Content area
  ".cm-content": {
    padding: "16px 0",
    caretColor: "hsl(var(--foreground))",
  },

  // Line wrapping
  ".cm-line": {
    padding: "0 16px",
  },

  // Gutter (line numbers)
  ".cm-gutters": {
    backgroundColor: "hsl(var(--muted) / 0.5)",
    borderRight: "1px solid hsl(var(--border))",
    color: "hsl(var(--muted-foreground))",
  },

  ".cm-gutter": {
    minWidth: "48px",
  },

  ".cm-lineNumbers .cm-gutterElement": {
    padding: "0 8px 0 16px",
    minWidth: "32px",
    textAlign: "right",
  },

  // Active line highlighting
  ".cm-activeLine": {
    backgroundColor: "hsl(var(--accent) / 0.3)",
  },

  ".cm-activeLineGutter": {
    backgroundColor: "hsl(var(--accent) / 0.5)",
  },

  // Selection
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
    backgroundColor: "hsl(var(--primary) / 0.2)",
  },

  // Cursor
  ".cm-cursor": {
    borderLeftColor: "hsl(var(--foreground))",
    borderLeftWidth: "2px",
  },

  // Focus outline
  "&.cm-focused": {
    outline: "none",
  },

  // Matching brackets
  ".cm-matchingBracket": {
    backgroundColor: "hsl(var(--primary) / 0.3)",
    outline: "1px solid hsl(var(--primary) / 0.5)",
  },

  // Search highlight
  ".cm-searchMatch": {
    backgroundColor: "hsl(var(--warning) / 0.3)",
  },

  ".cm-searchMatch.cm-searchMatch-selected": {
    backgroundColor: "hsl(var(--warning) / 0.5)",
  },

  // Selection match highlight
  ".cm-selectionMatch": {
    backgroundColor: "hsl(var(--primary) / 0.15)",
  },

  // Placeholder text
  ".cm-placeholder": {
    color: "hsl(var(--muted-foreground))",
    fontStyle: "italic",
  },

  // Tooltips (for autocomplete, etc.)
  ".cm-tooltip": {
    backgroundColor: "hsl(var(--popover))",
    border: "1px solid hsl(var(--border))",
    borderRadius: "6px",
    boxShadow: "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
  },

  ".cm-tooltip-autocomplete": {
    "& > ul": {
      fontFamily: "inherit",
      maxHeight: "200px",
    },
    "& > ul > li": {
      padding: "4px 8px",
      borderRadius: "4px",
      margin: "2px 4px",
    },
    "& > ul > li[aria-selected]": {
      backgroundColor: "hsl(var(--accent))",
      color: "hsl(var(--accent-foreground))",
    },
  },

  // Panel (search panel, etc.)
  ".cm-panel": {
    backgroundColor: "hsl(var(--muted))",
    borderBottom: "1px solid hsl(var(--border))",
    padding: "8px 12px",
  },

  ".cm-panel input": {
    backgroundColor: "hsl(var(--background))",
    border: "1px solid hsl(var(--border))",
    borderRadius: "4px",
    padding: "4px 8px",
    fontSize: "13px",
  },

  ".cm-panel button": {
    backgroundColor: "hsl(var(--primary))",
    color: "hsl(var(--primary-foreground))",
    border: "none",
    borderRadius: "4px",
    padding: "4px 12px",
    fontSize: "13px",
    cursor: "pointer",
  },
});
