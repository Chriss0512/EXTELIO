/**
 * Flow Editor - Kapitel 2.14.
 * Diese Ausbaustufe zeigt den typisierten Graphen als Diagramm, validiert ihn
 * serverseitig und veroeffentlicht ihn. Das freie Zeichnen von Kanten folgt.
 */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { Flow, FlowValidation, ListResponse } from "../lib/api";
import {
  Badge, Button, Dialog, EmptyState, ErrorState, Field, LoadingBlock, Notice,
  PageHeader, TextInput,
} from "../components/ui";

const NODE_W = 168;
const NODE_H = 52;

function FlowDiagram({ flow }: { flow: Flow }) {
  const nodes = flow.graph.nodes;
  if (nodes.length === 0) {
    return <div className="state-block"><span className="state-text">Dieser Flow enthält noch keine Knoten.</span></div>;
  }
  const positioned = nodes.map((n, i) => ({
    ...n,
    px: n.x > 0 ? n.x : 40 + (i % 3) * (NODE_W + 70),
    py: n.y > 0 ? n.y : 40 + Math.floor(i / 3) * (NODE_H + 70),
  }));
  const width = Math.max(...positioned.map((n) => n.px)) + NODE_W + 60;
  const height = Math.max(...positioned.map((n) => n.py)) + NODE_H + 60;

  return (
    <div className="flow-canvas">
      <svg width={width} height={height} role="img" aria-label={`Flow ${flow.name}`}>
        {flow.graph.edges.map((e, i) => {
          const from = positioned.find((n) => n.id === e.from);
          const to = positioned.find((n) => n.id === e.to);
          if (!from || !to) return null;
          const x1 = from.px + NODE_W;
          const y1 = from.py + NODE_H / 2;
          const x2 = to.px;
          const y2 = to.py + NODE_H / 2;
          const mid = (x1 + x2) / 2;
          return (
            <g key={`${e.from}-${e.port}-${i}`}>
              <path className="flow-edge" d={`M ${x1} ${y1} C ${mid} ${y1}, ${mid} ${y2}, ${x2} ${y2}`} />
              <text className="flow-edge-label" x={mid} y={(y1 + y2) / 2 - 5} textAnchor="middle">{e.port}</text>
            </g>
          );
        })}
        {positioned.map((n) => (
          <g key={n.id}>
            <rect className="flow-node-box" data-kind={n.kind} x={n.px} y={n.py} width={NODE_W} height={NODE_H} rx="8" />
            <text className="flow-node-label" x={n.px + 12} y={n.py + 22}>{n.label || n.kind}</text>
            <text className="flow-node-sub" x={n.px + 12} y={n.py + 38}>{n.kind}</text>
          </g>
        ))}
      </svg>
    </div>
  );
}

export function FlowsPage({ notify }: { notify: (t: string) => void }) {
  const [items, setItems] = useState<Flow[] | null>(null);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [validation, setValidation] = useState<FlowValidation | null>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");

  async function load() {
    setError("");
    try {
      const res = await api.get<ListResponse<Flow>>("/flows");
      setItems(res.items);
      if (!selected && res.items.length > 0) setSelected(res.items[0].id);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function validate(id: string) {
    try {
      setValidation(await api.post<FlowValidation>(`/flows/${id}/validate`));
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  async function publish(id: string) {
    try {
      await api.post(`/flows/${id}/publish`);
      notify("Flow veröffentlicht");
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  async function create() {
    try {
      await api.post("/flows", { name, graph: { nodes: [], edges: [] } });
      setCreating(false);
      setName("");
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={4} />;

  const current = items.find((f) => f.id === selected) ?? null;

  return (
    <>
      <PageHeader
        title="Flows"
        subtitle="Routing ist ein typisierter Graph, kein Wählplan aus Textzeilen."
        actions={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Flow anlegen</Button>}
      />

      {items.length === 0 ? (
        <EmptyState
          icon="flow"
          title="Noch kein Flow"
          text="Ein Flow beschreibt, was mit einem eingehenden Anruf passiert: Zeitsteuerung, Gruppe, Ansage, Anrufbeantworter."
          action={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Flow anlegen</Button>}
        />
      ) : (
        <div className="stack">
          <div className="row-wrap">
            {items.map((f) => (
              <Button key={f.id} variant={f.id === selected ? "primary" : "secondary"} onClick={() => { setSelected(f.id); setValidation(null); }}>
                {f.name}
              </Button>
            ))}
          </div>

          {current ? (
            <section className="panel">
              <div className="panel-header">
                <div className="row">
                  <h2 className="panel-title">{current.name}</h2>
                  <Badge tone={current.status === "published" ? "success" : "neutral"}>
                    {current.status === "published" ? "Veröffentlicht" : "Entwurf"}
                  </Badge>
                  <span className="metadata">Version {current.version}</span>
                </div>
                <div className="row">
                  <Button small onClick={() => void validate(current.id)}>Prüfen</Button>
                  <Button small variant="primary" onClick={() => void publish(current.id)}>Veröffentlichen</Button>
                </div>
              </div>
              <div className="panel-body stack">
                {validation ? (
                  <div className="stack-sm">
                    {validation.valid ? (
                      <Notice tone="success">Der Flow ist gültig.</Notice>
                    ) : (
                      <Notice tone="error">{validation.errors.join(" · ")}</Notice>
                    )}
                    {validation.warnings.length > 0 ? (
                      <Notice tone="warning">{validation.warnings.join(" · ")}</Notice>
                    ) : null}
                    {validation.simulation.length > 0 ? (
                      <div className="stack-sm">
                        <span className="panel-title">Simulierter Anrufweg</span>
                        <pre className="code-block">{validation.simulation.join("\n  ↓ ")}</pre>
                      </div>
                    ) : null}
                  </div>
                ) : null}
                <FlowDiagram flow={current} />
                <div className="legend">
                  <span>Startknoten sind blau umrandet.</span>
                  <span>Kantenbeschriftungen zeigen den Ausgang des Knotens.</span>
                </div>
              </div>
            </section>
          ) : null}
        </div>
      )}

      {creating ? (
        <Dialog
          title="Flow anlegen"
          onClose={() => setCreating(false)}
          footer={
            <>
              <Button onClick={() => setCreating(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={create} disabled={!name}>Anlegen</Button>
            </>
          }
        >
          <Field label="Name" help="Zum Beispiel: Zentrale Geschäftszeiten.">
            <TextInput value={name} onChange={setName} />
          </Field>
        </Dialog>
      ) : null}
    </>
  );
}
