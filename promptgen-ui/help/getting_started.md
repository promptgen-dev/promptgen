# Getting Started with PromptGen

Welcome to PromptGen! This guide will help you get started with creating and managing reusable prompt templates.

## Core Concepts

### Libraries

A **library** is a collection of prompts and variables stored in a single file. Think of it as a knowledge base for a specific domain or project.

**Example Use cases:**

- **Portrait Library** - Variables and prompts for generating character portraits
- **Coding Library** - Commonly used context snippets and prompts for code generation ("Please ask clarifying questions!")

**Managing libraries:**

- Create new libraries via **File > Create Library**
- Open existing libraries via **File > Open Library**
- Edit library name via **File > Edit Library**

### Variables

Variables are reusable options that you can reference in prompts. They help you:

- Maintain consistency across prompts
- Quickly generate variations
- Organize related concepts

Variables may have one value which can be useful for referencing a common phrase often using the `@VariableName` syntax in your prompt. Variables can also have multiple values which will be randomly selected when using the `@VariableName` syntax. This can be useful for A/B testing, generating variations, or adding randomness to your prompts. Grouping related options into variables also helps you create Slots which we'll explore later.

**Example: Style variable**

```
oil painting
watercolor
digital art
pencil sketch
photograph
```

By default, variables are separated by newlines, however you can make multiline strings by surrounding them with `---`

**Example: Persona Variable**

```
---
You are an expert accountant specializing in small business finances.

Your advice is practical, clear, and tailored to entrepreneurs.
---
You are a creative writing tutor who helps students develop their storytelling skills.
```

### Prompts

Prompts in PromptGen can be as simple as plain text you want to save and reuse later, or they can be used as templates that reference Variables, Slots, and other advanced features to templatize and share your workflows.

## Practical Examples

### Example 1: Character Portrait Generator

**Variables:**

```
Gender: woman, man, person
Age: young, middle-aged, elderly
Expression: serene, confident, mysterious, joyful
HairStyle: long flowing hair, short cropped hair, braided hair
```

**Prompt:**

```
Portrait of a @Age @Gender with @HairStyle,
@Expression expression, @Style, @Lighting
```

**Sample outputs:**

- "Portrait of a young woman with long flowing hair, mysterious expression, oil painting, golden hour sunlight"
- "Portrait of an elderly man with short cropped hair, serene expression, photograph, soft diffused light"

### Example 2: Code Review Assistant

**Variables:**

```
Language:
  Python
  TypeScript
  Rust
  Go

ReviewFocus:
  security vulnerabilities
  performance optimizations
  code readability
  error handling

Tone:
  concise and direct
  detailed with explanations
  educational for junior developers
```

**Prompt:**

```
Review the following @Language code with a focus on @ReviewFocus.

Be @Tone in your feedback. Include specific line references
and suggest concrete improvements.

```

**Sample outputs:**

- "Review the following Rust code with a focus on error handling. Be concise and direct in your feedback..."
- "Review the following Python code with a focus on security vulnerabilities. Be educational for junior developers..."

## Working with Slots

When you use variables in a prompt, they appear as **slots** below the editor. Slots let you:

- **Select specific options** instead of random selection
- **Pick multiple values** for variety
- **Search and filter** through large option lists
- **Manually enter** custom values

### Slot Configuration

- **Separator**: How multiple values are joined (default: ", ")
- **Suffix**: Text added after the slot value

## Quick Start Workflow

1. **Create a library** - File > Create Library, name it for your project
2. **Add variables** - Click the Variables tab in sidebar, then "+ New"
3. **Create a prompt** - Click "+ New" in the tab bar
4. **Write your template** - Use `@VariableName` syntax
5. **Configure slots** - Select or randomize values in the Slots section
6. **Preview and iterate** - Use the Preview panel to see results

## Autocomplete Keyboard Shortcuts

| Shortcut       | Action                         |
| -------------- | ------------------------------ |
| `Ctrl+Space`   | Toggle autocomplete            |
| `Up/Down`      | Choose options                 |
| `Escape`       | Close picker/dialog            |
| `Tab or Enter` | Accept autocomplete suggestion |

## Tips

- Start with broad variables (Tone, Persona, Format) that work across many prompts, then add specific ones as needed.
- Use the search in the slot picker to quickly find options in large variables.
- Export your variables to share them across libraries or with others.

## Variable Organization

For complex libraries, consider organizing variables by category:

**For coding prompts:** Language, Framework, ReviewFocus, OutputFormat

**For creative prompts:** Style, Mood, Subject, Medium

**For workflow prompts:** Persona, Tone, Audience, Context
