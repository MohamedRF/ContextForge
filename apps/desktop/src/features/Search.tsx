import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Search as SearchIcon, Copy, ArrowRight } from "lucide-react";
import { api } from "../lib/api";
import {
  Code,
  Empty,
  Evidence,
  Failure,
  Notice,
  Pending,
} from "../components/common";

export function Search({
  id,
  onOpen,
}: {
  id: number;
  onOpen: (path: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [offset, setOffset] = useState(0);
  const [copied, setCopied] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebounced(query.trim());
      setOffset(0);
    }, 250);
    return () => clearTimeout(timer);
  }, [query]);
  useEffect(() => {
    input.current?.focus();
  }, []);
  const result = useQuery({
    queryKey: ["search", id, debounced, offset],
    queryFn: () => api.search(id, debounced, offset),
    enabled: debounced.length > 0,
  });
  return (
    <section>
      <div className="search-box">
        <SearchIcon size={21} />
        <input
          ref={input}
          aria-label="Search code"
          placeholder="Search a symbol, file, or phrase…"
          maxLength={256}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <kbd>⌘ / Ctrl K</kbd>
      </div>
      <Notice>
        Exact symbols → file names → symbol prefixes → full-text relevance. All
        retrieval runs locally.
      </Notice>
      {result.isFetching && <Pending />}
      {result.error && (
        <Failure error={result.error} retry={() => void result.refetch()} />
      )}
      {!debounced && (
        <Empty title="Your codebase, within reach">
          Search captured source with file and line citations. Index the
          repository first.
        </Empty>
      )}
      {result.data && (
        <>
          <div className="section-label">
            {result.data.results.length} results · captured snapshot · freshness
            not checked
          </div>
          {result.data.results.length === 0 && (
            <Empty title="No matches">
              Try a symbol name, a filename or a shorter phrase.
            </Empty>
          )}
          <div className="results">
            {result.data.results.map((hit, i) => (
              <article className="card result" key={`${hit.path}-${i}`}>
                <div className="row spread">
                  <Evidence
                    path={hit.path}
                    line={hit.span.start_line}
                    onOpen={onOpen}
                  />
                  <span className="badge">{hit.method}</span>
                </div>
                <h3>
                  {hit.symbol ?? hit.path.split("/").pop()}{" "}
                  <span className="muted small">{hit.kind}</span>
                </h3>
                <Code text={hit.snippet} start={hit.span.start_line} />
                {hit.snippet_truncated && (
                  <Notice>Snippet clipped by the backend.</Notice>
                )}
                <div className="row">
                  <button
                    className="small"
                    onClick={() => {
                      void navigator.clipboard
                        .writeText(
                          `${hit.path}:${hit.span.start_line}\n\n${hit.snippet}`,
                        )
                        .then(() => setCopied(true));
                    }}
                  >
                    <Copy size={13} /> Copy result
                  </button>
                  <button className="small" onClick={() => onOpen(hit.path)}>
                    View source <ArrowRight size={13} />
                  </button>
                </div>
              </article>
            ))}
          </div>
          <div className="row">
            <button
              disabled={offset === 0}
              onClick={() => setOffset(Math.max(0, offset - 30))}
            >
              Previous
            </button>
            <button
              disabled={result.data.next_offset === null}
              onClick={() => setOffset(result.data?.next_offset ?? offset)}
            >
              Next results
            </button>
            {copied && <span role="status">Copied</span>}
          </div>
        </>
      )}
    </section>
  );
}
