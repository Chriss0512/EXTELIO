/** Trunks und Provider - Kapitel 10. */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { ListResponse, ProviderProfile, Trunk } from "../lib/api";
import {
  Advanced, Badge, Button, Dialog, EmptyState, ErrorState, Field, LoadingBlock,
  Notice, PageHeader, Select, TextInput, formatDate,
} from "../components/ui";

export function TrunksPage({ notify }: { notify: (t: string) => void }) {
  const [items, setItems] = useState<Trunk[] | null>(null);
  const [providers, setProviders] = useState<ProviderProfile[]>([]);
  const [error, setError] = useState("");
  const [creating, setCreating] = useState(false);
  const [formError, setFormError] = useState("");
  const [form, setForm] = useState({
    name: "", provider_profile_id: "", mode: "register", host: "",
    auth_username: "", password: "", transport: "udp", from_user: "",
  });

  async function load() {
    setError("");
    try {
      const [t, p] = await Promise.all([
        api.get<ListResponse<Trunk>>("/trunks"),
        api.get<ListResponse<ProviderProfile>>("/providers"),
      ]);
      setItems(t.items);
      setProviders(p.items);
      if (!form.provider_profile_id && p.items.length > 0) {
        setForm((f) => ({ ...f, provider_profile_id: p.items[0].id }));
      }
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function create() {
    setFormError("");
    try {
      await api.post("/trunks", form);
      setCreating(false);
      notify("Trunk angelegt");
      await load();
    } catch (e) {
      setFormError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={4} />;

  return (
    <>
      <PageHeader
        title="Trunks"
        subtitle="Verbindungen zu SIP-Providern"
        actions={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Trunk anlegen</Button>}
      />

      {items.length === 0 ? (
        <EmptyState
          icon="trunk"
          title="Noch kein Trunk"
          text="Ohne Trunk sind nur interne Gespräche möglich. Lege die Verbindung zu deinem Provider an."
          action={<Button variant="primary" icon="plus" onClick={() => setCreating(true)}>Trunk anlegen</Button>}
        />
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr><th>Name</th><th>Provider</th><th>Ziel</th><th>Modus</th><th>Status</th><th>Zuletzt geprüft</th></tr>
            </thead>
            <tbody>
              {items.map((t) => (
                <tr key={t.id}>
                  <td className="cell-strong">{t.name}</td>
                  <td className="cell-muted">{t.provider_name}</td>
                  <td className="mono">{t.host}:{t.port} / {t.transport.toUpperCase()}</td>
                  <td className="cell-muted">{t.mode === "register" ? "Registrierung" : "Feste IP"}</td>
                  <td>
                    <Badge tone={t.status === "registered" ? "success" : t.enabled ? "warning" : "neutral"}>
                      {t.status === "registered" ? "Registriert" : t.enabled ? "Nicht registriert" : "Deaktiviert"}
                    </Badge>
                  </td>
                  <td className="cell-muted">{formatDate(t.last_status_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {creating ? (
        <Dialog
          title="Trunk anlegen"
          onClose={() => setCreating(false)}
          footer={
            <>
              <Button onClick={() => setCreating(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={create} disabled={!form.name || !form.host}>Anlegen</Button>
            </>
          }
        >
          {formError ? <Notice tone="error">{formError}</Notice> : null}
          <Field label="Name">
            <TextInput value={form.name} onChange={(v) => setForm({ ...form, name: v })} />
          </Field>
          <Field label="Providerprofil">
            <Select
              value={form.provider_profile_id}
              onChange={(v) => setForm({ ...form, provider_profile_id: v })}
              options={providers.map((p) => ({ value: p.id, label: p.name }))}
            />
          </Field>
          <Field label="Registrar oder Proxy" help="Hostname oder IP-Adresse des Providers.">
            <TextInput value={form.host} onChange={(v) => setForm({ ...form, host: v })} />
          </Field>
          <Field label="Anmeldeart">
            <Select
              value={form.mode}
              onChange={(v) => setForm({ ...form, mode: v })}
              options={[
                { value: "register", label: "Registrierung mit Zugangsdaten" },
                { value: "static_ip", label: "Feste IP ohne Registrierung" },
              ]}
            />
          </Field>
          {form.mode === "register" ? (
            <>
              <Field label="Benutzername">
                <TextInput value={form.auth_username} onChange={(v) => setForm({ ...form, auth_username: v })} />
              </Field>
              <Field label="Passwort" help="Wird sofort verschlüsselt abgelegt und nie wieder angezeigt.">
                <TextInput value={form.password} onChange={(v) => setForm({ ...form, password: v })} type="password" />
              </Field>
            </>
          ) : null}
          <Advanced>
            <Field label="Transport">
              <Select
                value={form.transport}
                onChange={(v) => setForm({ ...form, transport: v })}
                options={[{ value: "udp", label: "UDP" }, { value: "tcp", label: "TCP" }, { value: "tls", label: "TLS" }]}
              />
            </Field>
            <Field label="From-User" help="Nur setzen, wenn der Provider das ausdrücklich verlangt.">
              <TextInput value={form.from_user} onChange={(v) => setForm({ ...form, from_user: v })} />
            </Field>
          </Advanced>
        </Dialog>
      ) : null}
    </>
  );
}
