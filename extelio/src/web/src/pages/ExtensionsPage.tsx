/**
 * Nebenstellen - Kapitel 2.11: Liste -> Auswahl -> Detailpanel.
 * User != Extension != Device (Kapitel 8.1).
 */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { Extension, ListResponse } from "../lib/api";
import {
  Badge,
  Button,
  DetailPanel,
  Dialog,
  EmptyState,
  ErrorState,
  Field,
  KeyValue,
  LoadingBlock,
  Notice,
  PageHeader,
  TextInput,
  Toggle,
} from "../components/ui";

interface Created {
  number: string;
  sip_username: string;
  sip_realm: string;
  sip_password: string;
}

export function ExtensionsPage({ search, notify }: { search: string; notify: (t: string) => void }) {
  const [items, setItems] = useState<Extension[] | null>(null);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [tab, setTab] = useState("Übersicht");
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<Created | null>(null);
  const [form, setForm] = useState({ number: "", name: "", voicemail: true });
  const [formError, setFormError] = useState("");

  async function load() {
    setError("");
    try {
      const res = await api.get<ListResponse<Extension>>("/extensions");
      setItems(res.items);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => {
    void load();
  }, []);

  async function create() {
    setFormError("");
    try {
      const res = await api.post<Created>("/extensions", form);
      setCreated(res);
      setCreating(false);
      setForm({ number: "", name: "", voicemail: true });
      await load();
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  async function remove(id: string) {
    try {
      await api.del(`/extensions/${id}`);
      setSelected(null);
      notify("Nebenstelle gelöscht");
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  async function toggleDnd(ext: Extension) {
    try {
      await api.put(`/extensions/${ext.id}`, { dnd: !ext.dnd });
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={6} />;

  const term = search.trim().toLowerCase();
  const filtered = term
    ? items.filter((i) => i.number.includes(term) || i.name.toLowerCase().includes(term))
    : items;
  const current = items.find((i) => i.id === selected) ?? null;

  return (
    <>
      <PageHeader
        title="Nebenstellen"
        subtitle={`${items.length} angelegt`}
        actions={
          <Button variant="primary" icon="plus" onClick={() => setCreating(true)}>
            Nebenstelle anlegen
          </Button>
        }
      />

      {created ? (
        <div className="stack">
          <Notice tone="success">
            Nebenstelle {created.number} angelegt. Das SIP-Passwort wird nur jetzt angezeigt.
          </Notice>
          <div className="panel panel-pad stack-sm">
            <span className="panel-title">Zugangsdaten für das Endgerät</span>
            <pre className="code-block">
              {`Benutzername: ${created.sip_username}\nRealm:        ${created.sip_realm}\nPasswort:     ${created.sip_password}`}
            </pre>
            <div className="row">
              <div className="spacer" />
              <Button onClick={() => setCreated(null)}>Verstanden</Button>
            </div>
          </div>
        </div>
      ) : null}

      {items.length === 0 ? (
        <EmptyState
          icon="extension"
          title="Noch keine Nebenstellen"
          text="Lege die erste Nebenstelle an. EXTELIO erzeugt dabei automatisch ein starkes SIP-Passwort."
          action={
            <Button variant="primary" icon="plus" onClick={() => setCreating(true)}>
              Nebenstelle anlegen
            </Button>
          }
        />
      ) : (
        <div className={current ? "split" : ""}>
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>Nummer</th>
                  <th>Name</th>
                  <th>Art</th>
                  <th>Geräte</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((e) => (
                  <tr
                    key={e.id}
                    data-selected={e.id === selected ? "true" : "false"}
                    onClick={() => setSelected(e.id)}
                  >
                    <td className="cell-strong">{e.number}</td>
                    <td>{e.name}</td>
                    <td className="cell-muted">{e.kind === "user" ? "Benutzer" : "Funktion"}</td>
                    <td className="cell-muted">{e.device_count}</td>
                    <td>
                      {e.dnd ? (
                        <Badge tone="warning">Nicht stören</Badge>
                      ) : (
                        <Badge tone="success">Aktiv</Badge>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {current ? (
            <DetailPanel
              title={current.name}
              subtitle={`Nebenstelle ${current.number}`}
              tabs={["Übersicht", "Konfiguration", "Geräte"]}
              activeTab={tab}
              onTab={setTab}
              onClose={() => setSelected(null)}
            >
              {tab === "Übersicht" ? (
                <KeyValue
                  items={[
                    ["Nummer", current.number],
                    ["Art", current.kind],
                    ["Anrufbeantworter", current.voicemail ? "aktiv" : "inaktiv"],
                    ["Umleitung", current.forward_target ?? "keine"],
                    ["Ausgehende Nummer", current.outbound_caller_id ?? "Standard"],
                  ]}
                />
              ) : null}

              {tab === "Konfiguration" ? (
                <div className="stack">
                  <Toggle
                    checked={current.dnd}
                    onChange={() => void toggleDnd(current)}
                    label="Nicht stören"
                  />
                  <Button variant="destructive" onClick={() => void remove(current.id)}>
                    Nebenstelle löschen
                  </Button>
                  <span className="field-help">
                    Beim Löschen wird das zugehörige SIP-Passwort kryptografisch unbrauchbar
                    gemacht.
                  </span>
                </div>
              ) : null}

              {tab === "Geräte" ? (
                <span className="metadata">
                  {current.device_count === 0
                    ? "Dieser Nebenstelle ist kein Gerät zugeordnet."
                    : `${current.device_count} Gerät(e) zugeordnet.`}
                </span>
              ) : null}
            </DetailPanel>
          ) : null}
        </div>
      )}

      {creating ? (
        <Dialog
          title="Nebenstelle anlegen"
          onClose={() => setCreating(false)}
          footer={
            <>
              <Button onClick={() => setCreating(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={create} disabled={!form.number || !form.name}>
                Anlegen
              </Button>
            </>
          }
        >
          {formError ? <Notice tone="error">{formError}</Notice> : null}
          <Field label="Nummer" help="2 bis 8 Ziffern, z. B. 201.">
            <TextInput
              value={form.number}
              onChange={(v) => setForm({ ...form, number: v })}
              inputMode="numeric"
            />
          </Field>
          <Field label="Name">
            <TextInput value={form.name} onChange={(v) => setForm({ ...form, name: v })} />
          </Field>
          <Toggle
            checked={form.voicemail}
            onChange={(v) => setForm({ ...form, voicemail: v })}
            label="Anrufbeantworter einrichten"
          />
        </Dialog>
      ) : null}
    </>
  );
}
