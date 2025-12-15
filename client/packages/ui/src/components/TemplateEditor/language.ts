import { StreamLanguage, StringStream } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

/**
 * Token types for promptgen syntax.
 * Maps to Lezer highlight tags for theming.
 */
interface PromptgenState {
  inInlineOptions: boolean;
  inSlot: boolean;
}

/**
 * StreamLanguage tokenizer for promptgen template syntax.
 *
 * Syntax elements:
 * - @GroupName or @"Group Name" - Library references
 * - {option1|option2|option3} - Inline options
 * - {{ slot_name }} - Slots for user input
 * - # comment - Line comments
 */
function tokenize(stream: StringStream, state: PromptgenState): string | null {
  // Handle comments (# at start of line or after whitespace)
  if (stream.sol() && stream.match(/^#.*/)) {
    return "comment";
  }

  // Check for comment mid-line (after whitespace)
  if (stream.match(/\s+#.*/)) {
    return "comment";
  }

  // Handle slots: {{ slot_name }}
  if (stream.match("{{")) {
    state.inSlot = true;
    return "brace";
  }

  if (state.inSlot) {
    if (stream.match("}}")) {
      state.inSlot = false;
      return "brace";
    }
    // Slot name content
    if (stream.match(/[a-zA-Z_][a-zA-Z0-9_]*/)) {
      return "variableName";
    }
    // Skip whitespace inside slot
    if (stream.match(/\s+/)) {
      return null;
    }
    stream.next();
    return null;
  }

  // Handle inline options: {option1|option2|option3}
  if (stream.match("{") && !stream.match("{", false)) {
    state.inInlineOptions = true;
    return "brace";
  }

  if (state.inInlineOptions) {
    if (stream.match("}")) {
      state.inInlineOptions = false;
      return "brace";
    }
    if (stream.match("|")) {
      return "punctuation";
    }
    // Option content (anything except | and })
    if (stream.match(/[^|}]+/)) {
      return "string";
    }
    stream.next();
    return null;
  }

  // Handle library references: @GroupName or @"Group Name"
  if (stream.match("@")) {
    // Check for quoted reference
    if (stream.match('"')) {
      // Read until closing quote
      while (!stream.eol()) {
        if (stream.next() === '"') {
          break;
        }
      }
      return "typeName";
    }
    // Unquoted reference - read identifier
    if (stream.match(/[a-zA-Z_][a-zA-Z0-9_]*/)) {
      return "typeName";
    }
    return "typeName";
  }

  // Plain text - consume until we hit a special character
  if (stream.match(/[^@{#\s]+/)) {
    return null;
  }

  // Single character advancement
  stream.next();
  return null;
}

/**
 * Create initial state for tokenizer.
 */
function startState(): PromptgenState {
  return {
    inInlineOptions: false,
    inSlot: false,
  };
}

/**
 * Copy state for tokenizer.
 */
function copyState(state: PromptgenState): PromptgenState {
  return {
    inInlineOptions: state.inInlineOptions,
    inSlot: state.inSlot,
  };
}

/**
 * Promptgen template language definition for CodeMirror.
 */
export const promptgenLanguage = StreamLanguage.define<PromptgenState>({
  token: tokenize,
  startState,
  copyState,
  tokenTable: {
    comment: t.comment,
    brace: t.brace,
    punctuation: t.punctuation,
    string: t.string,
    variableName: t.definition(t.variableName),
    typeName: t.typeName,
  },
});
