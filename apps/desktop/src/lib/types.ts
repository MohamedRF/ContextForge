export interface Project {
  id: number;
  name: string;
  path: string;
  available: boolean;
}
export interface Status {
  repository: {
    initialized: boolean;
    git_detected: boolean;
    max_file_bytes: number;
  };
  index: {
    state: string;
    files: number;
    freshness: string;
    schema_version: number | null;
  };
  version: string;
}
export interface Progress {
  project_id: number;
  operation_id: string;
  phase: string;
  processed: number;
  total: number;
  elapsed_ms: number;
}
export interface Span {
  start_line: number;
  end_line: number;
  start_byte: number;
  end_byte: number;
}
export interface Hit {
  path: string;
  symbol: string | null;
  kind: string | null;
  snippet: string;
  span: Span;
  method: string;
  snippet_truncated: boolean;
}
export interface SearchResults {
  results: Hit[];
  truncated: boolean;
  next_offset: number | null;
  freshness: string;
}
export interface ProjectMap {
  files: { path: string; language: string; bytes: number }[];
  truncated: boolean;
}
export interface Outline {
  symbols: { id: string; name: string; kind: string; span: Span }[];
  occurrences: { kind: string; text: string; span: Span }[];
  has_errors: boolean;
  truncated: boolean;
}
export interface FilePreview {
  path: string;
  text: string;
  offset: number;
  lines: number;
  has_more: boolean;
  outline: Outline;
}
export interface Node {
  id: string;
  path: string;
  qualified_name: string;
  kind: string;
  span: Span;
}
export interface Visit {
  node: Node;
  depth: number;
  status: string;
}
export interface Edge {
  from: string;
  to: string | null;
  kind: string;
  status: string;
  path: string;
  span: Span | null;
  text: string;
  method: string;
}
export interface Graph {
  nodes: Visit[];
  edges: Edge[];
  truncated: boolean;
  partial_syntax: boolean;
}
export interface Impact {
  direct_changes: {
    old_path: string | null;
    new_path: string | null;
    kind: string;
    hunks: {
      old_start: number;
      old_lines: number;
      new_start: number;
      new_lines: number;
    }[];
    current_symbols: Node[];
  }[];
  resolved_dependents: Visit[];
  heuristic_dependents: Visit[];
  potentially_affected_tests: Visit[];
  dependency_evidence: Edge[];
  uncertainty: string;
  truncated: boolean;
}
export interface ContextPackage {
  query: string;
  results: {
    path: string;
    symbol: string | null;
    snippet: string;
    span: Span;
    reason: string;
    confidence: string;
  }[];
  estimated_tokens: number;
  budget_tokens: number;
  truncated: boolean;
  freshness: string;
}
export interface Integration {
  operation_id: string;
  harness: string;
  configuration: string;
  destination: string;
  instructions: string;
  verification: string;
}
export type Theme = "system" | "dark" | "light";
