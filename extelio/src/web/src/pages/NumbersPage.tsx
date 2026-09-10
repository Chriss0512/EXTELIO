/** Rufnummern - Kapitel 8.2: canonical, display, source, type, country. */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { Flow, ListResponse, PhoneNumber, Trunk } from "../lib/api";
import {
  Badge, Button, Dialog, EmptyState, ErrorState, Field, LoadingBlock, Notice,
  PageHeader, Select, TextInput,
} from "../components/ui";

export function NumbersPage({ notify }: { notify: (t: string) => void }) {
  const [items, setItems] = useState<PhoneNumber[] | null>(null);
  const [trunks, setTrunks] = useState<Trunk[]>([]);
  const [flows, setFlows] = useState<Flow[]>([]);
  const [error, setError] = useState("");
  const [creating, setCreating] = useState(false);
  const [formError, setFormError] = useState("");
  const [form, setForm] = useState({ number: "", country: "DE", trunk_id: "", route_graph_id: "" });

  async function load() {
    setError("");
    try {
      const [n, t, f] = await Promise.all([
        api.get<ListResponse<PhoneNumber>>("/numbers"),
        api.get<ListResponse<Trunk>>("/trunks"),
        api.get<ListResponse<Flow>>("/flows"),
      ]);
      setItems(n.items); setTrunks(t.items); setFlows(f.items);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function create() {
    setFormError("");
    try {
      await api.post("/numbers", form);
      setCreating(false);
      setForm({ number: "", country: "DE", trunk_id: "", route_graph_id: "" });
      notify("Rufnummer angelegt");
      await load();
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  async function assignFlow(id: string, flowId: string) {
    try {
      await api.put(`/numbers/${id}`, { route_graph_id: flowId || null });
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={4} />;

  return (
    <>
      <PageHeader
        title="Rufnummern"
        subtitle="Externe Nummern werden nach E.164 normalisiert."
        actions={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Rufnummer anlegen</Button>}
      />

      {items.length === 0 ? (
        <EmptyState
          icon="number"
          title="Noch keine Rufnummern"
          text="Trage die Nummern deines Anschlusses ein und verbinde sie mit einem Flow."
          action={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Rufnummer anlegen</Button>}
        />
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr><th>Nummer</th><th>Land</th><th>Trunk</th><th>Flow</th><th>Eingabe</th></tr>
            </thead>
            <tbody>
              {items.map((n) => (
                <tr key={n.id}>
                  <td className="cell-strong">{n.display}</td>
                  <td className="cell-muted">{n.country ?? "–"}</td>
                  <td>{n.trunk_name ? <Badge tone="info">{n.trunk_name}</Badge> : <span className="cell-muted">nicht zugeordnet</span>}</td>
                  <td>
                    <Select
                      value={n.route_graph_id ?? ""}
                      onChange={(v) => void assignFlow(n.id, v)}
                      options={[{ value: "", label: "Kein Flow" }].concat(
                        flows.map((f) => ({ value: f.id, label: f.name })),
                      )}
                    />
                  </td>
                  <td className="cell-muted mono">{n.source_repr}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {creating ? (
        <Dialog
          title="Rufnummer anlegen"
          onClose={() => setCreating(false)}
          footer={
            <>
              <Button onClick={() => setCreating(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={create} disabled={!form.number}>Anlegen</Button>
            </>
          }
        >
          {formError ? <Notice tone="error">{formError}</Notice> : null}
          <Field label="Rufnummer" help="Nationale Schreibweise oder E.164, z. B. 0511 123456 oder +49511123456.">
            <TextInput value={form.number} onChange={(v) => setForm({ ...form, number: v })} inputMode="tel" />
          </Field>
          <Field label="Land">
            <Select
              value={form.country}
              onChange={(v) => setForm({ ...form, country: v })}
              options={[
                { value: "DE", label: "Deutschland" }, { value: "AT", label: "Österreich" },
                { value: "CH", label: "Schweiz" }, { value: "NL", label: "Niederlande" },
                { value: "FR", label: "Frankreich" }, { value: "GB", label: "Großbritannien" },
              ]}
            />
          </Field>
          <Field label="Trunk">
            <Select
              value={form.trunk_id}
              onChange={(v) => setForm({ ...form, trunk_id: v })}
              options={[{ value: "", label: "Später zuordnen" }].concat(
                trunks.map((t) => ({ value: t.id, label: t.name })),
              )}
            />
          </Field>
        </Dialog>
      ) : null}
    </>
  );
}
