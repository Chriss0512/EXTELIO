/** Geraete - Kapitel 2.12 und 2.17: Detailansicht mit den Pflicht-Tabs. */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { Device, Extension, ListResponse } from "../lib/api";
import {
  Badge, Button, DetailPanel, Dialog, EmptyState, ErrorState, Field, KeyValue,
  LoadingBlock, Notice, PageHeader, Presence, Select, TextInput, formatDate,
} from "../components/ui";

const DEVICE_TABS = ["Overview", "Configuration", "Lines", "Network", "Provisioning", "Logs"];

export function DevicesPage({ search, notify }: { search: string; notify: (t: string) => void }) {
  const [items, setItems] = useState<Device[] | null>(null);
  const [extensions, setExtensions] = useState<Extension[]>([]);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [tab, setTab] = useState(DEVICE_TABS[0]);
  const [creating, setCreating] = useState(false);
  const [formError, setFormError] = useState("");
  const [form, setForm] = useState({ name: "", vendor: "generic", mac: "", extension_id: "" });

  async function load() {
    setError("");
    try {
      const [d, e] = await Promise.all([
        api.get<ListResponse<Device>>("/devices"),
        api.get<ListResponse<Extension>>("/extensions"),
      ]);
      setItems(d.items);
      setExtensions(e.items);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function create() {
    setFormError("");
    try {
      await api.post("/devices", form);
      setCreating(false);
      setForm({ name: "", vendor: "generic", mac: "", extension_id: "" });
      notify("Gerät angelegt");
      await load();
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={5} />;

  const term = search.trim().toLowerCase();
  const filtered = term ? items.filter((d) => d.name.toLowerCase().includes(term)) : items;
  const current = items.find((d) => d.id === selected) ?? null;

  return (
    <>
      <PageHeader
        title="Geräte"
        subtitle={`${items.filter((d) => d.status === "online").length} von ${items.length} online`}
        actions={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Gerät hinzufügen</Button>}
      />

      {items.length === 0 ? (
        <EmptyState
          icon="device"
          title="Noch keine Geräte"
          text="Registriere Tischtelefone, DECT-Basen oder Softphones und weise ihnen Leitungen zu."
          action={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Gerät hinzufügen</Button>}
        />
      ) : (
        <div className={current ? "split" : ""}>
          <div className="grid-cards">
            {filtered.map((d) => (
              <button
                className="device-card"
                data-selected={d.id === selected ? "true" : "false"}
                key={d.id}
                onClick={() => setSelected(d.id)}
                type="button"
              >
                <Presence status={d.status} />
                <span className="stack-sm">
                  <span className="cell-strong">{d.name}</span>
                  <span className="metadata">{d.vendor} · {d.model}</span>
                  <span className="metadata">
                    {d.lines.length === 0 ? "Keine Leitung" : `Leitung ${d.lines.map((l) => l.extension_number ?? "?").join(", ")}`}
                  </span>
                </span>
              </button>
            ))}
          </div>

          {current ? (
            <DetailPanel
              title={current.name}
              subtitle={`${current.vendor} ${current.model}`}
              tabs={DEVICE_TABS}
              activeTab={tab}
              onTab={setTab}
              onClose={() => setSelected(null)}
            >
              {tab === "Overview" ? (
                <KeyValue items={[
                  ["Status", <Badge key="s" tone={current.status === "online" ? "success" : "neutral"}>{current.status}</Badge>],
                  ["Zuletzt gesehen", formatDate(current.last_seen_at)],
                  ["Registrierung", current.enrollment_state],
                  ["Firmware", current.firmware ?? "unbekannt"],
                ]} />
              ) : null}
              {tab === "Configuration" ? (
                <KeyValue items={[
                  ["Hersteller", current.vendor],
                  ["Modell", current.model],
                  ["Gerätetyp", current.device_type],
                  ["Provisionierung", current.provisioning_mode === "managed" ? "verwaltet" : "manuell"],
                ]} />
              ) : null}
              {tab === "Lines" ? (
                current.lines.length === 0 ? (
                  <span className="metadata">Diesem Gerät ist keine Leitung zugeordnet.</span>
                ) : (
                  <KeyValue items={current.lines.map((l) => [
                    `Leitung ${l.line_no}`, l.extension_number ?? "nicht zugewiesen",
                  ])} />
                )
              ) : null}
              {tab === "Network" ? (
                <KeyValue items={[["IP-Adresse", current.ip ?? "unbekannt"], ["MAC-Adresse", current.mac ?? "nicht hinterlegt"]]} />
              ) : null}
              {tab === "Provisioning" ? (
                <div className="stack-sm">
                  <span className="metadata">
                    Die automatische Provisionierung erfordert eine Enrollment-Freigabe. Ohne
                    Freigabe werden keine Konfigurationen ausgeliefert.
                  </span>
                  <Badge tone={current.enrollment_state === "enrolled" ? "success" : "warning"}>
                    {current.enrollment_state}
                  </Badge>
                </div>
              ) : null}
              {tab === "Logs" ? (
                <span className="metadata">
                  Geräteprotokolle erscheinen hier, sobald der Telefonie-Core Ereignisse liefert.
                </span>
              ) : null}
            </DetailPanel>
          ) : null}
        </div>
      )}

      {creating ? (
        <Dialog
          title="Gerät hinzufügen"
          onClose={() => setCreating(false)}
          footer={
            <>
              <Button onClick={() => setCreating(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={create} disabled={!form.name}>Hinzufügen</Button>
            </>
          }
        >
          {formError ? <Notice tone="error">{formError}</Notice> : null}
          <Field label="Name">
            <TextInput value={form.name} onChange={(v) => setForm({ ...form, name: v })} />
          </Field>
          <Field label="Hersteller">
            <TextInput value={form.vendor} onChange={(v) => setForm({ ...form, vendor: v })} />
          </Field>
          <Field label="MAC-Adresse" help="Optional, zwölf Hexadezimalstellen.">
            <TextInput value={form.mac} onChange={(v) => setForm({ ...form, mac: v })} />
          </Field>
          <Field label="Leitung 1">
            <Select
              value={form.extension_id}
              onChange={(v) => setForm({ ...form, extension_id: v })}
              options={[{ value: "", label: "Keine Zuordnung" }].concat(
                extensions.map((e) => ({ value: e.id, label: `${e.number} · ${e.name}` })),
              )}
            />
          </Field>
        </Dialog>
      ) : null}
    </>
  );
}
