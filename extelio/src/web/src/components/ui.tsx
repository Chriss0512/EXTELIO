/**
 * Verbindlicher UI-Komponentensatz - Kapitel 2.17.
 * Alle Darstellungszustaende laufen ueber Klassen und data-Attribute,
 * damit keine Inline-Styles noetig sind (Kapitel 12, CSP).
 */
import type { ReactNode } from "react";
import { Fragment, useEffect, useState } from "react";
import { Icon } from "../design/icons";
import type { IconName } from "../design/icons";

export type Tone = "success" | "warning" | "critical" | "info" | "neutral";

/* ------------------------------------------------------------- Buttons */

interface ButtonProps {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "secondary" | "ghost" | "destructive";
  icon?: IconName;
  disabled?: boolean;
  small?: boolean;
  type?: "button" | "submit";
}

export function Button({
  children,
  onClick,
  variant = "secondary",
  icon,
  disabled,
  small,
  type = "button",
}: ButtonProps) {
  const classes = ["btn", `btn-${variant}`];
  if (small) classes.push("btn-sm");
  return (
    <button className={classes.join(" ")} onClick={onClick} disabled={disabled} type={type}>
      {icon ? <Icon name={icon} size={16} /> : null}
      {children}
    </button>
  );
}

/** Icon Button - Tooltip ist laut Kapitel 2.9 Pflicht. */
export function IconButton({
  icon,
  label,
  onClick,
  disabled,
}: {
  icon: IconName;
  label: string;
  onClick?: () => void;
  disabled?: boolean;
}) {
  return (
    <span className="tooltip-host">
      <button className="btn-icon" onClick={onClick} disabled={disabled} aria-label={label} type="button">
        <Icon name={icon} size={17} />
      </button>
      <span className="tooltip" role="tooltip">{label}</span>
    </span>
  );
}

/* -------------------------------------------------------------- Inputs */

export function Field({
  label,
  help,
  error,
  children,
}: {
  label: string;
  help?: string;
  error?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
      {help && !error ? <span className="field-help">{help}</span> : null}
      {error ? <span className="field-error">{error}</span> : null}
    </label>
  );
}

export function TextInput({
  value,
  onChange,
  placeholder,
  type = "text",
  invalid,
  autoComplete,
  inputMode,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: "text" | "password" | "email";
  invalid?: boolean;
  autoComplete?: string;
  inputMode?: "numeric" | "tel" | "text";
}) {
  return (
    <input
      className="input"
      type={type}
      value={value}
      placeholder={placeholder}
      autoComplete={autoComplete}
      inputMode={inputMode}
      aria-invalid={invalid ? "true" : undefined}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

export function Select({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <select className="select" value={value} onChange={(e) => onChange(e.target.value)}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <div className="checkbox-row">
      <button
        className="switch"
        data-on={checked ? "true" : "false"}
        onClick={() => onChange(!checked)}
        role="switch"
        aria-checked={checked}
        aria-label={label}
        type="button"
      >
        <span className="switch-knob" />
      </button>
      <span>{label}</span>
    </div>
  );
}

/** Advanced Settings sind standardmaessig eingeklappt (Kapitel 2.9). */
export function Advanced({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="stack-sm">
      <button className="advanced-toggle" onClick={() => setOpen(!open)} type="button">
        <Icon name="chevron" size={14} />
        {open ? "Erweiterte Einstellungen ausblenden" : "Erweiterte Einstellungen"}
      </button>
      {open ? <div className="stack">{children}</div> : null}
    </div>
  );
}

/* -------------------------------------------------------------- Status */

export function Badge({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span className="badge" data-tone={tone}>
      <span className="badge-dot" />
      {children}
    </span>
  );
}

const healthTone: Record<string, Tone> = {
  HEALTHY: "success",
  healthy: "success",
  DEGRADED: "warning",
  degraded: "warning",
  UNHEALTHY: "critical",
  unhealthy: "critical",
  RECOVERY: "info",
  recovery: "info",
  MAINTENANCE: "neutral",
  maintenance: "neutral",
  UNSAFE_OVERRIDE_ACTIVE: "critical",
  unsafe_override_active: "critical",
};

const healthLabel: Record<string, string> = {
  HEALTHY: "Gesund",
  healthy: "Gesund",
  DEGRADED: "Eingeschränkt",
  degraded: "Eingeschränkt",
  UNHEALTHY: "Gestört",
  unhealthy: "Gestört",
  RECOVERY: "Erholung",
  recovery: "Erholung",
  MAINTENANCE: "Wartung",
  maintenance: "Wartung",
  UNSAFE_OVERRIDE_ACTIVE: "Übersteuert",
  unsafe_override_active: "Übersteuert",
};

export function HealthBadge({ state }: { state: string }) {
  return <Badge tone={healthTone[state] ?? "neutral"}>{healthLabel[state] ?? state}</Badge>;
}

export function Presence({ status }: { status: string }) {
  return <span className="presence" data-state={status} title={status} />;
}

/* ------------------------------------------------------------- Zustände */

export function EmptyState({
  icon,
  title,
  text,
  action,
}: {
  icon: IconName;
  title: string;
  text: string;
  action?: ReactNode;
}) {
  return (
    <div className="state-block">
      <Icon name={icon} size={30} />
      <span className="state-title">{title}</span>
      <span className="state-text">{text}</span>
      {action}
    </div>
  );
}

export function LoadingBlock({ rows = 4 }: { rows?: number }) {
  return (
    <div className="stack-sm" aria-busy="true" aria-live="polite">
      {Array.from({ length: rows }, (_, i) => (
        <div className="skeleton skeleton-row" key={i} />
      ))}
    </div>
  );
}

export function Notice({ tone, children }: { tone: "error" | "warning" | "info" | "success"; children: ReactNode }) {
  return (
    <div className="notice" data-tone={tone} role={tone === "error" ? "alert" : "status"}>
      <Icon name={tone === "error" || tone === "warning" ? "warning" : "check"} size={17} />
      <span>{children}</span>
    </div>
  );
}

export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="state-block">
      <Icon name="warning" size={28} />
      <span className="state-title">Das hat nicht geklappt</span>
      <span className="state-text">{message}</span>
      {onRetry ? <Button onClick={onRetry}>Erneut versuchen</Button> : null}
    </div>
  );
}

/* ------------------------------------------------------------- Dialoge */

export function Dialog({
  title,
  children,
  onClose,
  footer,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="dialog-backdrop" role="dialog" aria-modal="true" aria-label={title}>
      <div className="dialog">
        <div className="panel-header">
          <h2 className="panel-title">{title}</h2>
          <IconButton icon="close" label="Schließen" onClick={onClose} />
        </div>
        <div className="panel-body stack">{children}</div>
        {footer ? <div className="panel-footer">{footer}</div> : null}
      </div>
    </div>
  );
}

export interface ToastMessage {
  id: number;
  tone: "success" | "error" | "info";
  text: string;
}

export function ToastStack({ toasts }: { toasts: ToastMessage[] }) {
  if (toasts.length === 0) return null;
  return (
    <div className="toast-stack" aria-live="polite">
      {toasts.map((t) => (
        <div className="toast" data-tone={t.tone} key={t.id}>
          <Icon name={t.tone === "error" ? "warning" : "check"} size={16} />
          <span>{t.text}</span>
        </div>
      ))}
    </div>
  );
}

/* ------------------------------------------------------- Seitenbausteine */

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div className="page-header-text">
        <h1 className="page-title">{title}</h1>
        {subtitle ? <span className="metadata">{subtitle}</span> : null}
      </div>
      {actions ? <div className="row">{actions}</div> : null}
    </header>
  );
}

export function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string | number;
  hint?: string;
}) {
  return (
    <div className="stat-card">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
      {hint ? <span className="stat-hint">{hint}</span> : null}
    </div>
  );
}

export function DetailPanel({
  title,
  subtitle,
  tabs,
  activeTab,
  onTab,
  onClose,
  children,
}: {
  title: string;
  subtitle?: string;
  tabs: string[];
  activeTab: string;
  onTab: (t: string) => void;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <aside className="detail-panel">
      <div className="detail-header">
        <div className="page-header-text">
          <h2 className="section-title">{title}</h2>
          {subtitle ? <span className="metadata">{subtitle}</span> : null}
        </div>
        <IconButton icon="close" label="Detailbereich schließen" onClick={onClose} />
      </div>
      <div className="detail-tabs" role="tablist">
        {tabs.map((t) => (
          <button
            className="detail-tab"
            data-active={t === activeTab ? "true" : "false"}
            key={t}
            onClick={() => onTab(t)}
            role="tab"
            aria-selected={t === activeTab}
            type="button"
          >
            {t}
          </button>
        ))}
      </div>
      <div className="detail-body">{children}</div>
    </aside>
  );
}

export function KeyValue({ items }: { items: [string, ReactNode][] }) {
  return (
    <dl className="kv">
      {items.map(([k, v]) => (
        <Fragment key={k}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </Fragment>
      ))}
    </dl>
  );
}

export function formatDate(value: string | null | undefined): string {
  if (!value) return "–";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" });
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / 1024 / 1024).toFixed(1)} MiB`;
}
