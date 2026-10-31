import type { ReactNode } from "react";
import { AlertTriangle, LoaderCircle, SearchX } from "lucide-react";
import { errorMessage } from "../lib/api";

export function Empty({
  title,
  children,
}: {
  title: string;
  children?: ReactNode;
}) {
  return (
    <div className="empty">
      <SearchX size={30} />
      <h2>{title}</h2>
      <p>{children}</p>
    </div>
  );
}
export function Pending() {
  return (
    <div role="status" className="loading">
      <LoaderCircle className="spin" size={18} /> Reading local evidence…
      <div className="skeleton" />
    </div>
  );
}
export function Failure({
  error,
  retry,
}: {
  error: unknown;
  retry?: () => void;
}) {
  return (
    <div role="alert" className="error">
      <AlertTriangle size={18} />
      <span>{errorMessage(error)}</span>
      {retry && <button onClick={retry}>Retry</button>}
    </div>
  );
}
export function Notice({ children }: { children: ReactNode }) {
  return <p className="notice">{children}</p>;
}
export function Code({ text, start = 1 }: { text: string; start?: number }) {
  return (
    <pre className="code">
      {text.split("\n").map((line, i) => (
        <div key={i}>
          <span className="line-number">{start + i}</span>
          <code>{highlight(line || " ")}</code>
        </div>
      ))}
    </pre>
  );
}

// Lexical presentation only. React escapes all source tokens; no source HTML is injected.
function highlight(line: string): ReactNode[] {
  const tokens = line.split(
    /("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\/\/.*|#.*|\b(?:pub|fn|let|mut|impl|struct|enum|use|mod|return|if|else|match|async|await|const|function|class|import|export|from|def|self|for|while|true|false|None|null|type|interface)\b|\b\d+\b)/g,
  );
  return tokens.map((token, index) => {
    const kind = /^(\/\/|#)/.test(token)
      ? "comment"
      : /^["']/.test(token)
        ? "string"
        : /^\d+$/.test(token)
          ? "number"
          : index % 2 === 1
            ? "keyword"
            : "plain";
    return (
      <span className={`token-${kind}`} key={index}>
        {token}
      </span>
    );
  });
}
export function Evidence({
  path,
  line,
  onOpen,
}: {
  path: string;
  line?: number;
  onOpen: (path: string) => void;
}) {
  return (
    <button className="text-button mono" onClick={() => onOpen(path)}>
      {path}
      {line ? `:${line}` : ""}
    </button>
  );
}
