-- EXTELIO initial schema
-- Spec: Kapitel 6 (IAM), 7 (Secrets), 8 (Domain Model), 14 (Audit/Retention)
-- Domaenengrenzen: Identity | Organization | Telephony | Routing | Scheduling | Media | Audit
-- Betrieb: Single Tenant, Schema tenant-aware (tenant_id ueberall vorhanden, Default 'default').

CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    applied_at  TEXT NOT NULL
);

-- ---------------------------------------------------------------- Identity

CREATE TABLE roles (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL DEFAULT 'default',
    key         TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    permissions TEXT NOT NULL,          -- JSON array
    builtin     INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE TABLE users (
    id             TEXT PRIMARY KEY,
    tenant_id      TEXT NOT NULL DEFAULT 'default',
    username       TEXT NOT NULL UNIQUE,
    display_name   TEXT NOT NULL,
    email          TEXT,
    role_id        TEXT NOT NULL REFERENCES roles(id),
    scope_type     TEXT NOT NULL DEFAULT 'organization',
    scope_value    TEXT,
    password_hash  TEXT NOT NULL,
    password_algo  TEXT NOT NULL,       -- z.B. argon2id$v=19$m=65536,t=3,p=4
    pepper_backend TEXT NOT NULL DEFAULT 'none',
    totp_secret_id TEXT REFERENCES secrets(id),
    totp_enabled   INTEGER NOT NULL DEFAULT 0,
    status         TEXT NOT NULL DEFAULT 'active',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    last_login_at  TEXT
);

CREATE TABLE sessions (
    id                 TEXT PRIMARY KEY,
    user_id            TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash         TEXT NOT NULL UNIQUE,
    transport          TEXT NOT NULL,          -- http | https  (6.4: keine Uebernahme zwischen Transporten)
    created_at         TEXT NOT NULL,
    last_seen_at       TEXT NOT NULL,
    idle_expires_at    TEXT NOT NULL,
    absolute_expires_at TEXT NOT NULL,
    step_up_at         TEXT,
    revoked_at         TEXT,
    remote_ip          TEXT,
    user_agent         TEXT
);
CREATE INDEX idx_sessions_user ON sessions(user_id);

-- 6.8 Brute Force: Account / IP / Subnet / Auth method
CREATE TABLE auth_throttle (
    id               TEXT PRIMARY KEY,
    scope_type       TEXT NOT NULL,     -- account | ip | subnet | method
    scope_value      TEXT NOT NULL,
    auth_method      TEXT NOT NULL,
    failures         INTEGER NOT NULL DEFAULT 0,
    first_failure_at TEXT,
    last_failure_at  TEXT,
    locked_until     TEXT,
    UNIQUE (scope_type, scope_value, auth_method)
);

CREATE TABLE service_accounts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    principal   TEXT NOT NULL,
    token_hash  TEXT NOT NULL,
    scopes      TEXT NOT NULL,          -- JSON array
    created_at  TEXT NOT NULL,
    rotated_at  TEXT,
    revoked_at  TEXT
);

-- ------------------------------------------------------------- Secrets/KMS

CREATE TABLE secrets (
    id             TEXT PRIMARY KEY,
    class          TEXT NOT NULL,       -- 7.3 Secret Classes
    label          TEXT,
    scope          TEXT NOT NULL DEFAULT 'system',
    crypto_version INTEGER NOT NULL,
    algorithm      TEXT NOT NULL,       -- AES-256-GCM
    key_id         TEXT NOT NULL,
    wrapped_dek    BLOB NOT NULL,
    dek_nonce      BLOB NOT NULL,
    nonce          BLOB NOT NULL,
    ciphertext     BLOB NOT NULL,
    created_at     TEXT NOT NULL,
    rotated_at     TEXT,
    revoked_at     TEXT
);
CREATE INDEX idx_secrets_class ON secrets(class);

-- ---------------------------------------------------------------- Audit

-- 14.4 hash-chained append-only ledger
CREATE TABLE audit_log (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    id          TEXT NOT NULL UNIQUE,
    ts          TEXT NOT NULL,
    actor_type  TEXT NOT NULL,          -- user | service | system
    actor_id    TEXT,
    action      TEXT NOT NULL,
    object_type TEXT,
    object_id   TEXT,
    outcome     TEXT NOT NULL,          -- success | failure | denied
    reason      TEXT,
    detail      TEXT NOT NULL DEFAULT '{}',
    remote_ip   TEXT,
    prev_hash   TEXT NOT NULL,
    entry_hash  TEXT NOT NULL
);
CREATE INDEX idx_audit_ts ON audit_log(ts);
CREATE INDEX idx_audit_action ON audit_log(action);

CREATE TABLE audit_checkpoints (
    id        TEXT PRIMARY KEY,
    ts        TEXT NOT NULL,
    upto_seq  INTEGER NOT NULL,
    mac       TEXT NOT NULL
);

-- ------------------------------------------------------------ Organization

CREATE TABLE sites (
    id         TEXT PRIMARY KEY,
    tenant_id  TEXT NOT NULL DEFAULT 'default',
    name       TEXT NOT NULL,
    timezone   TEXT NOT NULL DEFAULT 'Europe/Berlin',
    created_at TEXT NOT NULL
);

CREATE TABLE departments (
    id         TEXT PRIMARY KEY,
    site_id    TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- ---------------------------------------------------------------- Telephony
-- 8.1: User != Extension != Device

CREATE TABLE extensions (
    id             TEXT PRIMARY KEY,
    tenant_id      TEXT NOT NULL DEFAULT 'default',
    number         TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    kind           TEXT NOT NULL DEFAULT 'user',   -- user | function | group | queue
    site_id        TEXT REFERENCES sites(id),
    department_id  TEXT REFERENCES departments(id),
    user_id        TEXT REFERENCES users(id),
    voicemail      INTEGER NOT NULL DEFAULT 1,
    dnd            INTEGER NOT NULL DEFAULT 0,
    forward_target TEXT,
    outbound_caller_id TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE sip_credentials (
    id           TEXT PRIMARY KEY,
    username     TEXT NOT NULL UNIQUE,
    realm        TEXT NOT NULL,
    secret_id    TEXT NOT NULL REFERENCES secrets(id),
    extension_id TEXT REFERENCES extensions(id) ON DELETE CASCADE,
    profile      TEXT NOT NULL DEFAULT 'local',   -- local | public | trunk
    created_at   TEXT NOT NULL,
    rotated_at   TEXT
);

CREATE TABLE devices (
    id                TEXT PRIMARY KEY,
    tenant_id         TEXT NOT NULL DEFAULT 'default',
    name              TEXT NOT NULL,
    vendor            TEXT NOT NULL DEFAULT 'generic',
    model             TEXT NOT NULL DEFAULT 'generic-sip',
    mac               TEXT UNIQUE,
    device_type       TEXT NOT NULL DEFAULT 'desk_phone',
    provisioning_mode TEXT NOT NULL DEFAULT 'manual',  -- manual | managed
    enrollment_state  TEXT NOT NULL DEFAULT 'pending', -- pending | enrolled | blocked
    site_id           TEXT REFERENCES sites(id),
    firmware          TEXT,
    ip                TEXT,
    status            TEXT NOT NULL DEFAULT 'unknown', -- online | degraded | offline | unknown
    last_seen_at      TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE TABLE device_lines (
    id           TEXT PRIMARY KEY,
    device_id    TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    line_no      INTEGER NOT NULL,
    extension_id TEXT REFERENCES extensions(id) ON DELETE SET NULL,
    credential_id TEXT REFERENCES sip_credentials(id),
    UNIQUE (device_id, line_no)
);

-- 8.2 Rufnummern
CREATE TABLE numbers (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL DEFAULT 'default',
    canonical   TEXT NOT NULL UNIQUE,   -- E.164 bevorzugt
    display     TEXT NOT NULL,
    source_repr TEXT NOT NULL,
    number_type TEXT NOT NULL,          -- external | internal | emergency | service
    country     TEXT,
    trunk_id    TEXT REFERENCES trunks(id) ON DELETE SET NULL,
    route_graph_id TEXT REFERENCES route_graphs(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL
);

-- 8.3 Provider
CREATE TABLE provider_profiles (
    id            TEXT PRIMARY KEY,
    key           TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    registrar     TEXT,
    proxy         TEXT,
    transport     TEXT NOT NULL DEFAULT 'udp',   -- udp | tcp | tls
    codecs        TEXT NOT NULL DEFAULT '["OPUS","PCMA","PCMU"]',
    number_format TEXT NOT NULL DEFAULT 'e164',
    capabilities  TEXT NOT NULL DEFAULT '{}',
    auth_capabilities TEXT NOT NULL DEFAULT '["register"]',
    builtin       INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL
);

CREATE TABLE trunks (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL UNIQUE,
    provider_profile_id TEXT NOT NULL REFERENCES provider_profiles(id),
    mode                TEXT NOT NULL DEFAULT 'register',  -- register | static_ip
    host                TEXT NOT NULL,
    port                INTEGER NOT NULL DEFAULT 5060,
    transport           TEXT NOT NULL DEFAULT 'udp',
    auth_username       TEXT,
    from_user           TEXT,
    from_domain         TEXT,
    secret_id           TEXT REFERENCES secrets(id),
    inbound_trust       TEXT NOT NULL DEFAULT 'strict',
    security_profile    TEXT NOT NULL DEFAULT 'strict',
    enabled             INTEGER NOT NULL DEFAULT 1,
    status              TEXT NOT NULL DEFAULT 'unknown',
    last_status_at      TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE TABLE ring_groups (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    strategy   TEXT NOT NULL DEFAULT 'simultaneous',
    members    TEXT NOT NULL DEFAULT '[]',
    timeout_s  INTEGER NOT NULL DEFAULT 25,
    created_at TEXT NOT NULL
);

CREATE TABLE queues (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    strategy   TEXT NOT NULL DEFAULT 'longest-idle-agent',
    members    TEXT NOT NULL DEFAULT '[]',
    recording  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE ivrs (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    greeting_media_id TEXT REFERENCES media_assets(id),
    options    TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);

CREATE TABLE voicemail_boxes (
    id           TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    pin_secret_id TEXT REFERENCES secrets(id),
    delegates    TEXT NOT NULL DEFAULT '[]',
    created_at   TEXT NOT NULL
);

-- --------------------------------------------------------------- Scheduling

CREATE TABLE schedules (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    timezone   TEXT NOT NULL DEFAULT 'Europe/Berlin',
    weekly     TEXT NOT NULL DEFAULT '[]',
    holidays   TEXT NOT NULL DEFAULT '[]',
    exceptions TEXT NOT NULL DEFAULT '[]',
    priority   INTEGER NOT NULL DEFAULT 100,
    created_at TEXT NOT NULL
);

-- -------------------------------------------------------------------- Media

CREATE TABLE media_assets (
    id          TEXT PRIMARY KEY,
    sha256      TEXT NOT NULL UNIQUE,
    mime        TEXT NOT NULL,
    duration_ms INTEGER,
    scope       TEXT NOT NULL DEFAULT 'system',
    purpose     TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

-- ------------------------------------------------------------------ Routing

-- 8.4 typisierter Graph, kein Raw-Dialplan
CREATE TABLE route_graphs (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    graph      TEXT NOT NULL DEFAULT '{"nodes":[],"edges":[]}',
    version    INTEGER NOT NULL DEFAULT 1,
    status     TEXT NOT NULL DEFAULT 'draft',   -- draft | published
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 8.8 Config Versioning
CREATE TABLE config_generations (
    id           TEXT PRIMARY KEY,
    number       INTEGER NOT NULL UNIQUE,
    status       TEXT NOT NULL,      -- compiled | active | superseded | failed | rolled_back
    manifest     TEXT NOT NULL DEFAULT '{}',
    hash         TEXT NOT NULL,
    created_by   TEXT,
    created_at   TEXT NOT NULL,
    activated_at TEXT
);

-- 8.9 Transactional Outbox
CREATE TABLE outbox (
    id            TEXT PRIMARY KEY,
    topic         TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    payload       TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    delivered_at  TEXT,
    attempts      INTEGER NOT NULL DEFAULT 0
);

-- 8.7 append-only Call Event Ledger
CREATE TABLE call_events (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    id         TEXT NOT NULL UNIQUE,
    ts         TEXT NOT NULL,
    call_id    TEXT NOT NULL,
    leg_id     TEXT,
    event_type TEXT NOT NULL,
    direction  TEXT,
    payload    TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_call_events_call ON call_events(call_id);

-- ----------------------------------------------------------------- Betrieb

CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 14.1 keine unbegrenzte Default-Retention
CREATE TABLE retention_policies (
    key        TEXT PRIMARY KEY,
    purpose    TEXT NOT NULL,
    days       INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE health_snapshots (
    id         TEXT PRIMARY KEY,
    ts         TEXT NOT NULL,
    state      TEXT NOT NULL,
    checks     TEXT NOT NULL
);

CREATE TABLE backups (
    id          TEXT PRIMARY KEY,
    created_at  TEXT NOT NULL,
    kind        TEXT NOT NULL,       -- hot | manual
    units       TEXT NOT NULL,
    path        TEXT NOT NULL,
    sha256      TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    encrypted   INTEGER NOT NULL DEFAULT 1
);
