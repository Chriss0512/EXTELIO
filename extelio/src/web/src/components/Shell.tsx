/**
 * Grundlayout - Kapitel 2.10.
 * Permanente linke Navigation, reduzierte Topbar.
 * Navigationsgruppen exakt nach Spezifikation: COMMUNICATE / MANAGE / SYSTEM.
 */
import type { ReactNode } from "react";
import { Icon } from "../design/icons";
import type { IconName } from "../design/icons";
import { HealthBadge, IconButton } from "./ui";
import type { SessionInfo } from "../lib/api";

export interface NavEntry {
  id: string;
  label: string;
  icon: IconName;
}

export interface NavGroup {
  title: string;
  entries: NavEntry[];
}

export const NAVIGATION: NavGroup[] = [
  {
    title: "Communicate",
    entries: [
      { id: "dashboard", label: "Übersicht", icon: "dashboard" },
      { id: "people", label: "Personen", icon: "user" },
      { id: "dialer", label: "Wählhilfe", icon: "call" },
      { id: "messages", label: "Nachrichten", icon: "message" },
      { id: "calls", label: "Gespräche", icon: "call" },
      { id: "voicemail", label: "Anrufbeantworter", icon: "voicemail" },
    ],
  },
  {
    title: "Manage",
    entries: [
      { id: "users", label: "Benutzer", icon: "user" },
      { id: "extensions", label: "Nebenstellen", icon: "extension" },
      { id: "devices", label: "Geräte", icon: "device" },
      { id: "numbers", label: "Rufnummern", icon: "number" },
      { id: "routing", label: "Routing", icon: "route" },
      { id: "groups", label: "Gruppen", icon: "group" },
      { id: "trunks", label: "Trunks", icon: "trunk" },
      { id: "flows", label: "Flows", icon: "flow" },
      { id: "provisioning", label: "Provisionierung", icon: "provisioning" },
    ],
  },
  {
    title: "System",
    entries: [
      { id: "overview", label: "Systemstatus", icon: "dashboard" },
      { id: "network", label: "Netzwerk", icon: "network" },
      { id: "security", label: "Sicherheit", icon: "security" },
      { id: "updates", label: "Aktualisierungen", icon: "update" },
      { id: "logs", label: "Protokolle", icon: "logs" },
      { id: "settings", label: "Einstellungen", icon: "settings" },
    ],
  },
];

export function Wordmark() {
  return (
    <span className="row">
      <svg width="22" height="22" viewBox="0 0 32 32" fill="none" aria-hidden="true" focusable="false">
        <rect x="1.5" y="1.5" width="29" height="29" rx="8" stroke="var(--accent)" strokeWidth="2" />
        <path
          d="M10 10h12M10 16h8M10 22h12"
          stroke="var(--accent)"
          strokeWidth="2.4"
          strokeLinecap="round"
        />
        <circle cx="22.5" cy="16" r="2.2" fill="var(--accent)" />
      </svg>
      <span className="wordmark brand-word">Extelio</span>
    </span>
  );
}

export function Shell({
  active,
  onNavigate,
  session,
  healthState,
  onLogout,
  search,
  onSearch,
  children,
}: {
  active: string;
  onNavigate: (id: string) => void;
  session: SessionInfo;
  healthState: string;
  onLogout: () => void;
  search: string;
  onSearch: (v: string) => void;
  children: ReactNode;
}) {
  return (
    <div className="app-shell">
      <div className="brand-cell">
        <Wordmark />
      </div>

      <header className="topbar">
        <label className="row">
          <Icon name="search" size={16} />
          <input
            className="search-input"
            type="search"
            value={search}
            placeholder="Nebenstelle, Gerät oder Rufnummer suchen"
            onChange={(e) => onSearch(e.target.value)}
            aria-label="Suche"
          />
        </label>
        <div className="topbar-right">
          <HealthBadge state={healthState} />
          <span className="metadata">
            {session.display_name} · {session.role}
          </span>
          <IconButton icon="logout" label="Abmelden" onClick={onLogout} />
        </div>
      </header>

      <nav className="sidebar" aria-label="Hauptnavigation">
        {NAVIGATION.map((group) => (
          <div className="nav-group" key={group.title}>
            <div className="nav-group-title">{group.title}</div>
            {group.entries.map((entry) => (
              <button
                className="nav-item"
                data-active={active === entry.id ? "true" : "false"}
                key={entry.id}
                onClick={() => onNavigate(entry.id)}
                type="button"
              >
                <Icon name={entry.icon} size={17} />
                <span className="sidebar-label">{entry.label}</span>
              </button>
            ))}
          </div>
        ))}
      </nav>

      <main className="main">{children}</main>
    </div>
  );
}
