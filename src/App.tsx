import { useEffect, useMemo, useState } from "react";
import { FilePlus2, FileText, Download, Play, ShieldCheck } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./App.css";
import {
  tauriBackend,
  type Backend,
  type Capabilities,
  type ConversionResult,
  type InputSummary,
} from "./lib/backend";

interface AppProps {
  backend?: Backend;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

function App({ backend = tauriBackend }: AppProps) {
  const [capabilities, setCapabilities] = useState<Capabilities | null>(null);
  const [inputs, setInputs] = useState<InputSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [results, setResults] = useState<Record<string, ConversionResult>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selected = useMemo(
    () => inputs.find((input) => input.inputId === selectedId) ?? null,
    [inputs, selectedId],
  );
  const result = selected ? results[selected.inputId] : undefined;

  useEffect(() => {
    backend.getCapabilities().then(setCapabilities).catch(() => {
      setError("Rust backend is not available.");
    });
  }, [backend]);

  async function addFiles() {
    setError(null);
    try {
      const picked = await backend.pickInputs();
      setInputs((current) => {
        const known = new Set(current.map((item) => item.inputId));
        return [...current, ...picked.filter((item) => !known.has(item.inputId))];
      });
      if (picked[0]) setSelectedId(picked[0].inputId);
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function convertSelected() {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      const converted = await backend.convertInput(selected.inputId);
      setResults((current) => ({ ...current, [selected.inputId]: converted }));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function exportSelected() {
    if (!result) return;
    setError(null);
    try {
      await backend.exportResult(result.resultId);
    } catch (cause) {
      setError(String(cause));
    }
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark"><FileText size={20} /></div>
          <div>
            <h1>Document to Markdown</h1>
            <p>Private, local, Rust-powered conversion</p>
          </div>
        </div>
        <div className="local-badge"><ShieldCheck size={15} /> Offline processing</div>
      </header>

      <section className="toolbar" aria-label="Conversion actions">
        <button className="button primary" type="button" onClick={addFiles}>
          <FilePlus2 size={17} /> Add files
        </button>
        <button className="button" type="button" onClick={convertSelected} disabled={!selected || busy}>
          <Play size={16} /> {busy ? "Converting…" : "Convert"}
        </button>
        <button className="button" type="button" onClick={exportSelected} disabled={!result}>
          <Download size={16} /> Export Markdown
        </button>
        <span className="format-note">
          {capabilities ? `${capabilities.formats.join(", ")} enabled` : "Connecting to Rust…"}
        </span>
      </section>

      {error && <div className="error-banner" role="alert">{error}</div>}

      <div className="workspace">
        <section className="queue-panel" aria-label="Document queue">
          <div className="panel-heading">
            <div><span className="eyebrow">QUEUE</span><h2>Documents</h2></div>
            <span className="count">{inputs.length}</span>
          </div>
          {inputs.length === 0 ? (
            <button className="empty-state" type="button" onClick={addFiles}>
              <FilePlus2 size={26} />
              <strong>Add documents</strong>
              <span>TXT, Markdown, CSV, TSV, JSON, and XML are available locally.</span>
            </button>
          ) : (
            <ul className="file-list">
              {inputs.map((input) => (
                <li key={input.inputId}>
                  <button
                    className={input.inputId === selectedId ? "file-row selected" : "file-row"}
                    type="button"
                    onClick={() => setSelectedId(input.inputId)}
                  >
                    <FileText size={18} />
                    <span className="file-copy"><strong>{input.name}</strong><small>{input.format.toUpperCase()} · {formatBytes(input.size)}</small></span>
                    <span className={results[input.inputId] ? "status ready" : "status"}>
                      {results[input.inputId] ? "Ready" : "Pending"}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="preview-panel" aria-label="Markdown preview">
          <div className="panel-heading preview-heading">
            <div><span className="eyebrow">PREVIEW</span><h2>{selected?.name ?? "No document selected"}</h2></div>
            {result && <span className="status ready">Converted</span>}
          </div>
          <div className="preview-body">
            {result ? (
              <>
                {result.diagnostics.length > 0 && (
                  <aside className="diagnostics" aria-label="Conversion diagnostics">
                    {result.diagnostics.map((diagnostic) => (
                      <div className={`diagnostic ${diagnostic.severity}`} key={diagnostic.code}>
                        <strong>{diagnostic.code}</strong>
                        <span>{diagnostic.message}</span>
                      </div>
                    ))}
                  </aside>
                )}
                <article className="markdown-preview">
                  <ReactMarkdown remarkPlugins={[remarkGfm]} skipHtml>{result.markdown}</ReactMarkdown>
                </article>
              </>
            ) : (
              <div className="preview-placeholder">
                <FileText size={34} />
                <strong>{selected ? "Ready to convert" : "Preview appears here"}</strong>
                <span>{selected ? "Press Convert to run the Rust converter." : "Add a local document to begin."}</span>
              </div>
            )}
          </div>
        </section>
      </div>

      <footer>
        <span>{inputs.length} document{inputs.length === 1 ? "" : "s"}</span>
        <span>Protocol {capabilities?.protocolVersion ?? "—"}</span>
      </footer>
    </main>
  );
}

export default App;
