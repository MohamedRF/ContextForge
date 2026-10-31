import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { FileCode2 } from "lucide-react";
import { api } from "../lib/api";
import { Code, Empty, Failure, Notice, Pending } from "../components/common";

export function Explorer({
  id,
  initialPath = "",
}: {
  id: number;
  initialPath?: string;
}) {
  const [path, setPath] = useState(initialPath);
  const [filter, setFilter] = useState("");
  const [offset, setOffset] = useState(0);
  const files = useQuery({
    queryKey: ["files", id],
    queryFn: () => api.files(id),
  });
  const source = useQuery({
    queryKey: ["preview", id, path, offset],
    queryFn: () => api.preview(id, path, offset),
    enabled: path.length > 0,
  });
  return (
    <div className="explorer">
      <aside className="file-panel">
        <label className="section-label" htmlFor="file-filter">
          REPOSITORY FILES
        </label>
        <input
          id="file-filter"
          placeholder="Filter paths…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        {files.isPending && <Pending />}
        {files.error && <Failure error={files.error} />}
        {files.data?.truncated && (
          <Notice>File map limited to 1,000 files.</Notice>
        )}
        <div className="file-list">
          {files.data?.files
            .filter((file) =>
              file.path.toLowerCase().includes(filter.toLowerCase()),
            )
            .map((file) => (
              <button
                key={file.path}
                className={path === file.path ? "file active" : "file"}
                title={file.path}
                onClick={() => {
                  setPath(file.path);
                  setOffset(0);
                }}
              >
                <FileCode2 size={14} />
                {file.path}
              </button>
            ))}
        </div>
      </aside>
      <section className="source-panel">
        {!path && (
          <Empty title="Explore source evidence">
            Choose a file to inspect its current source and syntax outline.
          </Empty>
        )}
        {source.isFetching && <Pending />}
        {source.error && (
          <Failure error={source.error} retry={() => void source.refetch()} />
        )}
        {source.data && (
          <>
            <div className="row spread">
              <h3 className="mono">{path}</h3>
              <span className="badge">Current source · read only</span>
            </div>
            <div className="outline">
              {source.data.outline.symbols.map((symbol) => (
                <button
                  className="small"
                  key={symbol.id}
                  onClick={() =>
                    setOffset(Math.max(0, symbol.span.start_line - 1))
                  }
                >
                  {symbol.kind} <strong>{symbol.name}</strong>{" "}
                  <span className="muted">:{symbol.span.start_line}</span>
                </button>
              ))}
            </div>
            {(source.data.outline.has_errors ||
              source.data.outline.truncated) && (
              <Notice>
                Syntax is partial or capped. Relationships may be incomplete.
              </Notice>
            )}
            <Code text={source.data.text} start={offset + 1} />
            <div className="row">
              <button
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 200))}
              >
                Previous lines
              </button>
              <span className="muted small">
                Lines {offset + 1}–{offset + source.data.lines}
              </span>
              <button
                disabled={!source.data.has_more || source.data.lines === 0}
                onClick={() => setOffset(offset + (source.data?.lines ?? 0))}
              >
                Next lines
              </button>
            </div>
            <details>
              <summary>
                Imports, exports and syntax occurrences (
                {source.data.outline.occurrences.length})
              </summary>
              {source.data.outline.occurrences.map((item, i) => (
                <p className="mono small" key={i}>
                  {item.kind} · {item.text} · line {item.span.start_line}
                </p>
              ))}
            </details>
          </>
        )}
      </section>
    </div>
  );
}
