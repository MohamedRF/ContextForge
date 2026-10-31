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
    invoke<SearchResults>("search_code", { id, query, offset }),
  files: (id: number) => invoke<ProjectMap>("project_files", { id }),
  preview: (id: number, path: string, offset = 0) =>
    invoke<FilePreview>("file_preview", { id, path, offset }),
  graph: (id: number, selector: string, incoming: boolean, depth: number) =>
    invoke<Graph>("dependencies", { id, selector, incoming, depth }),
  impact: (id: number, mode: string, base: string) =>
    invoke<Impact>("git_impact", { id, mode, base }),
  context: (id: number, query: string, budget: number, includeGit: boolean) =>
    invoke<ContextPackage>("pack_context", { id, query, budget, includeGit }),
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
  return error instanceof Error ? error.message : String(error);
}
