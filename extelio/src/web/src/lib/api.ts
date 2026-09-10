/**
 * API-Client.
 *
 * Kapitel 6.4: Es werden keine Auth-Tokens im localStorage gehalten. Die
 * Sitzung liegt ausschliesslich im HttpOnly-Cookie, deshalb `credentials:
 * "same-origin"` und kein manuelles Token-Handling.
 */

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
    this.name = "ApiError";
  }

  get isUnauthorized(): boolean {
    return this.status === 401;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api${path}`, {
      method,
      credentials: "same-origin",
      headers: body === undefined ? {} : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, "network", "Der Server ist nicht erreichbar.");
  }

  const text = await response.text();
  const payload: unknown = text ? JSON.parse(text) : {};

  if (!response.ok) {
    const record = payload as { error?: string; message?: string };
    throw new ApiError(
      response.status,
      record.error ?? "unknown",
      record.message ?? "Die Anfrage konnte nicht verarbeitet werden.",
    );
  }
  return payload as T;
}

export const api = {
  get: <T,>(path: string) => request<T>("GET", path),
  post: <T,>(path: string, body?: unknown) => request<T>("POST", path, body ?? {}),
  put: <T,>(path: string, body: unknown) => request<T>("PUT", path, body),
  del: <T,>(path: string) => request<T>("DELETE", path),
};

/* ------------------------------------------------------------------ Typen */

export interface SetupStatus {
  needs_setup: boolean;
  version: string;
  secure_context: boolean;
  web_mode: string;
}

export interface SessionInfo {
  user_id: string;
  username: string;
  display_name: string;
  role: string;
  permissions: string[];
  step_up_fresh: boolean;
  idle_expires_at: string;
  absolute_expires_at: string;
}

export type HealthStateName =
  | "HEALTHY"
  | "DEGRADED"
  | "UNHEALTHY"
  | "RECOVERY"
  | "UNSAFE_OVERRIDE_ACTIVE"
  | "MAINTENANCE";

export interface HealthCheck {
  name: string;
  state: string;
  detail: string;
  latency_ms: number | null;
}

export interface HealthReport {
  state: HealthStateName;
  ts: string;
  active_generation: number | null;
  checks: HealthCheck[];
}

export interface DashboardData {
  health: { state: HealthStateName; checks_total: number; checks_failing: number };
  counters: Record<string, number>;
  active_generation: number | null;
  recent_events: { ts: string; action: string; outcome: string; actor_type: string }[];
}

export interface Extension {
  id: string;
  number: string;
  name: string;
  kind: string;
  site_id: string | null;
  user_id: string | null;
  voicemail: boolean;
  dnd: boolean;
  forward_target: string | null;
  outbound_caller_id: string | null;
  device_count: number;
}

export interface DeviceLine {
  id: string;
  line_no: number;
  extension_id: string | null;
  extension_number: string | null;
}

export interface Device {
  id: string;
  name: string;
  vendor: string;
  model: string;
  mac: string | null;
  device_type: string;
  provisioning_mode: string;
  enrollment_state: string;
  firmware: string | null;
  ip: string | null;
  status: string;
  last_seen_at: string | null;
  lines: DeviceLine[];
}

export interface PhoneNumber {
  id: string;
  canonical: string;
  display: string;
  source_repr: string;
  number_type: string;
  country: string | null;
  trunk_id: string | null;
  trunk_name: string | null;
  route_graph_id: string | null;
  flow_name: string | null;
}

export interface ProviderProfile {
  id: string;
  key: string;
  name: string;
  transport: string;
  codecs: string[];
  number_format: string;
  auth_capabilities: string[];
  builtin: boolean;
}

export interface Trunk {
  id: string;
  name: string;
  provider_profile_id: string;
  provider_name: string;
  mode: string;
  host: string;
  port: number;
  transport: string;
  auth_username: string | null;
  enabled: boolean;
  status: string;
  last_status_at: string | null;
  security_profile: string;
}

export interface RingGroup {
  id: string;
  name: string;
  strategy: string;
  members: string[];
  timeout_s: number;
}

export interface FlowNode {
  id: string;
  kind: string;
  label: string;
  ref_id: string | null;
  params: Record<string, unknown>;
  x: number;
  y: number;
}

export interface FlowEdge {
  from: string;
  port: string;
  to: string;
}

export interface Flow {
  id: string;
  name: string;
  graph: { nodes: FlowNode[]; edges: FlowEdge[] };
  version: number;
  status: string;
  updated_at: string;
}

export interface FlowValidation {
  valid: boolean;
  errors: string[];
  warnings: string[];
  simulation: string[];
}

export interface AuditEntry {
  seq: number;
  id: string;
  ts: string;
  actor_type: string;
  actor_id: string | null;
  action: string;
  object_type: string | null;
  object_id: string | null;
  outcome: string;
  reason: string | null;
  detail: unknown;
  entry_hash: string;
}

export interface Generation {
  number: number;
  status: string;
  hash: string;
  created_by: string | null;
  created_at: string;
  activated_at: string | null;
  files: number;
  warnings: string[];
}

export interface BackupItem {
  id: string;
  created_at: string;
  kind: string;
  path: string;
  sha256: string;
  size_bytes: number;
  encrypted: boolean;
}

export interface UserItem {
  id: string;
  username: string;
  display_name: string;
  email: string | null;
  role: string;
  status: string;
  totp_enabled: boolean;
  created_at: string;
  last_login_at: string | null;
}

export interface SettingsData {
  settings: Record<string, unknown>;
  retention_policies: { key: string; purpose: string; days: number }[];
  runtime: {
    version: string;
    web_mode: string;
    canonical_hostname: string;
    sip_ports: Record<string, number>;
    rtp_range: [number, number];
    public_push_enabled: boolean;
    ipv6_enabled: boolean;
    secure_context: boolean;
  };
}

export interface ListResponse<T> {
  items: T[];
}
