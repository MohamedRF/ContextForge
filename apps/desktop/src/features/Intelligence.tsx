import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../lib/api";
import {
  Code,
  Empty,
  Evidence,
  Failure,
  Notice,
  Pending,
} from "../components/common";
import type { ContextPackage, Graph, Visit } from "../lib/types";

export function Dependencies({
  id,
  onOpen,
}: {
  id: number;
  onOpen: (path: string) => void;
}) {
  const [selector, setSelector] = useState("");
  const [incoming, setIncoming] = useState(false);
  const [depth, setDepth] = useState(2);
  const [selected, setSelected] = useState<string | null>(null);
  const graph = useMutation({
    mutationFn: () => api.graph(id, selector, incoming, depth),
  });
  return (
    <section>
      <form
        className="toolbar"
        onSubmit={(e) => {
          e.preventDefault();
          graph.mutate();
        }}
      >
        <label className="grow">
          Symbol or file
          <input
            required
            maxLength={1024}
            placeholder="e.g. Repository or src/main.rs"
            value={selector}
            onChange={(e) => setSelector(e.target.value)}
          />
        </label>
        <label>
          Direction
          <select
            value={String(incoming)}
            onChange={(e) => setIncoming(e.target.value === "true")}
          >
            <option value="false">Dependencies</option>
            <option value="true">Dependents</option>
          </select>
        </label>
        <label>
          Depth
          <input
            type="number"
            min={0}
            max={8}
            value={depth}
            onChange={(e) => setDepth(Number(e.target.value))}
          />
        </label>
        <button className="primary" disabled={graph.isPending}>
          Explore graph
        </button>
      </form>
      <Notice>
        Syntax-backed relationships, capped at 100 nodes and 200 edges. Dashed
        edges are heuristic or unresolved; this is not compiler-level
        resolution.
      </Notice>
      {graph.isPending && <Pending />}
      {graph.error && <Failure error={graph.error} />}
      {!graph.data && !graph.isPending && (
        <Empty title="Follow the relationships">
          Trace local calls, imports and lexical structure from the captured
          index.
        </Empty>
      )}
      {graph.data && (
        <>
          <DependencyCanvas
            graph={graph.data}
            selected={selected}
            onSelect={setSelected}
          />
          {(graph.data.truncated || graph.data.partial_syntax) && (
            <Notice>
              Traversal or syntax is partial. Expand a selected node to
              investigate further.
            </Notice>
          )}
          <div className="node-list">
            {graph.data.nodes
              .filter((visit) => !selected || visit.node.id === selected)
              .map((visit) => (
                <div key={visit.node.id} className="card row spread">
                  <div>
                    <strong>{visit.node.qualified_name}</strong>
                    <p>
                      <Evidence
                        path={visit.node.path}
                        line={visit.node.span?.start_line}
                        onOpen={onOpen}
                      />
                    </p>
                    <span className="badge">
                      {visit.status} · depth {visit.depth}
                    </span>
                  </div>
                  <button
                    onClick={() => {
                      setSelector(visit.node.id);
                      setSelected(null);
                    }}
                  >
                    Use as next starting node
                  </button>
                </div>
              ))}
          </div>
          <details>
            <summary>Source evidence ({graph.data.edges.length})</summary>
            {graph.data.edges.map((edge, i) => (
              <div className="card small" key={i}>
                <span className="badge">
                  {edge.status} · {edge.kind}
                </span>{" "}
                <Evidence
                  path={edge.path}
                  line={edge.span?.start_line}
                  onOpen={onOpen}
                />
                <pre>{edge.text}</pre>
                <span className="muted">{edge.method}</span>
              </div>
            ))}
          </details>
        </>
      )}
    </section>
  );
}

function DependencyCanvas({
  graph,
  selected,
  onSelect,
}: {
  graph: Graph;
  selected: string | null;
  onSelect: (id: string | null) => void;
}) {
  const [zoom, setZoom] = useState(1);
  const points = new Map(
    graph.nodes.map((visit, i) => [
      visit.node.id,
      {
        x: 450 + 300 * Math.cos((i * Math.PI * 2) / graph.nodes.length),
        y: 220 + 165 * Math.sin((i * Math.PI * 2) / graph.nodes.length),
      },
    ]),
  );
  return (
    <div className="graph-shell">
      <div className="row graph-controls">
        <button
          aria-label="Zoom out"
          onClick={() => setZoom(Math.max(0.5, zoom - 0.25))}
        >
          −
        </button>
        <button
          aria-label="Zoom in"
          onClick={() => setZoom(Math.min(3, zoom + 0.25))}
        >
          +
        </button>
        <button
          onClick={() => {
            setZoom(1);
            onSelect(null);
          }}
        >
          Reset
        </button>
        <span className="small muted">Scroll to pan · select a node</span>
      </div>
      <div className="graph-scroll">
        <svg
          width={900 * zoom}
          height={440 * zoom}
          viewBox="0 0 900 440"
          aria-label="Dependency graph"
        >
          {graph.edges.map((edge, i) => {
            const from = points.get(edge.from),
              to = edge.to ? points.get(edge.to) : null;
            return from && to ? (
              <line
                key={i}
                x1={from.x}
                y1={from.y}
                x2={to.x}
                y2={to.y}
                className={
                  edge.status === "resolved" ? "edge" : "edge heuristic"
                }
              />
            ) : null;
          })}
          {graph.nodes.map((visit) => {
            const point = points.get(visit.node.id)!;
            return (
              <g
                key={visit.node.id}
                tabIndex={0}
                role="button"
                aria-label={visit.node.qualified_name}
                className={`graph-node ${selected === visit.node.id ? "selected" : ""}`}
                onClick={() => onSelect(visit.node.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ")
                    onSelect(visit.node.id);
                }}
              >
                <title>
                  {visit.node.qualified_name} · {visit.node.path}
                </title>
                <circle
                  cx={point.x}
                  cy={point.y}
                  r={selected === visit.node.id ? 10 : 7}
                />
                <text x={point.x + 13} y={point.y + 4}>
                  {visit.node.qualified_name.slice(0, 28)}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}

function Visits({
  title,
  visits,
  onOpen,
}: {
  title: string;
  visits: Visit[];
  onOpen: (path: string) => void;
}) {
  return (
    <section className="card">
      <h3>
        {title} <span className="badge">{visits.length}</span>
      </h3>
      {visits.length === 0 ? (
        <p className="muted small">None detected within the analysis limits.</p>
      ) : (
        visits.map((visit, i) => (
          <p key={i}>
            <Evidence
              path={visit.node.path}
              line={visit.node.span?.start_line}
              onOpen={onOpen}
            />{" "}
            <span className="small">{visit.node.qualified_name}</span>
          </p>
        ))
      )}
    </section>
  );
}

export function GitImpact({
  id,
  onOpen,
}: {
  id: number;
  onOpen: (path: string) => void;
}) {
  const [mode, setMode] = useState("unstaged");
  const [base, setBase] = useState("main");
  const impact = useMutation({ mutationFn: () => api.impact(id, mode, base) });
  return (
    <section>
      <form
        className="toolbar"
        onSubmit={(e) => {
          e.preventDefault();
          impact.mutate();
        }}
      >
        <label>
          Compare
          <select value={mode} onChange={(e) => setMode(e.target.value)}>
            <option value="unstaged">Working tree → index</option>
            <option value="staged">Index → HEAD</option>
            <option value="base">Branch merge base → HEAD</option>
          </select>
        </label>
        {mode === "base" && (
          <label>
            Base ref
            <input
              required
              maxLength={256}
              value={base}
              onChange={(e) => setBase(e.target.value)}
            />
          </label>
        )}
        <button className="primary" disabled={impact.isPending}>
          Analyze changes
        </button>
      </form>
      <Notice>
        Evidence-qualified impact, not a safety verdict. Missing dependencies do
        not prove a change is safe.
      </Notice>
      {impact.isPending && <Pending />}
      {impact.error && <Failure error={impact.error} />}
      {!impact.data && !impact.isPending && (
        <Empty title="Understand the reach of a change">
          Compare Git snapshots without modifying source, staging or history.
        </Empty>
      )}
      {impact.data && (
        <>
          <div className="section-label">
            {impact.data.direct_changes.length} changed files
            {impact.data.truncated ? " · analysis truncated" : ""}
          </div>
          {impact.data.direct_changes.map((change, i) => (
            <article key={i} className="card">
              <div className="row spread">
                <Evidence
                  path={change.new_path ?? change.old_path ?? ""}
                  onOpen={onOpen}
                />
                <span className="badge">{change.kind}</span>
              </div>
              <p className="small">
                {change.current_symbols
                  .map((symbol) => symbol.qualified_name)
                  .join(", ") || "No current symbols captured"}
              </p>
              {change.hunks.map((hunk, j) => (
                <div key={j} className="diff-range mono">
                  − {hunk.old_start},{hunk.old_lines}{" "}
                  <span>
                    + {hunk.new_start},{hunk.new_lines}
                  </span>
                </div>
              ))}
            </article>
          ))}
          <div className="two-columns">
            <Visits
              title="Confirmed structural dependents"
              visits={impact.data.resolved_dependents}
              onOpen={onOpen}
            />
            <Visits
              title="Possible dependents"
              visits={impact.data.heuristic_dependents}
              onOpen={onOpen}
            />
          </div>
          <Visits
            title="Potentially relevant tests"
            visits={impact.data.potentially_affected_tests}
            onOpen={onOpen}
          />
          <Notice>{impact.data.uncertainty}</Notice>
          <details>
            <summary>Why these relationships were included</summary>
            {impact.data.dependency_evidence.map((edge, i) => (
              <div key={i} className="card small">
                <span className="badge">
                  {edge.kind} · {edge.status}
                </span>{" "}
                <Evidence
                  path={edge.path}
                  line={edge.span?.start_line}
                  onOpen={onOpen}
                />
                <pre>{edge.text}</pre>
              </div>
            ))}
          </details>
        </>
      )}
    </section>
  );
}

function markdown(pkg: ContextPackage): string {
  return (
    `# Context: ${pkg.query}\n\n` +
    pkg.results
      .map(
        (item) =>
          `## ${item.path}:${item.span.start_line}-${item.span.end_line}\n${item.reason}\n\n\`\`\`\n${item.snippet}\n\`\`\`\n`,
      )
      .join("\n")
  );
}

export function ContextBuilder({
  id,
  onOpen,
}: {
  id: number;
  onOpen: (path: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [budget, setBudget] = useState(4000);
  const [git, setGit] = useState(false);
  const [copyStatus, setCopyStatus] = useState("");
  const pack = useMutation({
    mutationFn: () => api.context(id, query, budget, git),
  });
  function copy(text: string) {
    void navigator.clipboard
      .writeText(text)
      .then(() => setCopyStatus("Copied"))
      .catch(() => setCopyStatus("Clipboard unavailable"));
  }
  return (
    <section>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          pack.mutate();
        }}
      >
        <label className="section-label" htmlFor="context-task">
          WHAT ARE YOU WORKING ON?
        </label>
        <textarea
          id="context-task"
          required
          maxLength={256}
          rows={3}
          placeholder="Find the authentication logic and affected modules…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="toolbar">
          <label>
            Estimated token budget
            <input
              type="number"
              min={256}
              max={32000}
              value={budget}
              onChange={(e) => setBudget(Number(e.target.value))}
            />
          </label>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={git}
              onChange={(e) => setGit(e.target.checked)}
            />{" "}
            Include Git proximity
          </label>
          <button className="primary" disabled={pack.isPending}>
            Build context
          </button>
        </div>
      </form>
      <Notice>
        Ranked and budgeted by the same Rust algorithm as CLI/MCP. Estimates use
        UTF-8 bytes ÷ 4, not a model tokenizer.
      </Notice>
      {pack.isPending && <Pending />}
      {pack.error && <Failure error={pack.error} />}
      {pack.data && (
        <>
          <div className="card row spread">
            <div>
              <strong>
                {pack.data.estimated_tokens.toLocaleString()} /{" "}
                {pack.data.budget_tokens.toLocaleString()}
              </strong>
              <p className="small muted">
                estimated tokens · {pack.data.results.length} snippets
                {pack.data.truncated ? " · truncated" : ""}
              </p>
            </div>
            <div className="row">
              <button onClick={() => copy(markdown(pack.data!))}>
                Copy Markdown
              </button>
              <button onClick={() => copy(JSON.stringify(pack.data, null, 2))}>
                Copy JSON
              </button>
              <span role="status">{copyStatus}</span>
            </div>
          </div>
          {pack.data.results.length === 0 && (
            <Empty title="No context found">
              Try a more specific task or refresh the index.
            </Empty>
          )}
          {pack.data.results.map((item, i) => (
            <article className="card result" key={i}>
              <div className="row spread">
                <Evidence
                  path={item.path}
                  line={item.span.start_line}
                  onOpen={onOpen}
                />
                <span className="badge">{item.confidence}</span>
              </div>
              <h3>{item.symbol ?? "Source evidence"}</h3>
              <p className="muted small">{item.reason}</p>
              <Code text={item.snippet} start={item.span.start_line} />
            </article>
          ))}
        </>
      )}
    </section>
  );
}
