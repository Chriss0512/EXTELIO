/** Dashboard - Kapitel 2.13: Systemzustand, Telefonie-Kennzahlen, Ereignisse. */
import { useEffect, useState } from "react";
import { api, ApiError } from "../lib/api";
import type { DashboardData } from "../lib/api";
import {
  ErrorState,
  HealthBadge,
  LoadingBlock,
  PageHeader,
  StatCard,
  formatDate,
} from "../components/ui";

export function DashboardPage({ onNavigate }: { onNavigate: (id: string) => void }) {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState("");

  async function load() {
    setError("");
    try {
      setData(await api.get<DashboardData>("/dashboard"));
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
  if (!data) return <LoadingBlock rows={5} />;

  const c = data.counters;
  return (
    <>
      <PageHeader
        title="Übersicht"
        subtitle={
          data.active_generation === null
            ? "Es ist noch keine Konfigurationsgeneration aktiv."
            : `Aktive Konfigurationsgeneration ${data.active_generation}`
        }
        actions={<HealthBadge state={data.health.state} />}
      />

      <div className="stack">
        <div className="grid-cards">
          <StatCard
            label="Nebenstellen"
            value={c.extensions ?? 0}
            hint={`${c.flows_draft ?? 0} Flows im Entwurf`}
          />
          <StatCard
            label="Geräte online"
            value={`${c.devices_online ?? 0} / ${c.devices ?? 0}`}
            hint="Registrierte Endgeräte"
          />
          <StatCard
            label="Trunks registriert"
            value={`${c.trunks_registered ?? 0} / ${c.trunks ?? 0}`}
            hint="Verbindungen zum Provider"
          />
          <StatCard label="Rufnummern" value={c.numbers ?? 0} hint="Externe Nummern" />
          <StatCard label="Gespräche heute" value={c.calls_today ?? 0} hint="Aus dem Call Event Ledger" />
          <StatCard
            label="Prüfungen auffällig"
            value={`${data.health.checks_failing} / ${data.health.checks_total}`}
            hint="Health-Pflichtprüfungen"
          />
        </div>

        <section className="panel">
          <div className="panel-header">
            <h2 className="panel-title">Letzte Ereignisse</h2>
            <button className="advanced-toggle" onClick={() => onNavigate("logs")} type="button">
              Alle Protokolle
            </button>
          </div>
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>Zeitpunkt</th>
                  <th>Aktion</th>
                  <th>Ergebnis</th>
                  <th>Auslöser</th>
                </tr>
              </thead>
              <tbody>
                {data.recent_events.map((e, i) => (
                  <tr key={`${e.ts}-${i}`}>
                    <td className="cell-muted">{formatDate(e.ts)}</td>
                    <td className="cell-strong">{e.action}</td>
                    <td>{e.outcome}</td>
                    <td className="cell-muted">{e.actor_type}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </>
  );
}
