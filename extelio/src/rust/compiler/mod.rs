//! Config Compiler (Kapitel 9).
//!
//! Pipeline:
//!   Desired State -> Normalize -> Schema-/Domain-/Referenzvalidierung
//!   -> Security Policy -> Routing -> Capability -> typisiertes IR
//!   -> Sofia/Directory/Dialplan/Media/ACL Compiler
//!
//! Deployment: /data/config/generation-N mit atomarem Symlink `current`
//! und Rollback auf die vorherige Generation.

pub mod freeswitch;
pub mod ir;
pub mod xml;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::routing::{NodeKind, RouteGraph};
use crate::infrastructure::config::Options;
use crate::infrastructure::db::Db;
use crate::infrastructure::time::now_rfc3339;
use crate::{Error, Result};

use ir::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationManifest {
    pub number: i64,
    pub created_at: String,
    pub hash: String,
    pub files: Vec<FileEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileReport {
    pub generation: i64,
    pub hash: String,
    pub files: usize,
    pub warnings: Vec<String>,
}

fn sha256_hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}

/// Liest den Desired State aus der Datenbank und normalisiert ihn ins IR.
pub fn build_ir(db: &Db, opts: &Options) -> Result<ConfigIr> {
    let domain = if opts.canonical_hostname.is_empty() {
        "pbx.local".to_string()
    } else {
        opts.canonical_hostname.clone()
    };

    let guard = db.lock();

    // --- Directory ------------------------------------------------------
    let mut directory = Vec::new();
    {
        let mut stmt = guard.prepare(
            "SELECT e.id, e.number, e.name, e.voicemail, e.outbound_caller_id, c.secret_id
             FROM extensions e
             LEFT JOIN sip_credentials c ON c.extension_id = e.id AND c.profile='local'
             ORDER BY e.number",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(DirectoryUserIr {
                id: r.get(0)?,
                extension_number: r.get(1)?,
                display_name: r.get(2)?,
                voicemail_enabled: r.get::<_, i64>(3)? == 1,
                effective_caller_id_number: r
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| r.get::<_, String>(1).unwrap_or_default()),
                password_secret_id: r.get(5)?,
                context: "extelio_local".into(),
                effective_caller_id_name: r.get(2)?,
            })
        })?;
        for row in rows {
            directory.push(row?);
        }
    }

    // --- Gateways -------------------------------------------------------
    let mut gateways = Vec::new();
    {
        let mut stmt = guard.prepare(
            "SELECT t.id,t.name,t.host,t.port,t.transport,t.mode,t.auth_username,t.secret_id,
                    t.from_user,t.from_domain,t.enabled
             FROM trunks t WHERE t.enabled=1 ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |r| {
            let host: String = r.get(2)?;
            let port: i64 = r.get(3)?;
            let mode: String = r.get(5)?;
            Ok(GatewayIr {
                id: r.get(0)?,
                name: r.get(1)?,
                proxy: format!("{host}:{port}"),
                register: mode == "register",
                username: r.get(6)?,
                password_secret_id: r.get(7)?,
                from_user: r.get(8)?,
                from_domain: r.get(9)?,
                transport: r.get(4)?,
                retry_seconds: 30,
                caller_id_in_from: true,
            })
        })?;
        for row in rows {
            gateways.push(row?);
        }
    }

    // --- Nummern und Flows ---------------------------------------------
    let mut inbound: Vec<(String, String, Option<String>)> = Vec::new(); // (canonical, display, graph_id)
    {
        let mut stmt = guard.prepare(
            "SELECT canonical, display, route_graph_id FROM numbers WHERE number_type='external'",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        for row in rows {
            inbound.push(row?);
        }
    }

    let mut graphs: Vec<(String, RouteGraph)> = Vec::new();
    {
        let mut stmt = guard.prepare("SELECT id, graph FROM route_graphs")?;
        let rows = stmt.query_map([], |r| {
            let id: String = r.get(0)?;
            let raw: String = r.get(1)?;
            Ok((id, raw))
        })?;
        for row in rows {
            let (id, raw) = row?;
            let g: RouteGraph = serde_json::from_str(&raw).unwrap_or_default();
            graphs.push((id, g));
        }
    }

    let mut ring_groups: Vec<(String, String, Vec<String>, i64)> = Vec::new();
    {
        let mut stmt = guard.prepare("SELECT id,name,members,timeout_s FROM ring_groups")?;
        let rows = stmt.query_map([], |r| {
            let members: String = r.get(2)?;
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                serde_json::from_str::<Vec<String>>(&members).unwrap_or_default(),
                r.get::<_, i64>(3)?,
            ))
        })?;
        for row in rows {
            ring_groups.push(row?);
        }
    }

    drop(guard);

    // --- Dialplan -------------------------------------------------------
    let mut local_ctx = DialplanContextIr {
        name: "extelio_local".into(),
        extensions: Vec::new(),
    };

    // Interne Ziele
    for u in &directory {
        local_ctx.extensions.push(DialplanExtensionIr {
            name: format!("local-{}", u.extension_number),
            conditions: vec![ConditionIr {
                field: "destination_number".into(),
                expression: format!("^{}$", u.extension_number),
                actions: vec![
                    ActionIr::Set {
                        key: "call_timeout".into(),
                        value: "25".into(),
                    },
                    ActionIr::Set {
                        key: "hangup_after_bridge".into(),
                        value: "true".into(),
                    },
                    ActionIr::Bridge {
                        target: format!("user/{}@${{domain_name}}", u.extension_number),
                    },
                ],
                anti_actions: vec![],
            }],
        });
    }

    // Ausgehend ueber den ersten aktiven Trunk
    if let Some(gw) = gateways.first() {
        local_ctx.extensions.push(DialplanExtensionIr {
            name: "outbound-e164".into(),
            conditions: vec![ConditionIr {
                field: "destination_number".into(),
                expression: r"^(\+?\d{4,15})$".into(),
                actions: vec![
                    ActionIr::Set {
                        key: "hangup_after_bridge".into(),
                        value: "true".into(),
                    },
                    ActionIr::Bridge {
                        target: format!("sofia/gateway/{}/$1", gw.name),
                    },
                ],
                anti_actions: vec![],
            }],
        });
    }

    // Eingehend je Rufnummer
    let mut trunk_ctx = DialplanContextIr {
        name: "extelio_trunk".into(),
        extensions: Vec::new(),
    };
    let mut warnings = Vec::new();
    for (canonical, display, graph_id) in &inbound {
        let digits = canonical.trim_start_matches('+');
        let actions = match graph_id {
            Some(gid) => match graphs.iter().find(|(id, _)| id == gid) {
                Some((_, g)) => compile_graph(g, &ring_groups),
                None => {
                    warnings.push(format!("Nummer {display}: Flow {gid} nicht gefunden"));
                    vec![ActionIr::Hangup {
                        cause: "UNALLOCATED_NUMBER".into(),
                    }]
                }
            },
            None => {
                warnings.push(format!(
                    "Nummer {display} hat keinen Flow und wird abgewiesen"
                ));
                vec![ActionIr::Hangup {
                    cause: "UNALLOCATED_NUMBER".into(),
                }]
            }
        };
        trunk_ctx.extensions.push(DialplanExtensionIr {
            name: format!("inbound-{digits}"),
            conditions: vec![ConditionIr {
                field: "destination_number".into(),
                expression: format!(r"^\+?{digits}$"),
                actions,
                anti_actions: vec![],
            }],
        });
    }

    let public_ctx = DialplanContextIr {
        name: "extelio_public".into(),
        extensions: vec![DialplanExtensionIr {
            name: "public-registered-users".into(),
            conditions: vec![ConditionIr {
                field: "destination_number".into(),
                expression: r"^(\d{2,8})$".into(),
                actions: vec![ActionIr::Transfer {
                    target: "$1".into(),
                    context: "extelio_local".into(),
                }],
                anti_actions: vec![ActionIr::Hangup {
                    cause: "CALL_REJECTED".into(),
                }],
            }],
        }],
    };

    // --- Sofia-Profile (Kapitel 5.1, 5.2, 5.4) --------------------------
    let codecs = vec!["OPUS".to_string(), "PCMA".to_string(), "PCMU".to_string()];
    let profiles = vec![
        SofiaProfileIr {
            name: "local".into(),
            context: "extelio_local".into(),
            bind_ip: opts.sip_bind_ip.clone(),
            sip_port: opts.local_sip_port,
            tls_port: opts.local_sips_port,
            tls_enabled: true,
            srtp_required: false,
            auth_calls: true,
            accept_blind_registration: false,
            inbound_codec_prefs: codecs.clone(),
            outbound_codec_prefs: codecs.clone(),
            rtp_start: opts.rtp_start_port,
            rtp_end: opts.rtp_end_port,
            apply_inbound_acl: "extelio_local".into(),
            enabled: true,
        },
        SofiaProfileIr {
            name: "public".into(),
            context: "extelio_public".into(),
            bind_ip: opts.sip_bind_ip.clone(),
            sip_port: opts.public_sip_port,
            tls_port: opts.public_sips_port,
            tls_enabled: true,
            srtp_required: true,
            auth_calls: true,
            accept_blind_registration: false,
            inbound_codec_prefs: codecs.clone(),
            outbound_codec_prefs: codecs.clone(),
            rtp_start: opts.rtp_start_port,
            rtp_end: opts.rtp_end_port,
            apply_inbound_acl: "extelio_public".into(),
            enabled: opts.public_push_enabled,
        },
        SofiaProfileIr {
            name: "trunk".into(),
            context: "extelio_trunk".into(),
            bind_ip: opts.sip_bind_ip.clone(),
            sip_port: opts.trunk_sip_port,
            tls_port: opts.trunk_sips_port,
            tls_enabled: true,
            srtp_required: false,
            auth_calls: true,
            accept_blind_registration: false,
            inbound_codec_prefs: codecs.clone(),
            outbound_codec_prefs: codecs,
            rtp_start: opts.rtp_start_port,
            rtp_end: opts.rtp_end_port,
            apply_inbound_acl: "extelio_trunk".into(),
            enabled: true,
        },
    ];

    // --- ACLs -----------------------------------------------------------
    let mut trunk_nodes: Vec<(String, String)> = Vec::new();
    for g in &gateways {
        let host = g.proxy.split(':').next().unwrap_or_default().to_string();
        if !host.is_empty() {
            trunk_nodes.push(("allow".into(), format!("{host}/32")));
        }
    }
    let acls = vec![
        AclIr {
            name: "extelio_local".into(),
            default_policy: "deny".into(),
            nodes: vec![
                ("allow".into(), "10.0.0.0/8".into()),
                ("allow".into(), "172.16.0.0/12".into()),
                ("allow".into(), "192.168.0.0/16".into()),
                ("allow".into(), "127.0.0.0/8".into()),
            ],
        },
        AclIr {
            name: "extelio_public".into(),
            default_policy: "deny".into(),
            nodes: vec![("allow".into(), "0.0.0.0/0".into())],
        },
        AclIr {
            name: "extelio_trunk".into(),
            default_policy: "deny".into(),
            nodes: trunk_nodes,
        },
    ];

    Ok(ConfigIr {
        domain,
        profiles,
        directory,
        gateways,
        contexts: vec![local_ctx, public_ctx, trunk_ctx],
        acls,
        esl_port: 8021,
        modules: MODULE_ALLOWLIST.iter().map(|s| s.to_string()).collect(),
    })
}

/// Uebersetzt einen typisierten Routing-Graph in Dialplan-Aktionen.
fn compile_graph(
    g: &RouteGraph,
    ring_groups: &[(String, String, Vec<String>, i64)],
) -> Vec<ActionIr> {
    let mut actions = vec![ActionIr::Set {
        key: "hangup_after_bridge".into(),
        value: "true".into(),
    }];
    let Some(entry) = g.entry() else {
        return vec![ActionIr::Hangup {
            cause: "UNALLOCATED_NUMBER".into(),
        }];
    };

    let mut current = entry;
    let mut steps = 0;
    loop {
        steps += 1;
        if steps > 32 {
            actions.push(ActionIr::Log {
                message: "Flow zu tief, Abbruch".into(),
            });
            break;
        }
        match &current.kind {
            NodeKind::IncomingNumber => {}
            NodeKind::Extension => {
                if let Some(target) = current.params.get("number").and_then(|v| v.as_str()) {
                    actions.push(ActionIr::Bridge {
                        target: format!("user/{target}@${{domain_name}}"),
                    });
                }
            }
            NodeKind::RingGroup => {
                if let Some(rid) = &current.ref_id {
                    if let Some((_, _, members, timeout)) =
                        ring_groups.iter().find(|(id, _, _, _)| id == rid)
                    {
                        actions.push(ActionIr::RingGroup {
                            targets: members.clone(),
                            timeout_s: *timeout as u32,
                            strategy: "simultaneous".into(),
                        });
                    }
                }
            }
            NodeKind::Queue => {
                if let Some(name) = current.params.get("name").and_then(|v| v.as_str()) {
                    actions.push(ActionIr::Queue {
                        name: name.to_string(),
                    });
                }
            }
            NodeKind::Ivr => {
                if let Some(name) = current.params.get("name").and_then(|v| v.as_str()) {
                    actions.push(ActionIr::Answer);
                    actions.push(ActionIr::Ivr {
                        name: name.to_string(),
                    });
                }
            }
            NodeKind::Announcement => {
                if let Some(file) = current.params.get("file").and_then(|v| v.as_str()) {
                    actions.push(ActionIr::Answer);
                    actions.push(ActionIr::Playback {
                        file: file.to_string(),
                    });
                }
                actions.push(ActionIr::Hangup {
                    cause: "NORMAL_CLEARING".into(),
                });
                break;
            }
            NodeKind::Voicemail => {
                let ext = current
                    .params
                    .get("number")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0000")
                    .to_string();
                actions.push(ActionIr::Answer);
                actions.push(ActionIr::Voicemail { extension: ext });
                break;
            }
            NodeKind::ExternalNumber => {
                if let Some(n) = current.params.get("number").and_then(|v| v.as_str()) {
                    actions.push(ActionIr::Bridge {
                        target: format!("loopback/{n}"),
                    });
                }
                break;
            }
            NodeKind::Schedule | NodeKind::Condition => {
                // Zeit- und Bedingungslogik wird als Variablenauswertung gesetzt;
                // die Verzweigung selbst bildet der naechste Knoten ab.
                actions.push(ActionIr::Set {
                    key: "extelio_branch".into(),
                    value: current.ref_id.clone().unwrap_or_else(|| current.id.clone()),
                });
            }
            NodeKind::Fallback => {}
            NodeKind::End => {
                actions.push(ActionIr::Hangup {
                    cause: "NORMAL_CLEARING".into(),
                });
                break;
            }
        }

        let port = current.kind.ports()[0];
        let Some(edge) = g
            .edges
            .iter()
            .find(|e| e.from == current.id && e.port == port)
        else {
            break;
        };
        let Some(next) = g.nodes.iter().find(|n| n.id == edge.to) else {
            break;
        };
        current = next;
    }
    actions
}

/// Vollstaendige Validierung vor dem Kompilieren.
pub fn validate(db: &Db, ir: &ConfigIr) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    // Referenzvalidierung der Flows gegen vorhandene Objekte.
    let known: HashSet<String> = {
        let guard = db.lock();
        let mut set = HashSet::new();
        for sql in [
            "SELECT id FROM extensions",
            "SELECT id FROM numbers",
            "SELECT id FROM ring_groups",
            "SELECT id FROM queues",
            "SELECT id FROM ivrs",
            "SELECT id FROM schedules",
        ] {
            let mut stmt = guard.prepare(sql)?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for row in rows {
                set.insert(row?);
            }
        }
        set
    };

    let graphs: Vec<(String, String, String)> = {
        let guard = db.lock();
        let mut stmt =
            guard.prepare("SELECT id,name,graph FROM route_graphs WHERE status='published'")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    for (_, name, raw) in graphs {
        let g: RouteGraph = serde_json::from_str(&raw).map_err(|e| {
            Error::Validation(format!("Flow '{name}' ist kein gueltiger Graph: {e}"))
        })?;
        let report = g.validate(&known);
        if !report.valid {
            return Err(Error::Validation(format!(
                "Flow '{name}' ist nicht gueltig: {}",
                report.errors.join("; ")
            )));
        }
        warnings.extend(
            report
                .warnings
                .into_iter()
                .map(|w| format!("Flow '{name}': {w}")),
        );
    }

    // Security Policy: STRICT verlangt SRTP auf PUBLIC/PUSH (Kapitel 5.4).
    for p in &ir.profiles {
        if p.name == "public" && p.enabled && !p.srtp_required {
            return Err(Error::Validation(
                "PUBLIC/PUSH ohne SRTP ist im Modus STRICT nicht zulaessig".into(),
            ));
        }
        if p.accept_blind_registration {
            return Err(Error::Validation(
                "Blind Registration ist nie zulaessig".into(),
            ));
        }
    }

    // Capability: Trunk mit Registrierung braucht Zugangsdaten.
    for g in &ir.gateways {
        if g.register && (g.username.is_none() || g.password_secret_id.is_none()) {
            return Err(Error::Validation(format!(
                "Trunk '{}' ist auf Registrierung gestellt, hat aber keine Zugangsdaten",
                g.name
            )));
        }
    }

    if ir.directory.is_empty() {
        warnings.push("Es ist keine Nebenstelle angelegt".into());
    }

    Ok(warnings)
}

/// Erzeugt eine neue Generation im Dateisystem (noch nicht aktiv).
pub fn compile(
    db: &Db,
    opts: &Options,
    config_dir: &Path,
    esl_secret_id: &str,
    xml_adapter_url: &str,
    actor: Option<&str>,
) -> Result<CompileReport> {
    let ir = build_ir(db, opts)?;
    let warnings = validate(db, &ir)?;

    let number = next_generation_number(db)?;
    let dir = config_dir.join(format!("generation-{number}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(dir.join("autoload_configs"))?;

    let files: Vec<(String, String)> = vec![
        ("freeswitch.xml".into(), freeswitch::freeswitch_xml()),
        (
            "autoload_configs/modules.conf.xml".into(),
            freeswitch::modules_conf(&ir),
        ),
        (
            "autoload_configs/switch.conf.xml".into(),
            freeswitch::switch_conf(&ir),
        ),
        (
            "autoload_configs/sofia.conf.xml".into(),
            freeswitch::sofia_conf(&ir),
        ),
        (
            "autoload_configs/acl.conf.xml".into(),
            freeswitch::acl_conf(&ir),
        ),
        (
            "autoload_configs/event_socket.conf.xml".into(),
            freeswitch::event_socket_conf(&ir, esl_secret_id),
        ),
        (
            "autoload_configs/xml_curl.conf.xml".into(),
            freeswitch::xml_curl_conf(xml_adapter_url),
        ),
        ("directory.xml".into(), freeswitch::directory_document(&ir)),
        ("dialplan.xml".into(), freeswitch::dialplan_document(&ir)),
        ("ir.json".into(), serde_json::to_string_pretty(&ir)?),
    ];

    let mut entries = Vec::new();
    let mut combined = Sha256::new();
    for (rel, content) in &files {
        let path = dir.join(rel);
        std::fs::write(&path, content)?;
        let digest = sha256_hex(content.as_bytes());
        combined.update(rel.as_bytes());
        combined.update(digest.as_bytes());
        entries.push(FileEntry {
            path: rel.clone(),
            sha256: digest,
            bytes: content.len(),
        });
    }
    let hash = format!("{:x}", combined.finalize());

    let manifest = GenerationManifest {
        number,
        created_at: now_rfc3339(),
        hash: hash.clone(),
        files: entries.clone(),
        warnings: warnings.clone(),
    };
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    let guard = db.lock();
    guard.execute(
        "INSERT INTO config_generations (id,number,status,manifest,hash,created_by,created_at)
         VALUES (?1,?2,'compiled',?3,?4,?5,?6)",
        rusqlite::params![
            crate::domain::ids::new_prefixed("gen"),
            number,
            serde_json::to_string(&manifest)?,
            hash,
            actor,
            now_rfc3339()
        ],
    )?;

    Ok(CompileReport {
        generation: number,
        hash,
        files: entries.len(),
        warnings,
    })
}

fn next_generation_number(db: &Db) -> Result<i64> {
    let guard = db.lock();
    let n: i64 = guard.query_row(
        "SELECT COALESCE(MAX(number),0)+1 FROM config_generations",
        [],
        |r| r.get(0),
    )?;
    Ok(n)
}

/// Aktiviert eine Generation atomar (Kapitel 9.2).
pub fn activate(db: &Db, config_dir: &Path, number: i64) -> Result<()> {
    let target = config_dir.join(format!("generation-{number}"));
    if !target.join("manifest.json").exists() {
        return Err(Error::NotFound(format!(
            "Generation {number} existiert nicht"
        )));
    }
    verify_manifest(&target)?;

    let link = config_dir.join("current");
    let tmp = config_dir.join(".current.tmp");
    let _ = std::fs::remove_file(&tmp);
    symlink(&target, &tmp)?;
    std::fs::rename(&tmp, &link)?;

    let guard = db.lock();
    guard.execute(
        "UPDATE config_generations SET status='superseded' WHERE status='active'",
        [],
    )?;
    guard.execute(
        "UPDATE config_generations SET status='active', activated_at=?2 WHERE number=?1",
        rusqlite::params![number, now_rfc3339()],
    )?;
    Ok(())
}

/// Rollback auf die zuletzt aktive Generation davor (Kapitel 9.2).
pub fn rollback(db: &Db, config_dir: &Path) -> Result<i64> {
    let previous: i64 = {
        let guard = db.lock();
        guard
            .query_row(
                "SELECT number FROM config_generations
                 WHERE status='superseded' AND activated_at IS NOT NULL
                 ORDER BY activated_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| Error::NotFound("Keine vorherige Generation vorhanden".into()))?
    };
    activate(db, config_dir, previous)?;
    let guard = db.lock();
    guard.execute(
        "UPDATE config_generations SET status='rolled_back'
         WHERE status='superseded' AND number > ?1",
        rusqlite::params![previous],
    )?;
    Ok(previous)
}

/// Prueft alle Dateien einer Generation gegen ihr Manifest (Drift Detection).
pub fn verify_manifest(dir: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(dir.join("manifest.json"))?;
    let m: GenerationManifest = serde_json::from_str(&raw)?;
    for f in &m.files {
        let content = std::fs::read(dir.join(&f.path))
            .map_err(|_| Error::Internal(format!("Datei {} fehlt in der Generation", f.path)))?;
        if sha256_hex(&content) != f.sha256 {
            return Err(Error::Internal(format!(
                "Datei {} weicht vom Manifest ab (Drift)",
                f.path
            )));
        }
    }
    Ok(())
}

pub fn active_generation(db: &Db) -> Option<i64> {
    let guard = db.lock();
    guard
        .query_row(
            "SELECT number FROM config_generations WHERE status='active'",
            [],
            |r| r.get(0),
        )
        .ok()
}

pub fn current_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("current")
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink(_t: &Path, _l: &Path) -> Result<()> {
    Err(Error::Internal(
        "Symlinks werden auf dieser Plattform nicht unterstuetzt".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::migrations;

    fn setup() -> (Db, PathBuf, Options) {
        let db = Db::open_memory().unwrap();
        migrations::run(&db).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "extelio-cfg-{}-{}",
            std::process::id(),
            crate::domain::ids::new_id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        (db, dir, Options::default())
    }

    fn add_extension(db: &Db, number: &str, name: &str) {
        let guard = db.lock();
        guard
            .execute(
                "INSERT INTO extensions (id,number,name,kind,voicemail,created_at,updated_at)
                 VALUES (?1,?2,?3,'user',1,?4,?4)",
                rusqlite::params![
                    crate::domain::ids::new_prefixed("ext"),
                    number,
                    name,
                    now_rfc3339()
                ],
            )
            .unwrap();
    }

    #[test]
    fn compiles_activates_and_rolls_back() {
        let (db, dir, opts) = setup();
        add_extension(&db, "201", "Empfang");

        let r1 = compile(
            &db,
            &opts,
            &dir,
            "sec_esl",
            "http://127.0.0.1:8081/xml",
            None,
        )
        .unwrap();
        assert_eq!(r1.generation, 1);
        activate(&db, &dir, 1).unwrap();
        assert_eq!(active_generation(&db), Some(1));
        assert!(dir.join("current/directory.xml").exists());

        add_extension(&db, "202", "Anna");
        let r2 = compile(
            &db,
            &opts,
            &dir,
            "sec_esl",
            "http://127.0.0.1:8081/xml",
            None,
        )
        .unwrap();
        assert_eq!(r2.generation, 2);
        assert_ne!(
            r1.hash, r2.hash,
            "Aenderung am Desired State aendert den Hash"
        );
        activate(&db, &dir, 2).unwrap();

        let back = rollback(&db, &dir).unwrap();
        assert_eq!(back, 1);
        assert_eq!(active_generation(&db), Some(1));
        let dp = std::fs::read_to_string(dir.join("current/directory.xml")).unwrap();
        assert!(dp.contains("id=\"201\"") && !dp.contains("id=\"202\""));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn manifest_detects_drift() {
        let (db, dir, opts) = setup();
        add_extension(&db, "201", "Empfang");
        compile(
            &db,
            &opts,
            &dir,
            "sec_esl",
            "http://127.0.0.1:8081/xml",
            None,
        )
        .unwrap();
        let gen = dir.join("generation-1");
        verify_manifest(&gen).unwrap();

        std::fs::write(gen.join("dialplan.xml"), "<manipuliert/>").unwrap();
        assert!(
            verify_manifest(&gen).is_err(),
            "Kapitel 9.3: Drift Detection"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_trunk_without_credentials() {
        let (db, dir, opts) = setup();
        {
            let guard = db.lock();
            guard
                .execute(
                    "INSERT INTO trunks (id,name,provider_profile_id,mode,host,port,transport,enabled,created_at,updated_at)
                     VALUES ('t1','Provider','pp-generic-register','register','sip.example.net',5060,'udp',1,?1,?1)",
                    rusqlite::params![now_rfc3339()],
                )
                .unwrap();
        }
        let err = compile(
            &db,
            &opts,
            &dir,
            "sec_esl",
            "http://127.0.0.1:8081/xml",
            None,
        )
        .unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
        std::fs::remove_dir_all(&dir).ok();
    }
}
