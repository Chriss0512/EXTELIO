/**
 * Anwendungsrahmen: Sitzungspruefung, Routing ueber den URL-Fragmentbezeichner,
 * Zuordnung der Navigationseintraege zu Seiten.
 */
import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "./lib/api";
import type { HealthReport, SessionInfo, SetupStatus } from "./lib/api";
import { Shell } from "./components/Shell";
import { LoadingBlock, ToastStack } from "./components/ui";
import type { ToastMessage } from "./components/ui";
import { SetupPage } from "./pages/SetupPage";
import { LoginPage } from "./pages/LoginPage";
import { DashboardPage } from "./pages/DashboardPage";
import { ExtensionsPage } from "./pages/ExtensionsPage";
import { DevicesPage } from "./pages/DevicesPage";
import { NumbersPage } from "./pages/NumbersPage";
import { TrunksPage } from "./pages/TrunksPage";
import { FlowsPage } from "./pages/FlowsPage";
import { GroupsPage } from "./pages/GroupsPage";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import {
  LogsPage, NetworkPage, SecurityPage, SettingsPage, SystemOverviewPage, UsersPage,
} from "./pages/SystemPages";

type Phase = "loading" | "setup" | "login" | "ready";

export function App() {
  const [phase, setPhase] = useState<Phase>("loading");
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [health, setHealth] = useState<string>("MAINTENANCE");
  const [route, setRoute] = useState(window.location.hash.slice(1) || "dashboard");
  const [search, setSearch] = useState("");
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const notify = useCallback((text: string, tone: ToastMessage["tone"] = "info") => {
    const id = Date.now() + Math.random();
    setToasts((prev) => [...prev, { id, tone, text }]);
    window.setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 4200);
  }, []);

  const bootstrap = useCallback(async () => {
    try {
      const s = await api.get<SetupStatus>("/setup/status");
      setStatus(s);
      if (s.needs_setup) {
        setPhase("setup");
        return;
      }
      const me = await api.get<SessionInfo>("/auth/session");
      setSession(me);
      setPhase("ready");
    } catch (e) {
      if (e instanceof ApiError && e.isUnauthorized) {
        setPhase("login");
        return;
      }
      setPhase("login");
    }
  }, []);

  useEffect(() => { void bootstrap(); }, [bootstrap]);

  useEffect(() => {
    const onHash = () => setRoute(window.location.hash.slice(1) || "dashboard");
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  useEffect(() => {
    if (phase !== "ready") return undefined;
    const poll = () => {
      api.get<HealthReport>("/system/health")
        .then((r) => setHealth(r.state))
        .catch(() => setHealth("DEGRADED"));
    };
    poll();
    const t = window.setInterval(poll, 30000);
    return () => window.clearInterval(t);
  }, [phase]);

  function navigate(id: string) {
    window.location.hash = id;
    setRoute(id);
  }

  async function logout() {
    try {
      await api.post("/auth/logout");
    } finally {
      setSession(null);
      setPhase("login");
    }
  }

  if (phase === "loading") {
    return <div className="centered-page"><div className="centered-card"><LoadingBlock rows={3} /></div></div>;
  }
  if (phase === "setup") {
    return <SetupPage onDone={() => { setPhase("login"); void bootstrap(); }} />;
  }
  if (phase === "login" || !session) {
    return (
      <LoginPage
        secureContext={status?.secure_context ?? false}
        onSuccess={() => { setPhase("loading"); void bootstrap(); }}
      />
    );
  }

  return (
    <>
      <Shell
        active={route}
        onNavigate={navigate}
        session={session}
        healthState={health}
        onLogout={() => void logout()}
        search={search}
        onSearch={setSearch}
      >
        {renderRoute(route, search, notify, navigate)}
      </Shell>
      <ToastStack toasts={toasts} />
    </>
  );
}

function renderRoute(
  route: string,
  search: string,
  notify: (text: string) => void,
  navigate: (id: string) => void,
) {
  switch (route) {
    case "dashboard":
      return <DashboardPage onNavigate={navigate} />;
    case "extensions":
      return <ExtensionsPage search={search} notify={notify} />;
    case "devices":
      return <DevicesPage search={search} notify={notify} />;
    case "numbers":
      return <NumbersPage notify={notify} />;
    case "trunks":
      return <TrunksPage notify={notify} />;
    case "flows":
      return <FlowsPage notify={notify} />;
    case "groups":
      return <GroupsPage notify={notify} />;
    case "users":
      return <UsersPage notify={notify} />;
    case "overview":
      return <SystemOverviewPage />;
    case "network":
      return <NetworkPage />;
    case "security":
      return <SecurityPage notify={notify} />;
    case "logs":
      return <LogsPage />;
    case "settings":
      return <SettingsPage notify={notify} />;
    case "people":
      return (
        <PlaceholderPage
          title="Personen"
          subtitle="Verzeichnis mit Präsenz"
          icon="user"
          text="Das Personenverzeichnis zeigt Präsenz und Erreichbarkeit. Es benötigt den laufenden Telefonie-Core."
        />
      );
    case "dialer":
      return (
        <PlaceholderPage
          title="Wählhilfe"
          subtitle="Anrufe aus der Oberfläche starten"
          icon="call"
          text="Die Wählhilfe steuert ein registriertes Endgerät. Sie wird aktiv, sobald der Telefonie-Core läuft."
        />
      );
    case "messages":
      return (
        <PlaceholderPage
          title="Nachrichten"
          subtitle="Interne Kurznachrichten"
          icon="message"
          text="Der Nachrichtenbereich folgt in einer späteren Ausbaustufe."
        />
      );
    case "calls":
      return (
        <PlaceholderPage
          title="Gespräche"
          subtitle="Verlauf aus dem Call Event Ledger"
          icon="call"
          text="Sobald Gespräche geführt werden, erscheint hier der Verlauf mit den geltenden Aufbewahrungsfristen."
        />
      );
    case "voicemail":
      return (
        <PlaceholderPage
          title="Anrufbeantworter"
          subtitle="Verschlüsselte Nachrichten"
          icon="voicemail"
          text="Nachrichten werden verschlüsselt gespeichert. Der Zugriff ist auf Besitzer, Vertretungen und ausdrücklich berechtigte Administratoren begrenzt."
        />
      );
    case "routing":
      return (
        <PlaceholderPage
          title="Routing"
          subtitle="Regeln für ein- und ausgehende Wege"
          icon="route"
          text="Die Routingübersicht fasst Nummern, Flows und Trunks zusammen. Bis dahin werden die Wege je Rufnummer gepflegt."
        />
      );
    case "provisioning":
      return (
        <PlaceholderPage
          title="Provisionierung"
          subtitle="Automatische Geräteeinrichtung"
          icon="provisioning"
          text="Die automatische Einrichtung liefert erst nach ausdrücklicher Freigabe eines Geräts Konfigurationen aus."
        />
      );
    case "updates":
      return (
        <PlaceholderPage
          title="Aktualisierungen"
          subtitle="Versionsstand und Hinweise"
          icon="update"
          text="Aktualisierungen werden über Home Assistant eingespielt. Der Versionsstand steht unter Einstellungen."
        />
      );
    default:
      return <DashboardPage onNavigate={navigate} />;
  }
}
