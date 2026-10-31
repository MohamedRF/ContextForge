import {
  Activity,
  ArrowUpRight,
  Box,
  Braces,
  Code2,
  Database,
  FolderGit2,
  GitBranch,
  Plug,
  Search as SearchIcon,
  ShieldCheck,
} from "lucide-react";
import type { Project, Status } from "../lib/types";
import { Empty, Failure } from "../components/common";

type Navigate = (
  view: "Search" | "Context Builder" | "Projects" | "MCP Integrations",
) => void;
type ProjectsQuery = { data?: Project[] };
type StatusQuery = { data?: Status; error?: unknown };
type Mutation = { isPending: boolean; mutate: (id: number) => void };

export function DashboardOverview({
  id,
  projects,
  status,
  ready,
  totalFiles,
  version,
  setView,
}: {
  id: number | null;
  projects: ProjectsQuery;
  status: StatusQuery;
  ready: number;
  totalFiles: number;
  version?: string;
  setView: Navigate;
}) {
  return (
    <>
      <div className="hero card">
        <div>
          <div className="eyebrow">ONE ENGINE. EVERY WORKFLOW.</div>
          <h2>
            Know your code.
            <br />
            <span>Build with context.</span>
          </h2>
          <p>
            Explore repositories, trace changes, and assemble precise
            <br className="wide-only" /> context for your next task — entirely
            on your machine.
          </p>
          <div className="row">
            <button
              className="primary"
              disabled={id === null}
              onClick={() => setView("Search")}
            >
              <SearchIcon size={15} /> Search code <ArrowUpRight size={15} />
            </button>
            <button
              disabled={id === null}
              onClick={() => setView("Context Builder")}
            >
              Build context
            </button>
          </div>
        </div>
        <div className="hero-art" aria-hidden="true">
          <div className="orbit orbit-one" />
          <div className="orbit orbit-two" />
          <div className="engine-symbol">
            <Braces size={48} />
          </div>
          <div className="satellite one">
            <Code2 size={20} />
          </div>
          <div className="satellite two">
            <GitBranch size={20} />
          </div>
          <div className="satellite three">
            <Database size={20} />
          </div>
        </div>
      </div>
      <div className="metrics">
        <div className="card metric">
          <FolderGit2 size={19} />
          <span>Registered repositories</span>
          <strong>{projects.data?.length ?? "—"}</strong>
          <small>{ready} with captured indexes</small>
        </div>
        <div className="card metric">
          <Code2 size={19} />
          <span>Indexed source files</span>
          <strong>{projects.data ? totalFiles.toLocaleString() : "—"}</strong>
          <small>Across available repositories</small>
        </div>
        <div className="card metric">
          <Database size={19} />
          <span>Selected index</span>
          <strong className="word">{status.data?.index.state ?? "—"}</strong>
          <small>
            Freshness {status.data?.index.freshness ?? "not checked"}
          </small>
        </div>
        <div className="card metric">
          <Box size={19} />
          <span>Application version</span>
          <strong>{version ?? "—"}</strong>
          <small>CLI / Desktop / MCP</small>
        </div>
      </div>
      <div className="row spread section-heading">
        <h2>Your repositories</h2>
        <button
          className="text-button small"
          onClick={() => setView("Projects")}
        >
          View all <ArrowUpRight size={14} />
        </button>
      </div>
    </>
  );
}

export function ProjectCards({
  view,
  languages,
  projects,
  statuses,
  index,
  remove,
  chooseProject,
  setView,
}: {
  view: "Dashboard" | "Projects";
  languages: string[];
  projects: ProjectsQuery;
  statuses: StatusQuery[];
  index: Mutation;
  remove: Mutation;
  chooseProject: (id: number) => void;
  setView: Navigate;
}) {
  return (
    <>
      {projects.data?.length === 0 && (
        <Empty title="Start with a repository">
          Add a local folder, then capture an index to explore code and build
          context.
        </Empty>
      )}
      <div className="project-grid">
        {projects.data?.map((item, i) => (
          <article className="card project-card" key={item.id}>
            <div className="row spread">
              <FolderGit2 size={24} />
              <span className="badge">
                {!item.available
                  ? "Unavailable"
                  : (statuses[i]?.data?.index.state ?? "Checking")}
              </span>
            </div>
            <button
              className="project-name text-button"
              onClick={() => chooseProject(item.id)}
            >
              {item.name}
            </button>
            <p className="mono path" title={item.path}>
              {item.path}
            </p>
            <div className="row spread">
              <span className="small muted">
                {statuses[i]?.data?.index.files ?? "—"} indexed files
              </span>
              <span className="small muted">
                {statuses[i]?.data?.repository.git_detected
                  ? "Git repository"
                  : "Local folder"}
              </span>
            </div>
            {Boolean(statuses[i]?.error) && (
              <Failure error={statuses[i].error} />
            )}
            <div className="project-actions row">
              <button
                disabled={!item.available || index.isPending}
                onClick={() => index.mutate(item.id)}
              >
                <Activity size={14} /> Index
              </button>
              <button
                disabled={!item.available}
                onClick={() => {
                  chooseProject(item.id);
                  setView("Search");
                }}
              >
                Search
              </button>
              {view === "Projects" && (
                <button
                  className="danger small"
                  disabled={remove.isPending}
                  onClick={() => remove.mutate(item.id)}
                >
                  Unregister
                </button>
              )}
            </div>
          </article>
        ))}
      </div>
      {view === "Dashboard" && (
        <div className="two-columns info-panels">
          <div className="card">
            <h3>
              <Plug size={17} /> Intelligence, in your workflow
            </h3>
            <p>
              Use the same local index from CLI, Desktop, Claude Code, Codex,
              Cursor and VS Code.
            </p>
            <button
              className="text-button small"
              onClick={() => setView("MCP Integrations")}
            >
              Set up an MCP client <ArrowUpRight size={13} />
            </button>
          </div>
          <div className="card">
            <h3>
              <ShieldCheck size={17} /> A clear evidence trail
            </h3>
            <p>
              Source citations, bounded results and visible uncertainty. Know
              where each result came from before you act.
            </p>
            <div className="row">
              {languages.map((language) => (
                <span className="badge" key={language}>
                  {language}
                </span>
              ))}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
