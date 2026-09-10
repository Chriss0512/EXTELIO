//! IR -> FreeSWITCH-XML (Kapitel 9.1).
//!
//! Statisch: Module, Sofia-Profile, TLS, Core Settings, ESL, XML-Curl Bootstrap.
//! Dynamisch: Directory, Users, Extensions, Dialplan, Routing.
//!
//! Passwoerter erscheinen in den erzeugten Dateien nur als Referenz
//! `{{secret:<id>}}`. Der XML-Adapter loest sie zur Laufzeit ueber den Secret
//! Broker auf, damit kein Klartext auf /data landet (Kapitel 7.4).

use super::ir::*;
use super::xml::Element;

pub fn secret_ref(id: &str) -> String {
    format!("{{{{secret:{id}}}}}")
}

pub fn modules_conf(ir: &ConfigIr) -> String {
    let mut cfg = Element::new("configuration")
        .attr("name", "modules.conf")
        .attr("description", "EXTELIO module allowlist")
        .comment("Erzeugt aus dem Desired State. Manuelle Aenderungen werden ueberschrieben.");
    let mut modules = Element::new("modules");
    for m in &ir.modules {
        if module_allowed(m) {
            modules = modules.child(Element::new("load").attr("module", m.clone()));
        }
    }
    cfg = cfg.child(modules);
    document("configuration", cfg)
}

pub fn switch_conf(ir: &ConfigIr) -> String {
    let settings = Element::new("settings")
        .child(Element::param("colorize-console", "false"))
        .child(Element::param("max-sessions", "500"))
        .child(Element::param("sessions-per-second", "30"))
        .child(Element::param("loglevel", "warning"))
        .child(Element::param(
            "rtp-start-port",
            ir.profiles
                .first()
                .map(|p| p.rtp_start)
                .unwrap_or(16384)
                .to_string(),
        ))
        .child(Element::param(
            "rtp-end-port",
            ir.profiles
                .first()
                .map(|p| p.rtp_end)
                .unwrap_or(32768)
                .to_string(),
        ))
        .child(Element::param("core-db-name", "/data/state/freeswitch.db"))
        .child(Element::param("auto-create-schemas", "true"));

    let vars = Element::new("variables")
        .child(Element::param("domain", ir.domain.clone()))
        .child(Element::param("domain_name", ir.domain.clone()))
        .child(Element::param("global_codec_prefs", "OPUS,PCMA,PCMU"))
        .child(Element::param("outbound_codec_prefs", "OPUS,PCMA,PCMU"));

    document(
        "configuration",
        Element::new("configuration")
            .attr("name", "switch.conf")
            .attr("description", "EXTELIO core settings")
            .child(settings)
            .child(vars),
    )
}

pub fn event_socket_conf(ir: &ConfigIr, password_secret_id: &str) -> String {
    // Kapitel 5.6: nur Loopback, ACL loopback, nie das Default-Passwort.
    let settings = Element::new("settings")
        .child(Element::param("listen-ip", "127.0.0.1"))
        .child(Element::param("listen-port", ir.esl_port.to_string()))
        .child(Element::param("password", secret_ref(password_secret_id)))
        .child(Element::param("apply-inbound-acl", "loopback.auto"))
        .child(Element::param("nat-map", "false"));
    document(
        "configuration",
        Element::new("configuration")
            .attr("name", "event_socket.conf")
            .attr("description", "EXTELIO ESL")
            .child(settings),
    )
}

pub fn xml_curl_conf(bind_url: &str) -> String {
    let mut bindings = Element::new("bindings");
    for section in ["directory", "dialplan", "configuration"] {
        bindings = bindings.child(
            Element::new("binding")
                .attr("name", section)
                .child(
                    Element::new("param")
                        .attr("name", "gateway-url")
                        .attr("value", format!("{bind_url}/{section}"))
                        .attr("bindings", section),
                )
                .child(Element::param("method", "POST"))
                .child(Element::param("timeout", "5")),
        );
    }
    document(
        "configuration",
        Element::new("configuration")
            .attr("name", "xml_curl.conf")
            .attr("description", "EXTELIO XML adapter bootstrap")
            .child(bindings),
    )
}

pub fn acl_conf(ir: &ConfigIr) -> String {
    let mut lists = Element::new("network-lists");
    for acl in &ir.acls {
        let mut list = Element::new("list")
            .attr("name", acl.name.clone())
            .attr("default", acl.default_policy.clone());
        for (action, cidr) in &acl.nodes {
            list = list.child(
                Element::new("node")
                    .attr("type", action.clone())
                    .attr("cidr", cidr.clone()),
            );
        }
        lists = lists.child(list);
    }
    document(
        "configuration",
        Element::new("configuration")
            .attr("name", "acl.conf")
            .attr("description", "EXTELIO access control lists")
            .child(lists),
    )
}

pub fn sofia_conf(ir: &ConfigIr) -> String {
    let mut profiles = Element::new("profiles");
    for p in ir.profiles.iter().filter(|p| p.enabled) {
        profiles = profiles.child(sofia_profile(p, ir));
    }
    document(
        "configuration",
        Element::new("configuration")
            .attr("name", "sofia.conf")
            .attr("description", "EXTELIO sofia profiles")
            .child(Element::new("global_settings").child(Element::param("log-level", "0")))
            .child(profiles),
    )
}

fn sofia_profile(p: &SofiaProfileIr, ir: &ConfigIr) -> Element {
    let mut settings = Element::new("settings")
        .child(Element::param("context", p.context.clone()))
        .child(Element::param("sip-ip", p.bind_ip.clone()))
        .child(Element::param("rtp-ip", p.bind_ip.clone()))
        .child(Element::param("sip-port", p.sip_port.to_string()))
        .child(Element::param("dialplan", "XML"))
        .child(Element::param("auth-calls", bool_str(p.auth_calls)))
        .child(Element::param("auth-all-packets", "false"))
        .child(Element::param(
            "accept-blind-reg",
            bool_str(p.accept_blind_registration),
        ))
        .child(Element::param("accept-blind-auth", "false"))
        .child(Element::param(
            "inbound-codec-prefs",
            p.inbound_codec_prefs.join(","),
        ))
        .child(Element::param(
            "outbound-codec-prefs",
            p.outbound_codec_prefs.join(","),
        ))
        .child(Element::param("inbound-late-negotiation", "true"))
        .child(Element::param(
            "apply-inbound-acl",
            p.apply_inbound_acl.clone(),
        ))
        .child(Element::param(
            "apply-register-acl",
            p.apply_inbound_acl.clone(),
        ))
        .child(Element::param("challenge-realm", "auto_from"))
        .child(Element::param("disable-transcoding", "false"))
        .child(Element::param("manage-presence", "false"))
        .child(Element::param("rtp-timeout-sec", "300"))
        .child(Element::param("rtp-hold-timeout-sec", "1800"));

    if p.tls_enabled {
        settings = settings
            .child(Element::param("tls", "true"))
            .child(Element::param("tls-only", bool_str(p.srtp_required)))
            .child(Element::param("tls-sip-port", p.tls_port.to_string()))
            .child(Element::param("tls-version", "tlsv1.2,tlsv1.3"))
            .child(Element::param("tls-verify-date", "true"))
            .child(Element::param("tls-cert-dir", "/data/certs"));
    } else {
        settings = settings.child(Element::param("tls", "false"));
    }

    if p.srtp_required {
        settings = settings
            .child(Element::param(
                "rtp-secure-media",
                "mandatory:AEAD_AES_256_GCM,AES_CM_128_HMAC_SHA1_80",
            ))
            .child(Element::param("rtp-secure-media-inbound", "mandatory"));
    }

    let mut gateways = Element::new("gateways");
    if p.name == "trunk" {
        for g in &ir.gateways {
            gateways = gateways.child(gateway(g));
        }
    }

    Element::new("profile")
        .attr("name", p.name.clone())
        .child(settings)
        .child(gateways)
}

fn gateway(g: &GatewayIr) -> Element {
    let mut e = Element::new("gateway")
        .attr("name", g.name.clone())
        .child(Element::param("proxy", g.proxy.clone()))
        .child(Element::param("register", bool_str(g.register)))
        .child(Element::param("register-transport", g.transport.clone()))
        .child(Element::param("retry-seconds", g.retry_seconds.to_string()))
        .child(Element::param(
            "caller-id-in-from",
            bool_str(g.caller_id_in_from),
        ))
        .child(Element::param("ping", "30"));
    if let Some(u) = &g.username {
        e = e.child(Element::param("username", u.clone()));
    }
    if let Some(s) = &g.password_secret_id {
        e = e.child(Element::param("password", secret_ref(s)));
    }
    if let Some(f) = &g.from_user {
        e = e.child(Element::param("from-user", f.clone()));
    }
    if let Some(d) = &g.from_domain {
        e = e.child(Element::param("from-domain", d.clone()));
    }
    e
}

/// Directory-Dokument (dynamisch ueber mod_xml_curl ausgeliefert).
pub fn directory_document(ir: &ConfigIr) -> String {
    let mut domain = Element::new("domain")
        .attr("name", ir.domain.clone())
        .child(
            Element::new("params")
                .child(Element::param("dial-string",
                    "{^^:sip_invite_domain=${dialed_domain}:presence_id=${dialed_user}@${dialed_domain}}${sofia_contact(*/${dialed_user}@${dialed_domain})}")),
        );

    let mut users = Element::new("users");
    for u in &ir.directory {
        let mut params = Element::new("params");
        if let Some(s) = &u.password_secret_id {
            params = params.child(Element::param("password", secret_ref(s)));
        }
        params = params.child(Element::param("vm-enabled", bool_str(u.voicemail_enabled)));

        let variables = Element::new("variables")
            .child(Element::param("user_context", u.context.clone()))
            .child(Element::param(
                "effective_caller_id_name",
                u.effective_caller_id_name.clone(),
            ))
            .child(Element::param(
                "effective_caller_id_number",
                u.effective_caller_id_number.clone(),
            ))
            .child(Element::param("extelio_extension_id", u.id.clone()));

        users = users.child(
            Element::new("user")
                .attr("id", u.extension_number.clone())
                .child(params)
                .child(variables),
        );
    }
    domain = domain.child(users);

    document(
        "document",
        Element::new("document")
            .attr("type", "freeswitch/xml")
            .child(
                Element::new("section")
                    .attr("name", "directory")
                    .child(domain),
            ),
    )
}

/// Dialplan-Dokument (dynamisch ueber mod_xml_curl ausgeliefert).
pub fn dialplan_document(ir: &ConfigIr) -> String {
    let mut section = Element::new("section").attr("name", "dialplan");
    for ctx in &ir.contexts {
        let mut context = Element::new("context").attr("name", ctx.name.clone());
        for ext in &ctx.extensions {
            let mut e = Element::new("extension").attr("name", ext.name.clone());
            for c in &ext.conditions {
                let mut cond = Element::new("condition")
                    .attr("field", c.field.clone())
                    .attr("expression", c.expression.clone());
                for a in &c.actions {
                    cond = cond.child(action(a, false));
                }
                for a in &c.anti_actions {
                    cond = cond.child(action(a, true));
                }
                e = e.child(cond);
            }
            context = context.child(e);
        }
        section = section.child(context);
    }
    document(
        "document",
        Element::new("document")
            .attr("type", "freeswitch/xml")
            .child(section),
    )
}

fn action(a: &ActionIr, anti: bool) -> Element {
    let tag = if anti { "anti-action" } else { "action" };
    match a {
        ActionIr::Set { key, value } => Element::new(tag)
            .attr("application", "set")
            .attr("data", format!("{key}={value}")),
        ActionIr::Bridge { target } => Element::new(tag)
            .attr("application", "bridge")
            .attr("data", target.clone()),
        ActionIr::Answer => Element::new(tag).attr("application", "answer"),
        ActionIr::Playback { file } => Element::new(tag)
            .attr("application", "playback")
            .attr("data", file.clone()),
        ActionIr::Voicemail { extension } => Element::new(tag)
            .attr("application", "voicemail")
            .attr("data", format!("default ${{domain_name}} {extension}")),
        ActionIr::Transfer { target, context } => Element::new(tag)
            .attr("application", "transfer")
            .attr("data", format!("{target} XML {context}")),
        ActionIr::Hangup { cause } => Element::new(tag)
            .attr("application", "hangup")
            .attr("data", cause.clone()),
        ActionIr::RingGroup {
            targets,
            timeout_s,
            strategy,
        } => {
            let sep = if strategy == "sequential" { "|" } else { "," };
            let dial: Vec<String> = targets
                .iter()
                .map(|t| format!("user/{t}@${{domain_name}}"))
                .collect();
            Element::new(tag).attr("application", "bridge").attr(
                "data",
                format!("{{call_timeout={timeout_s}}}{}", dial.join(sep)),
            )
        }
        ActionIr::Queue { name } => Element::new(tag)
            .attr("application", "callcenter")
            .attr("data", name.clone()),
        ActionIr::Ivr { name } => Element::new(tag)
            .attr("application", "ivr")
            .attr("data", name.clone()),
        ActionIr::Log { message } => Element::new(tag)
            .attr("application", "log")
            .attr("data", format!("INFO {message}")),
    }
}

fn bool_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

fn document(_kind: &str, root: Element) -> String {
    root.render_document()
}

/// Haupt-`freeswitch.xml`, das die erzeugten Konfigurationen einbindet.
pub fn freeswitch_xml() -> String {
    document(
        "document",
        Element::new("document")
            .attr("type", "freeswitch/xml")
            .comment("EXTELIO generierte Konfiguration - nicht manuell bearbeiten")
            .child(
                Element::new("section")
                    .attr("name", "configuration")
                    .attr("description", "Configuration")
                    .child(
                        Element::new("X-PRE-PROCESS")
                            .attr("cmd", "include")
                            .attr("data", "autoload_configs/*.xml"),
                    ),
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConfigIr {
        ConfigIr {
            domain: "pbx.local".into(),
            profiles: vec![SofiaProfileIr {
                name: "local".into(),
                context: "extelio_local".into(),
                bind_ip: "0.0.0.0".into(),
                sip_port: 5060,
                tls_port: 5061,
                tls_enabled: true,
                srtp_required: false,
                auth_calls: true,
                accept_blind_registration: false,
                inbound_codec_prefs: vec!["OPUS".into(), "PCMA".into()],
                outbound_codec_prefs: vec!["OPUS".into(), "PCMA".into()],
                rtp_start: 16384,
                rtp_end: 32768,
                apply_inbound_acl: "extelio_local".into(),
                enabled: true,
            }],
            directory: vec![DirectoryUserIr {
                id: "ext1".into(),
                extension_number: "201".into(),
                display_name: "Empfang".into(),
                password_secret_id: Some("sec_abc".into()),
                voicemail_enabled: true,
                context: "extelio_local".into(),
                effective_caller_id_name: "Empfang".into(),
                effective_caller_id_number: "201".into(),
            }],
            gateways: vec![],
            contexts: vec![DialplanContextIr {
                name: "extelio_local".into(),
                extensions: vec![DialplanExtensionIr {
                    name: "local-201".into(),
                    conditions: vec![ConditionIr {
                        field: "destination_number".into(),
                        expression: "^201$".into(),
                        actions: vec![ActionIr::Bridge {
                            target: "user/201@${domain_name}".into(),
                        }],
                        anti_actions: vec![],
                    }],
                }],
            }],
            acls: vec![AclIr {
                name: "extelio_local".into(),
                default_policy: "deny".into(),
                nodes: vec![("allow".into(), "10.0.0.0/8".into())],
            }],
            esl_port: 8021,
            modules: MODULE_ALLOWLIST.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn sofia_profile_is_strict_by_default() {
        let out = sofia_conf(&sample());
        assert!(out.contains("<param name=\"auth-calls\" value=\"true\"/>"));
        assert!(
            out.contains("<param name=\"accept-blind-reg\" value=\"false\"/>"),
            "Kapitel 5.4: keine Guest Calls"
        );
        assert!(out.contains("<param name=\"sip-port\" value=\"5060\"/>"));
    }

    #[test]
    fn esl_never_uses_the_default_password() {
        let out = event_socket_conf(&sample(), "sec_esl");
        let forbidden = ["Clue", "Con"].concat();
	assert!(!out.contains(&forbidden), "Kapitel 5.6");
        assert!(out.contains("127.0.0.1"));
        assert!(out.contains("{{secret:sec_esl}}"));
    }

    #[test]
    fn directory_contains_only_secret_references() {
        let out = directory_document(&sample());
        assert!(out.contains("{{secret:sec_abc}}"));
        assert!(out.contains("<user id=\"201\">"));
    }

    #[test]
    fn modules_conf_respects_the_allowlist() {
        let mut ir = sample();
        ir.modules.push("mod_verto".into());
        let out = modules_conf(&ir);
        assert!(out.contains("mod_sofia"));
        assert!(!out.contains("mod_verto"), "Kapitel 22.4 Denylist");
    }

    #[test]
    fn dialplan_renders_conditions() {
        let out = dialplan_document(&sample());
        assert!(out.contains("<context name=\"extelio_local\">"));
        assert!(out.contains("expression=\"^201$\""));
        assert!(out.contains("application=\"bridge\""));
    }
}
