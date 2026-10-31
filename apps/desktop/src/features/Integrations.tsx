import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Plug } from "lucide-react";
import { api } from "../lib/api";
import { Failure, Notice, Pending } from "../components/common";

export function Integrations({ id }: { id: number }) {
  const [copied, setCopied] = useState(false);
  const preview = useMutation({
    mutationFn: ({ harness, remove }: { harness: string; remove: boolean }) =>
      remove
        ? api.removeIntegration(id, harness)
        : api.integration(id, harness),
  });
  const apply = useMutation({
    mutationFn: () => api.applyIntegration(id, preview.data!.operation_id),
  });
  const validation = useMutation({
    mutationFn: (harness: string) => api.integrationStatus(id, harness),
  });
  return (
    <section>
      <Notice>
        Each harness starts its own stdio MCP process; the desktop app can be
        closed. Choose a stable CLI executable, review the entry and confirm
        before applying. Connectivity requires an actual tool call in the
        harness.
      </Notice>
      <div className="project-grid">
        {[
          ["claude", "Claude Code"],
          ["codex", "OpenAI Codex"],
          ["cursor", "Cursor"],
          ["vscode", "VS Code / Copilot"],
        ].map(([key, title]) => (
          <article className="card" key={key}>
            <Plug size={22} />
            <h3>{title}</h3>
            <span className="badge">Connectivity not tested</span>
            <p className="muted small">
              Configuration changes preserve unrelated values and create a
              backup. JSONC is not supported.
            </p>
            <div className="row">
              <button
                disabled={preview.isPending}
                onClick={() => {
                  setCopied(false);
                  apply.reset();
                  preview.mutate({ harness: key, remove: false });
                }}
              >
                Select CLI & preview
              </button>
              <button
                className="small"
                disabled={validation.isPending}
                onClick={() => validation.mutate(key)}
              >
                Validate
              </button>
            </div>
            <button
              className="text-button small"
              style={{ marginTop: 14 }}
              disabled={preview.isPending}
              onClick={() => {
                apply.reset();
                preview.mutate({ harness: key, remove: true });
              }}
            >
              Preview removal
            </button>
          </article>
        ))}
      </div>
      {(preview.isPending || apply.isPending || validation.isPending) && (
        <Pending />
      )}
      {preview.error && <Failure error={preview.error} />}{" "}
      {apply.error && <Failure error={apply.error} />}{" "}
      {validation.error && <Failure error={validation.error} />}
      {validation.data && (
        <Notice>
          {validation.data.contextforge_entry_present
            ? "ContextForge entry found; syntax valid."
            : "No ContextForge entry found."}{" "}
          Actual connectivity: {validation.data.connectivity}.
        </Notice>
      )}
      {apply.data && (
        <Notice>
          {apply.data.applied
            ? `Configuration applied. Backup: ${apply.data.backup ?? "new file, no previous configuration"}. Restart the harness and test a tool call.`
            : "No changes applied."}
        </Notice>
      )}
      {preview.data && (
        <article className="card result">
          <h2>{preview.data.harness} configuration</h2>
          <p className="mono small">{preview.data.destination}</p>
          <CodeBlock text={preview.data.configuration} />
          <div className="row">
            <button
              className="primary"
              disabled={apply.isPending || apply.isSuccess}
              onClick={() => apply.mutate()}
            >
              Apply reviewed change�
            </button>
            <button
              onClick={() => {
                void navigator.clipboard
                  .writeText(preview.data!.configuration)
                  .then(() => setCopied(true));
              }}
            >
              Copy configuration
            </button>
            {copied && <span role="status">Copied</span>}
          </div>
          <Notice>{preview.data.instructions}</Notice>
          <Notice>{preview.data.verification}</Notice>
        </article>
      )}
    </section>
  );
}

function CodeBlock({ text }: { text: string }) {
  return (
    <pre className="code padded">
      <code>{text}</code>
    </pre>
  );
}
