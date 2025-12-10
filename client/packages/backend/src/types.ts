// Library types
export interface LibrarySummary {
  id: string;
  name: string;
  path: string;
  templateCount: number;
  lastModified: string;
}

export interface Library {
  id: string;
  name: string;
  path: string;
  templates: Template[];
  wildcards: Record<string, string[]>;
}

export interface Template {
  id: string;
  name: string;
  content: string;
  bindings?: Record<string, BindingValue>;
}

export interface PromptGroup {
  name: string;
  options: string[];
}

export type BindingValue =
  | { type: "literal"; value: string }
  | { type: "wildcard"; path: string }
  | { type: "template"; templateId: string };

// Parse types
export interface ParseResult {
  success: boolean;
  ast?: AstNode;
  errors?: ParseError[];
}

export interface AstNode {
  kind: string;
  span: Span;
  children?: AstNode[];
  value?: string;
}

export interface Span {
  start: number;
  end: number;
}

export interface ParseError {
  message: string;
  span: Span;
}

// Render types
export interface RenderInput {
  templateId: string;
  libraryId: string;
  bindings?: Record<string, string>;
  seed?: number;
}

export interface RenderResult {
  success: boolean;
  output?: string;
  error?: string;
}

// Auth types (for future cloud features)
export interface AuthState {
  user: User | null;
  features: FeatureFlags;
}

export interface User {
  id: string;
  email: string;
  name?: string;
}

export interface FeatureFlags {
  cloud_sync: boolean;
  team_libraries: boolean;
  analytics: boolean;
}

export interface LoginInput {
  email: string;
  password: string;
}
