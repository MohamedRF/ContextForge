import {
  fireEvent,
  render,
  screen,
  waitFor,
  cleanup,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { api } from "../lib/api";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));
vi.mock("../lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      applicationStatus: vi.fn().mockResolvedValue({
        version: "0.1.0",
        supported_languages: ["rust", "python", "typescript", "javascript"],
        offline: true,
      }),
      projects: vi.fn(),
      theme: vi.fn(),
      status: vi.fn(),
      search: vi.fn(),
      add: vi.fn(),
      index: vi.fn(),
      remove: vi.fn(),
      setTheme: vi.fn(),
      selectedProject: vi.fn().mockResolvedValue(null),
      selectProject: vi.fn().mockResolvedValue(undefined),
    },
  };
});
function mount() {
  return render(
    <QueryClientProvider
      client={
        new QueryClient({
          defaultOptions: {
            queries: { retry: false },
            mutations: { retry: false },
          },
        })
      }
    >
      <App />
    </QueryClientProvider>,
  );
}
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.theme).mockResolvedValue("dark");
  vi.mocked(api.projects).mockResolvedValue([]);
});
afterEach(cleanup);
describe("Desktop navigation and native operations", () => {
  it("refreshes shared state after changes from another interface", async () => {
    mount();
    await screen.findByText("Start with a repository");
    expect(screen.getByText("0.1.0")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Refresh local data" }));
    await waitFor(() => expect(api.projects).toHaveBeenCalledTimes(2));
  });
  it("shows real empty state and prevents search without a selected repository", async () => {
    mount();
    expect(
      await screen.findByText("Start with a repository"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(
      await screen.findByText("Select a repository first"),
    ).toBeInTheDocument();
    expect(api.search).not.toHaveBeenCalled();
  });
  it("reports registry errors and offers retry", async () => {
    vi.mocked(api.projects).mockRejectedValue("Registry unavailable");
    mount();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Registry unavailable",
    );
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(api.projects).toHaveBeenCalledTimes(2));
  });
  it("opens keyboard search and debounces queries through the typed native adapter", async () => {
    vi.mocked(api.projects).mockResolvedValue([
      { id: 7, name: "fixture", path: "/fixture", available: true },
    ]);
    vi.mocked(api.status).mockResolvedValue({
      repository: {
        initialized: true,
        git_detected: true,
        max_file_bytes: 1024,
      },
      index: {
        state: "ready",
        files: 1,
        freshness: "not_checked",
        schema_version: 2,
      },
      version: "0.1.0",
    });
    vi.mocked(api.search).mockResolvedValue({
      results: [],
      truncated: false,
      next_offset: null,
      freshness: "snapshot_not_checked",
    });
    mount();
    await screen.findByText("fixture", { selector: "button.project-name" });
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Search code" }),
      { target: { value: "authenticate" } },
    );
    expect(api.search).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(api.search).toHaveBeenCalledWith(7, "authenticate", 0),
    );
    expect(await screen.findByText("No matches")).toBeInTheDocument();
  });
});
