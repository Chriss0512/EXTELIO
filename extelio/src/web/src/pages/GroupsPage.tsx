/** Rufgruppen - Kapitel 8.1. */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { Extension, ListResponse, RingGroup } from "../lib/api";
import {
  Button, Dialog, EmptyState, ErrorState, Field, LoadingBlock, Notice,
  PageHeader, Select, TextInput, Toggle,
} from "../components/ui";

export function GroupsPage({ notify }: { notify: (t: string) => void }) {
  const [items, setItems] = useState<RingGroup[] | null>(null);
  const [extensions, setExtensions] = useState<Extension[]>([]);
  const [error, setError] = useState("");
  const [creating, setCreating] = useState(false);
  const [formError, setFormError] = useState("");
  const [name, setName] = useState("");
  const [strategy, setStrategy] = useState("simultaneous");
  const [members, setMembers] = useState<string[]>([]);

  async function load() {
    setError("");
    try {
      const [g, e] = await Promise.all([
        api.get<ListResponse<RingGroup>>("/groups"),
        api.get<ListResponse<Extension>>("/extensions"),
      ]);
      setItems(g.items);
      setExtensions(e.items);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function create() {
    setFormError("");
    try {
      await api.post("/groups", { name, strategy, members });
      setCreating(false);
      setName(""); setMembers([]);
      notify("Gruppe angelegt");
      await load();
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={3} />;

  return (
    <>
      <PageHeader
        title="Gruppen"
        subtitle="Mehrere Nebenstellen gemeinsam anrufen"
        actions={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Gruppe anlegen</Button>}
      />
      {items.length === 0 ? (
        <EmptyState
          icon="group"
          title="Noch keine Gruppen"
          text="Eine Rufgruppe lässt mehrere Nebenstellen gleichzeitig oder nacheinander klingeln."
          action={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Gruppe anlegen</Button>}
        />
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Name</th><th>Strategie</th><th>Mitglieder</th><th>Zeitlimit</th></tr></thead>
            <tbody>
              {items.map((g) => (
                <tr key={g.id}>
                  <td className="cell-strong">{g.name}</td>
                  <td className="cell-muted">{g.strategy === "sequential" ? "nacheinander" : "gleichzeitig"}</td>
                  <td className="cell-muted">{g.members.length}</td>
                  <td className="cell-muted">{g.timeout_s} s</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {creating ? (
        <Dialog
          title="Gruppe anlegen"
          onClose={() => setCreating(false)}
          footer={
            <>
              <Button onClick={() => setCreating(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={create} disabled={!name || members.length === 0}>Anlegen</Button>
            </>
          }
        >
          {formError ? <Notice tone="error">{formError}</Notice> : null}
          <Field label="Name"><TextInput value={name} onChange={setName} /></Field>
          <Field label="Strategie">
            <Select
              value={strategy}
              onChange={setStrategy}
              options={[
                { value: "simultaneous", label: "Alle gleichzeitig" },
                { value: "sequential", label: "Nacheinander" },
              ]}
            />
          </Field>
          <Field label="Mitglieder">
            <div className="stack-sm">
              {extensions.map((e) => (
                <Toggle
                  key={e.id}
                  checked={members.includes(e.number)}
                  label={`${e.number} · ${e.name}`}
                  onChange={(on) =>
                    setMembers(on ? [...members, e.number] : members.filter((m) => m !== e.number))
                  }
                />
              ))}
              {extensions.length === 0 ? (
                <span className="field-help">Lege zuerst Nebenstellen an.</span>
              ) : null}
            </div>
          </Field>
        </Dialog>
      ) : null}
    </>
  );
}
