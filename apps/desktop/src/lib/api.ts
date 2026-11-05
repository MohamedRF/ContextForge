import { invoke } from "@tauri-apps/api/core";
import type {
  ContextPackage,
  FilePreview,
  Graph,
  Impact,
  Integration,
  Project,
  ProjectMap,
  SearchResults,
  Status,
  Theme,
} from "./types";

// Bounds mirror the Rust engines. Clamp numerics for the UI; throw on invalid
// strings so callers surface a clear Failure instead of a generic backend error.
export function clampBudget(value: number): number {
  if (!Number.isFinite(value)) return 4000;
  return Math.min(32000, Math.max(256, Math.trunc(value)));
}
export function clampDepth(value: number): number {
  if (!Number.isFinite(value)) return 2;
  return Math.min(8, Math.max(0, Math.trunc(value)));
}
export function clampOffset(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(10000, Math.max(0, Math.trunc(value)));
}
function hasControlCharacters(value: string): boolean {
  for (const char of value) {
    const code = char.codePointAt(0) ?? 0;
    if (code < 0x20 || code === 0x7f) return true;
  }
  return false;
}
export function normalizeQuery(query: string): string {
  const trimmed = query.trim();
  if (trimmed.length === 0 || trimmed.length > 256) {
    throw new Error("Query must be 1–256 characters");
  }
  if (hasControlCharacters(trimmed)) {
    throw new Error("Query must not contain control characters");
  }
  return trimmed;
}
export function normalizeSelector(selector: string): string {
  const trimmed = selector.trim();
  if (trimmed.length === 0 || trimmed.length > 1024) {
    throw new Error("Selector must be 1–1024 characters");
  }
  if (hasControlCharacters(trimmed)) {
    throw new Error("Selector must not contain control characters");
  }
  return trimmed;
}
export function normalizePath(path: string): string {
  const trimmed = path.trim();
  if (trimmed.length === 0 || trimmed.length > 1024) {
    throw new Error("Path must be 1–1024 characters");
  }
  if (/^([a-zA-Z]:|\/|\\|\.\.(\/|\\|$))/.test(trimmed)) {
    throw new Error("Path must be relative to the repository");
  }
  return trimmed;
}
export const api = {
  applicationStatus: () =>
    invoke<{
      version: string;
      supported_languages: string[];
      offline: boolean;
    }>("application_status"),
  projects: () => invoke<Project[]>("list_projects"),
  add: () => invoke<number | null>("add_project"),
  remove: (id: number) => invoke<void>("remove_project", { id }),
  status: (id: number) => invoke<Status>("project_status", { id }),
  index: (id: number) =>
    invoke<Record<string, number>>("index_project", { id }),
  cancel: (id: number) => invoke<void>("cancel_indexing", { id }),
  search: (id: number, query: string, offset = 0) =>
    invoke<SearchResults>("search_code", {
      id,
      query: normalizeQuery(query),
      offset: clampOffset(offset),
    }),
  files: (id: number) => invoke<ProjectMap>("project_files", { id }),
  preview: (id: number, path: string, offset = 0) =>
    invoke<FilePreview>("file_preview", {
      id,
      path: normalizePath(path),
      offset: clampOffset(offset),
    }),
  graph: (id: number, selector: string, incoming: boolean, depth: number) =>
    invoke<Graph>("dependencies", {
      id,
      selector: normalizeSelector(selector),
      incoming,
      depth: clampDepth(depth),
    }),
  impact: (id: number, mode: string, base: string) =>
    invoke<Impact>("git_impact", { id, mode, base }),
  context: (id: number, query: string, budget: number, includeGit: boolean) =>
    invoke<ContextPackage>("pack_context", {
      id,
      query: normalizeQuery(query),
      budget: clampBudget(budget),
      includeGit,
    }),
  integration: (id: number, harness: string) =>
    invoke<Integration>("integration_preview", { id, harness }),
  removeIntegration: (id: number, harness: string) =>
    invoke<Integration>("integration_remove_preview", { id, harness }),
  applyIntegration: (id: number, operationId: string) =>
    invoke<{ applied: boolean; backup?: string; connectivity: string }>(
      "integration_apply",
      { id, operationId },
    ),
  integrationStatus: (id: number, harness: string) =>
    invoke<{
      configuration_exists: boolean;
      contextforge_entry_present: boolean;
      connectivity: string;
    }>("integration_status", { id, harness }),
  theme: () => invoke<Theme>("get_theme"),
  setTheme: (theme: Theme) => invoke<void>("set_theme", { theme }),
  selectedProject: () => invoke<number | null>("selected_project"),
  selectProject: (id: number) => invoke<void>("select_project", { id }),
};

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (error && typeof error === "object" && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  if (error == null) return "Unknown error";
  return String(error);
}
