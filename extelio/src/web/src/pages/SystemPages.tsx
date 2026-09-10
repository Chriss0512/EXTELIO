/**
 * Systembereich - Kapitel 18 (Status), 5 (Netzwerk), 9 (Generationen),
 * 14.4 (Protokolle), 15 (Backup), 19 (Einstellungen).
 */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type {
  AuditEntry, BackupItem, Generation, HealthReport, ListResponse, SettingsData, UserItem,
} from "../lib/api";
import {
  Badge, Button, Dialog, ErrorState, Field, HealthBadge, KeyValue, LoadingBlock,
  Notice, PageHeader, TextInput, formatBytes, formatDate,
} from "../components/ui";

const CHECK_TONE: Record<string, "success" | "warning" | "critical" | "neutral" | "info"> = {
  healthy: "success", degraded: "warning", unhealthy: "critical",
  recovery: "info", maintenance: "neutral", unsafe_override_active: "critical",
};

const CHECK_LABEL: Record<string, string> = {
  healthy: "Gesund", degraded: "Eingeschränkt", unhealthy: "Gestört",
  recovery: "Erholung", maintenance: "Wartung", unsafe_override_active: "Übersteuert",
};

export function SystemOverviewPage() {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [error, setError] = useState("");

  async function load() {
    setError("");
    try {
      setReport(await api.get<HealthReport>("/system/health"));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => {
    void load();
    const t = window.setInterval(() => void load(), 30000);
    return () => window.clearInterval(t);
  }, []);

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!report) return <LoadingBlock rows={6} />;

  return (
    <>
      <PageHeader
        title="Systemstatus"
        subtitle={`Zuletzt geprüft ${formatDate(report.ts)}`}
        actions={<HealthBadge state={report.state} />}
      />
      <div className="table-wrap">
        <table className="table">
          <thead><tr><th>Prüfung</th><th>Zustand</th><th>Details</th><th className="cell-right">Latenz</th></tr></thead>
          <tbody>
            {report.checks.map((c) => (
              <tr key={c.name}>
                <td className="cell-strong">{c.name}</td>
                <td><Badge tone={CHECK_TONE[c.state] ?? "neutral"}>{CHECK_LABEL[c.state] ?? c.state}</Badge></td>
                <td className="cell-muted">{c.detail}</td>
                <td className="cell-right cell-muted">{c.latency_ms === null ? "–" : `${c.latency_ms} ms`}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

export function NetworkPage() {
  const [data, setData] = useState<SettingsData | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    api.get<SettingsData>("/system/settings")
      .then(setData)
      .catch((e: unknown) => setError(e instanceof ApiError ? e.message : "Unbekannter Fehler"));
  }, []);

  if (error) return <ErrorState message={error} />;
  if (!data) return <LoadingBlock rows={4} />;

  const p = data.runtime.sip_ports;
  return (
    <>
      <PageHeader title="Netzwerk" subtitle="Belegte Ports und Betriebsmodi dieser Installation" />
      <div className="stack">
        <section className="panel panel-pad stack-sm">
          <h2 className="panel-title">Weboberfläche</h2>
          <KeyValue items={[
            ["Modus", data.runtime.web_mode],
            ["Kanonischer Hostname", data.runtime.canonical_hostname || "nicht gesetzt"],
            ["Sicherer Kontext", data.runtime.secure_context ? "ja" : "nein"],
          ]} />
        </section>
        <section className="panel panel-pad stack-sm">
          <h2 className="panel-title">SIP und Medien</h2>
          <KeyValue items={[
            ["LOCAL", `${p.local} / ${p.local_tls} (TLS)`],
            ["PUBLIC / PUSH", data.runtime.public_push_enabled ? `${p.public} / ${p.public_tls} (TLS)` : "deaktiviert"],
            ["TRUNK", `${p.trunk} / ${p.trunk_tls} (TLS)`],
            ["RTP-Bereich", `${data.runtime.rtp_range[0]} – ${data.runtime.rtp_range[1]}`],
            ["IPv6", data.runtime.ipv6_enabled ? "aktiv" : "deaktiviert"],
          ]} />
        </section>
        <Notice tone="info">
          SIP und RTP laufen direkt über den Host und nicht über einen Reverse Proxy. Ein
          vorgelagerter Proxy bedient ausschließlich die Weboberfläche.
        </Notice>
      </div>
    </>
  );
}

export function SecurityPage({ notify }: { notify: (t: string) => void }) {
  const [generations, setGenerations] = useState<Generation[] | null>(null);
  const [active, setActive] = useState<number | null>(null);
  const [backups, setBackups] = useState<BackupItem[]>([]);
  const [ledger, setLedger] = useState<{ valid: boolean; entries: number } | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [stepUp, setStepUp] = useState(false);
  const [password, setPassword] = useState("");

  async function load() {
    setError("");
    try {
      const [g, b, l] = await Promise.all([
        api.get<{ items: Generation[]; active: number | null }>("/config/generations"),
        api.get<ListResponse<BackupItem>>("/backup"),
        api.get<{ valid: boolean; entries: number }>("/audit/verify"),
      ]);
      setGenerations(g.items); setActive(g.active); setBackups(b.items); setLedger(l);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function run(action: () => Promise<unknown>, message: string) {
    setBusy(true);
    try {
      await action();
      notify(message);
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    } finally {
      setBusy(false);
    }
  }

  async function confirmStepUp() {
    try {
      await api.post("/auth/step-up", { password });
      setStepUp(false);
      setPassword("");
      notify("Bestätigt. Kritische Aktionen sind fünf Minuten lang freigeschaltet.");
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!generations) return <LoadingBlock rows={5} />;

  return (
    <>
      <PageHeader
        title="Sicherheit"
        subtitle="Konfigurationsgenerationen, Nachweiskette und Sicherungen"
        actions={<Button onClick={() => setStepUp(true)}>Identität bestätigen</Button>}
      />

      <div className="stack">
        {ledger ? (
          ledger.valid ? (
            <Notice tone="success">
              Die Nachweiskette ist über {ledger.entries} Einträge hinweg unverändert.
            </Notice>
          ) : (
            <Notice tone="error">
              Die Nachweiskette ist unterbrochen. Bitte den Vorfall prüfen und dokumentieren.
            </Notice>
          )
        ) : null}

        <section className="panel">
          <div className="panel-header">
            <h2 className="panel-title">Konfigurationsgenerationen</h2>
            <div className="row">
              <Button small disabled={busy} onClick={() => void run(() => api.post("/config/compile"), "Konfiguration erzeugt")}>
                Erzeugen
              </Button>
              <Button small disabled={busy} onClick={() => void run(() => api.post("/config/rollback"), "Auf vorherige Generation zurückgesetzt")}>
                Zurücksetzen
              </Button>
            </div>
          </div>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Generation</th><th>Status</th><th>Prüfsumme</th><th>Erzeugt</th><th>Dateien</th><th /></tr></thead>
              <tbody>
                {generations.map((g) => (
                  <tr key={g.number}>
                    <td className="cell-strong">{g.number}</td>
                    <td>
                      <Badge tone={g.number === active ? "success" : g.status === "failed" ? "critical" : "neutral"}>
                        {g.number === active ? "Aktiv" : g.status}
                      </Badge>
                    </td>
                    <td className="mono cell-muted">{g.hash.slice(0, 12)}</td>
                    <td className="cell-muted">{formatDate(g.created_at)}</td>
                    <td className="cell-muted">{g.files}</td>
                    <td className="cell-right">
                      {g.number === active ? null : (
                        <Button small disabled={busy} onClick={() => void run(() => api.post("/config/activate", { generation: g.number }), `Generation ${g.number} aktiviert`)}>
                          Aktivieren
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
                {generations.length === 0 ? (
                  <tr><td colSpan={6} className="cell-muted">Noch keine Generation erzeugt.</td></tr>
                ) : null}
              </tbody>
            </table>
          </div>
        </section>

        <section className="panel">
          <div className="panel-header">
            <h2 className="panel-title">Sicherungen</h2>
            <Button small disabled={busy} onClick={() => void run(() => api.post("/backup"), "Sicherung erstellt")}>
              Sicherung erstellen
            </Button>
          </div>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Zeitpunkt</th><th>Art</th><th>Größe</th><th>Verschlüsselt</th><th>Prüfsumme</th></tr></thead>
              <tbody>
                {backups.map((b) => (
                  <tr key={b.id}>
                    <td className="cell-strong">{formatDate(b.created_at)}</td>
                    <td className="cell-muted">{b.kind}</td>
                    <td className="cell-muted">{formatBytes(b.size_bytes)}</td>
                    <td><Badge tone={b.encrypted ? "success" : "critical"}>{b.encrypted ? "ja" : "nein"}</Badge></td>
                    <td className="mono cell-muted">{b.sha256.slice(0, 12)}</td>
                  </tr>
                ))}
                {backups.length === 0 ? (
                  <tr><td colSpan={5} className="cell-muted">Noch keine Sicherung vorhanden.</td></tr>
                ) : null}
              </tbody>
            </table>
          </div>
        </section>
      </div>

      {stepUp ? (
        <Dialog
          title="Identität bestätigen"
          onClose={() => setStepUp(false)}
          footer={
            <>
              <Button onClick={() => setStepUp(false)}>Abbrechen</Button>
              <Button variant="primary" onClick={confirmStepUp} disabled={!password}>Bestätigen</Button>
            </>
          }
        >
          <p className="body-text">
            Kritische Aktionen verlangen eine erneute Eingabe des Passworts.
          </p>
          <Field label="Passwort">
            <TextInput value={password} onChange={setPassword} type="password" autoComplete="current-password" />
          </Field>
        </Dialog>
      ) : null}
    </>
  );
}

export function LogsPage() {
  const [items, setItems] = useState<AuditEntry[] | null>(null);
  const [filter, setFilter] = useState("");
  const [error, setError] = useState("");

  async function load(prefix: string) {
    setError("");
    try {
      const query = prefix ? `?limit=200&action=${encodeURIComponent(prefix)}` : "?limit=200";
      const res = await api.get<ListResponse<AuditEntry>>(`/audit${query}`);
      setItems(res.items);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(""); }, []);

  if (error) return <ErrorState message={error} onRetry={() => void load(filter)} />;

  return (
    <>
      <PageHeader
        title="Protokolle"
        subtitle="Sicherheitsrelevante Ereignisse mit Nachweiskette. Passwörter, Tokens und Schlüssel erscheinen hier nie."
        actions={
          <div className="row">
            <input
              className="search-input"
              value={filter}
              placeholder="Aktion filtern, z. B. auth."
              onChange={(e) => setFilter(e.target.value)}
              aria-label="Aktion filtern"
            />
            <Button onClick={() => void load(filter)}>Filtern</Button>
          </div>
        }
      />
      {!items ? <LoadingBlock rows={8} /> : (
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Nr.</th><th>Zeitpunkt</th><th>Aktion</th><th>Objekt</th><th>Ergebnis</th><th>Kettenhash</th></tr></thead>
            <tbody>
              {items.map((a) => (
                <tr key={a.id}>
                  <td className="cell-muted">{a.seq}</td>
                  <td className="cell-muted">{formatDate(a.ts)}</td>
                  <td className="cell-strong">{a.action}</td>
                  <td className="cell-muted">{a.object_type ? `${a.object_type}` : "–"}</td>
                  <td>
                    <Badge tone={a.outcome === "success" ? "success" : a.outcome === "denied" ? "warning" : "critical"}>
                      {a.outcome}
                    </Badge>
                  </td>
                  <td className="mono cell-muted">{a.entry_hash.slice(0, 12)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
}

export function SettingsPage({ notify }: { notify: (t: string) => void }) {
  const [data, setData] = useState<SettingsData | null>(null);
  const [error, setError] = useState("");
  const [draft, setDraft] = useState<Record<string, number>>({});

  async function load() {
    setError("");
    try {
      const res = await api.get<SettingsData>("/system/settings");
      setData(res);
      setDraft(Object.fromEntries(res.retention_policies.map((p) => [p.key, p.days])));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  async function save() {
    try {
      await api.put("/system/settings", {
        retention_policies: Object.entries(draft).map(([key, days]) => ({ key, days })),
      });
      notify("Einstellungen gespeichert");
      await load();
    } catch (e) {
      notify(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!data) return <LoadingBlock rows={5} />;

  return (
    <>
      <PageHeader title="Einstellungen" subtitle={`EXTELIO ${data.runtime.version}`} />
      <div className="main-narrow stack">
        <section className="panel panel-pad stack">
          <h2 className="panel-title">Aufbewahrung</h2>
          <p className="metadata">
            Jede Datenart hat eine eigene, zweckgebundene Frist. Eine unbegrenzte Aufbewahrung
            ist nicht vorgesehen.
          </p>
          {data.retention_policies.map((p) => (
            <Field key={p.key} label={p.key} help={p.purpose}>
              <TextInput
                value={String(draft[p.key] ?? p.days)}
                onChange={(v) => setDraft({ ...draft, [p.key]: Number(v) || 0 })}
                inputMode="numeric"
              />
            </Field>
          ))}
          <div className="row">
            <div className="spacer" />
            <Button variant="primary" onClick={save}>Speichern</Button>
          </div>
        </section>

        <section className="panel panel-pad stack-sm">
          <h2 className="panel-title">Datenschutz</h2>
          <Notice tone="info">
            An Home Assistant werden ausschließlich technische Zustände übertragen. Rufnummern,
            Namen, Gesprächsdaten und Aufzeichnungen verlassen EXTELIO nicht.
          </Notice>
        </section>
      </div>
    </>
  );
}

export function UsersPage({ notify }: { notify: (t: string) => void }) {
  const [items, setItems] = useState<UserItem[] | null>(null);
  const [error, setError] = useState("");

  async function load() {
    setError("");
    try {
      const res = await api.get<ListResponse<UserItem>>("/users");
      setItems(res.items);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Unbekannter Fehler");
    }
  }

  useEffect(() => { void load(); }, []);

  if (error) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!items) return <LoadingBlock rows={4} />;

  return (
    <>
      <PageHeader
        title="Benutzer"
        subtitle="Konten der Weboberfläche. Ein Benutzer ist nicht dasselbe wie eine Nebenstelle."
        actions={<Button onClick={() => notify("Neue Konten werden über die Benutzerverwaltung angelegt.")}>Hinweis</Button>}
      />
      <div className="table-wrap">
        <table className="table">
          <thead><tr><th>Anmeldename</th><th>Name</th><th>Rolle</th><th>Zwei Faktoren</th><th>Status</th><th>Letzte Anmeldung</th></tr></thead>
          <tbody>
            {items.map((u) => (
              <tr key={u.id}>
                <td className="cell-strong">{u.username}</td>
                <td>{u.display_name}</td>
                <td className="cell-muted">{u.role}</td>
                <td><Badge tone={u.totp_enabled ? "success" : "warning"}>{u.totp_enabled ? "aktiv" : "offen"}</Badge></td>
                <td className="cell-muted">{u.status}</td>
                <td className="cell-muted">{formatDate(u.last_login_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
