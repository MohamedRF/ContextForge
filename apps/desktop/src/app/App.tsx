import { useEffect, useState } from "react";
import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Braces,
  ChevronLeft,
  CircleDot,
  Code2,
  FolderGit2,
  GitBranch,
  LayoutDashboard,
  Layers3,
  Menu,
  Plug,
  Plus,
  Search as SearchIcon,
  Settings2,
  ShieldCheck,
} from "lucide-react";
import { api, errorMessage } from "../lib/api";
import type { Progress, Theme } from "../lib/types";
import { Empty, Failure, Notice, Pending } from "../components/common";
import { Search } from "../features/Search";
import { Explorer } from "../features/Explorer";
import { Integrations } from "../features/Integrations";
import { DashboardOverview, ProjectCards } from "../features/Projects";
import {
  ContextBuilder,
  Dependencies,
  GitImpact,
} from "../features/Intelligence";

const navigation = [
  ["Dashboard", LayoutDashboard],
  ["Projects", FolderGit2],
  ["Search", SearchIcon],
  ["Code Explorer", Code2],
  ["Dependencies", GitBranch],
  ["Git Impact", Activity],
  ["Context Builder", Layers3],
  ["MCP Integrations", Plug],
  ["Settings", Settings2],
] as const;
type View = (typeof navigation)[number][0];
const descriptions: Record<View, string> = {
  Dashboard: "Local intelligence. Shared across every workflow.",
  Projects: "Your repositories, connected to one engine.",
  Search: "Find the evidence you need in your codebase.",
  "Code Explorer": "Read source and inspect its structure.",
  Dependencies: "Follow the connections in your code.",
  "Git Impact": "See what changed and what may be affected.",
  "Context Builder": "Give your next task the context it needs.",
  "MCP Integrations": "Bring local intelligence to your coding assistant.",
  Settings: "Make ContextForge feel at home.",
};

export function App() {
  const client = useQueryClient();
  const [view, setView] = useState<View>("Dashboard");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [sourcePath, setSourcePath] = useState("");
  const [toast, setToast] = useState("");
  const [progress, setProgress] = useState<Record<number, Progress>>({});
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const theme = useQuery({ queryKey: ["theme"], queryFn: api.theme });
  const savedSelection = useQuery({
    queryKey: ["selected-project"],
    queryFn: api.selectedProject,
  });
  const preferredId = selectedId ?? savedSelection.data ?? null;
  const id = projects.data?.some((project) => project.id === preferredId)
    ? preferredId
    : (projects.data?.[0]?.id ?? null);
  const project = projects.data?.find((project) => project.id === id);
  const status = useQuery({
    queryKey: ["status", id],
    queryFn: () => api.status(id!),
    enabled: id !== null,
  });
  const statuses = useQueries({
    queries: (projects.data ?? []).map((project) => ({
      queryKey: ["status", project.id],
      queryFn: () => api.status(project.id),
    })),
  });
  const add = useMutation({
    mutationFn: api.add,
    onSuccess: async (result) => {
      if (result !== null) {
        chooseProject(result);
        setToast("Repository added");
        await client.invalidateQueries({ queryKey: ["projects"] });
      }
    },
    onError: (error) => setToast(errorMessage(error)),
  });
  const index = useMutation({
    mutationFn: (projectId: number) => api.index(projectId),
    onSuccess: () => setToast("Index updated"),
    onError: (error) => setToast(errorMessage(error)),
    onSettled: async () => {
      await client.invalidateQueries();
    },
  });
  const remove = useMutation({
    mutationFn: api.remove,
    onSuccess: async () => {
      setToast("Registration removed. Source and indexes preserved.");
      await client.invalidateQueries();
    },
    onError: (error) => setToast(errorMessage(error)),
  });
  const setTheme = useMutation({
    mutationFn: api.setTheme,
    onSuccess: () => void client.invalidateQueries({ queryKey: ["theme"] }),
    onError: (error) => setToast(errorMessage(error)),
  });

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme =
      theme.data === "system" || !theme.data
        ? systemDark
          ? "dark"
          : "light"
        : theme.data;
  }, [theme.data, systemDark]);
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setView("Search");
      }
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, []);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<Progress>("index-progress", (event) => {
      setProgress((previous) => ({
        ...previous,
        [event.payload.project_id]: event.payload,
      }));
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((error) => setToast(errorMessage(error)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(timer);
  }, [toast]);
  function chooseProject(projectId: number) {
    setSelectedId(projectId);
    void api
      .selectProject(projectId)
      .catch((error) => setToast(errorMessage(error)));
  }
  function openSource(path: string) {
    setSourcePath(path);
    setView("Code Explorer");
  }
  const activeProgress = id === null ? undefined : progress[id];
  const indexing =
    activeProgress &&
    !["ready", "failed", "cancelled"].includes(activeProgress.phase);
  const totalFiles = statuses.reduce(
    (count, result) => count + (result.data?.index.files ?? 0),
    0,
  );
  const ready = statuses.filter(
    (result) => result.data?.index.state === "ready",
  ).length;

  return (
    <div className={`app ${collapsed ? "collapsed" : ""}`}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon">
            <Braces size={23} />
          </div>
          {!collapsed && (
            <div>
              ContextForge<span>LOCAL CODE INTELLIGENCE</span>
            </div>
          )}
        </div>
        {!collapsed && <div className="workspace-label">WORKSPACE</div>}
        <nav aria-label="Main navigation">
          {navigation.map(([name, Icon]) => (
            <button
              aria-label={name}
              title={name}
              className={view === name ? "nav-item active" : "nav-item"}
              key={name}
              onClick={() => setView(name)}
            >
              <Icon size={18} />
              {!collapsed && (
                <>
                  <span>{name}</span>
                  {name === "Search" && <kbd>⌘ K</kbd>}
                </>
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          {!collapsed && (
            <>
              <div className="local-badge">
                <ShieldCheck size={16} />
                <span>Private by design</span>
              </div>
              <p>
                Your source stays on this machine.
                <br />
                No account. No cloud required.
              </p>
            </>
          )}
          <button
            className="collapse"
            aria-label="Toggle sidebar"
            onClick={() => setCollapsed(!collapsed)}
          >
            {collapsed ? (
              <Menu size={17} />
            ) : (
              <>
                <ChevronLeft size={17} /> Collapse sidebar
              </>
            )}
          </button>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <div className="row">
            <FolderGit2 size={17} />
            <select
              aria-label="Current repository"
              value={id ?? ""}
              onChange={(event) => {
                chooseProject(Number(event.target.value));
                setSourcePath("");
              }}
            >
              {!projects.data?.length && (
                <option value="">No repository selected</option>
              )}
              {projects.data?.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                  {project.available ? "" : " (unavailable)"}
                </option>
              ))}
            </select>
          </div>
          <div className="row">
            <span className="status-dot" />{" "}
            <span className="small muted">Local engine</span>
            <span className="topbar-divider" />
            <button
              className="icon-button"
              title="Search code (Ctrl/Cmd K)"
              aria-label="Go to search"
              onClick={() => setView("Search")}
            >
              <SearchIcon size={17} />
            </button>
          </div>
        </header>
        <div className="page">
          <div className="page-heading">
            <div>
              <div className="eyebrow">CONTEXTFORGE / {view.toUpperCase()}</div>
              <h1>{view}</h1>
              <p className="muted">{descriptions[view]}</p>
            </div>
            {["Dashboard", "Projects"].includes(view) && (
              <button
                className="primary"
                disabled={add.isPending}
                onClick={() => add.mutate()}
              >
                <Plus size={16} /> Add repository
              </button>
            )}
          </div>
          {projects.isPending && <Pending />}
          {projects.error && (
            <Failure
              error={projects.error}
              retry={() => void projects.refetch()}
            />
          )}
          {toast && (
            <div role="status" className="toast">
              <CircleDot size={16} />
              {toast}
              <button
                aria-label="Dismiss notification"
                onClick={() => setToast("")}
              >
                ×
              </button>
            </div>
          )}
          {indexing && (
            <div className="index-progress">
              <div className="row spread">
                <span>Indexing · {activeProgress.phase}</span>
                <button
                  onClick={() => {
                    if (id !== null)
                      void api
                        .cancel(id)
                        .catch((error) => setToast(errorMessage(error)));
                  }}
                >
                  Cancel
                </button>
              </div>
              <progress
                max={Math.max(1, activeProgress.total)}
                value={activeProgress.processed}
              />
              <p className="small muted">
                {activeProgress.processed} / {activeProgress.total} files ·{" "}
                {(activeProgress.elapsed_ms / 1000).toFixed(1)}s
              </p>
            </div>
          )}
          {view === "Dashboard" && (
            <DashboardOverview
              id={id}
              projects={projects}
              status={status}
              ready={ready}
              totalFiles={totalFiles}
              setView={setView}
            />
          )}
          {(view === "Dashboard" || view === "Projects") && (
            <ProjectCards
              view={view}
              projects={projects}
              statuses={statuses}
              index={index}
              remove={remove}
              chooseProject={chooseProject}
              setView={setView}
            />
          )}
          {id === null &&
            !["Dashboard", "Projects", "Settings"].includes(view) && (
              <Empty title="Select a repository first">
                Add a local folder from Projects to use this tool.
              </Empty>
            )}
          {id !== null && (
            <div key={`${id}-${view}-${sourcePath}`}>
              {view === "Search" && <Search id={id} onOpen={openSource} />}
              {view === "Code Explorer" && (
                <Explorer id={id} initialPath={sourcePath} />
              )}
              {view === "Dependencies" && (
                <Dependencies id={id} onOpen={openSource} />
              )}
              {view === "Git Impact" && (
                <GitImpact id={id} onOpen={openSource} />
              )}
              {view === "Context Builder" && (
                <ContextBuilder id={id} onOpen={openSource} />
              )}
              {view === "MCP Integrations" && <Integrations id={id} />}
            </div>
          )}
          {view === "Settings" && (
            <section className="settings card">
              <h2>Appearance</h2>
              <p className="muted">Saved in the native user registry.</p>
              <label>
                Theme
                <select
                  aria-label="Theme"
                  value={theme.data ?? "system"}
                  onChange={(event) =>
                    setTheme.mutate(event.target.value as Theme)
                  }
                >
                  <option value="system">Follow system</option>
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                </select>
              </label>
              {theme.error && <Failure error={theme.error} />}
              <hr />
              <h2>Repository configuration</h2>
              <p>
                The canonical configuration is{" "}
                <code>.contextforge/config.toml</code> in each repository. CLI,
                MCP and Desktop use the same file.
              </p>
              <p>
                Maximum source size:{" "}
                {status.data?.repository.max_file_bytes.toLocaleString() ??
                  "Select a repository"}{" "}
                bytes.
              </p>
              <Notice>
                Indexing is explicit. No background daemon, cloud providers,
                telemetry or automatic updater is enabled. Source directories
                are read only; managed metadata is written under .contextforge.
              </Notice>
              <h2>Storage and recovery</h2>
              <p>
                Project registrations live in the operating system's local
                application data directory. Indexes remain in their existing
                repository cache locations. Unregistering preserves both source
                and caches.
              </p>
            </section>
          )}
          <footer>
            <span>
              <span className="status-dot" /> Offline by design
            </span>
            <span>
              {project?.name ?? "No repository selected"} ·{" "}
              {status.data?.version
                ? `v${status.data.version}`
                : "ContextForge"}
            </span>
          </footer>
        </div>
      </main>
    </div>
  );
}
