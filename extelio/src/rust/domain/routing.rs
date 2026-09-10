//! Routing-Domaene (Kapitel 8.4, 2.14).
//!
//! Routing ist ein typisierter Graph, kein Raw-Dialplan. Validiert werden
//! Referenzen, Dead Ends, Zyklen, Security Policies und Capability.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{Error, Result};

/// Knotentypen gemaess Kapitel 2.14 (Mindestumfang).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    IncomingNumber,
    Schedule,
    Extension,
    RingGroup,
    Queue,
    Ivr,
    Announcement,
    Voicemail,
    ExternalNumber,
    Condition,
    Fallback,
    End,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::IncomingNumber => "incoming_number",
            NodeKind::Schedule => "schedule",
            NodeKind::Extension => "extension",
            NodeKind::RingGroup => "ring_group",
            NodeKind::Queue => "queue",
            NodeKind::Ivr => "ivr",
            NodeKind::Announcement => "announcement",
            NodeKind::Voicemail => "voicemail",
            NodeKind::ExternalNumber => "external_number",
            NodeKind::Condition => "condition",
            NodeKind::Fallback => "fallback",
            NodeKind::End => "end",
        }
    }

    /// Startknoten eines Flows.
    pub fn is_entry(&self) -> bool {
        matches!(self, NodeKind::IncomingNumber)
    }

    /// Terminale Knoten duerfen keine ausgehende Kante brauchen.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            NodeKind::End | NodeKind::Voicemail | NodeKind::ExternalNumber | NodeKind::Announcement
        )
    }

    /// Knoten mit benannten Ausgaengen.
    pub fn ports(&self) -> &'static [&'static str] {
        match self {
            NodeKind::Schedule | NodeKind::Condition => &["yes", "no"],
            NodeKind::Extension | NodeKind::RingGroup | NodeKind::Queue => &["answered", "timeout"],
            NodeKind::Ivr => &["1", "2", "3", "timeout"],
            _ => &["next"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub label: String,
    /// Referenz auf ein Domaenenobjekt (Extension-ID, Queue-ID, Schedule-ID ...).
    #[serde(default)]
    pub ref_id: Option<String>,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    #[serde(default = "default_port")]
    pub port: String,
    pub to: String,
}

fn default_port() -> String {
    "next".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteGraph {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// Ergebnis einer Graphvalidierung.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl RouteGraph {
    pub fn entry(&self) -> Option<&Node> {
        self.nodes.iter().find(|n| n.kind.is_entry())
    }

    /// Vollstaendige Validierung: Referenzen, Ports, Dead Ends, Zyklen.
    pub fn validate(&self, known_refs: &HashSet<String>) -> ValidationReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        if ids.len() != self.nodes.len() {
            errors.push("Knoten-IDs sind nicht eindeutig".into());
        }

        match self.nodes.iter().filter(|n| n.kind.is_entry()).count() {
            0 => errors.push("Der Flow hat keinen Startknoten (Incoming Number)".into()),
            1 => {}
            n => errors.push(format!(
                "Der Flow hat {n} Startknoten, erlaubt ist genau einer"
            )),
        }

        for e in &self.edges {
            if !ids.contains(e.from.as_str()) {
                errors.push(format!(
                    "Kante verweist auf unbekannten Quellknoten '{}'",
                    e.from
                ));
            }
            if !ids.contains(e.to.as_str()) {
                errors.push(format!(
                    "Kante verweist auf unbekannten Zielknoten '{}'",
                    e.to
                ));
            }
            if let Some(n) = self.nodes.iter().find(|n| n.id == e.from) {
                if !n.kind.ports().contains(&e.port.as_str()) {
                    errors.push(format!(
                        "Knoten '{}' ({}) hat keinen Ausgang '{}'",
                        n.label,
                        n.kind.as_str(),
                        e.port
                    ));
                }
            }
        }

        for n in &self.nodes {
            let needs_ref = matches!(
                n.kind,
                NodeKind::Extension
                    | NodeKind::RingGroup
                    | NodeKind::Queue
                    | NodeKind::Ivr
                    | NodeKind::Schedule
                    | NodeKind::Voicemail
                    | NodeKind::IncomingNumber
            );
            if needs_ref {
                match &n.ref_id {
                    None => errors.push(format!(
                        "Knoten '{}' ({}) hat kein Ziel zugeordnet",
                        n.label,
                        n.kind.as_str()
                    )),
                    Some(r) if !known_refs.contains(r) => errors.push(format!(
                        "Knoten '{}' verweist auf ein nicht vorhandenes Objekt",
                        n.label
                    )),
                    _ => {}
                }
            }

            let outgoing = self.edges.iter().filter(|e| e.from == n.id).count();
            if !n.kind.is_terminal() && outgoing == 0 {
                errors.push(format!(
                    "Sackgasse: Knoten '{}' ({}) hat keinen Ausgang",
                    n.label,
                    n.kind.as_str()
                ));
            }
            if n.kind.is_terminal() && outgoing > 0 {
                warnings.push(format!(
                    "Knoten '{}' ist ein Endknoten, ausgehende Kanten werden ignoriert",
                    n.label
                ));
            }
        }

        // Erreichbarkeit ab Startknoten.
        if let Some(entry) = self.entry() {
            let reachable = self.reachable_from(&entry.id);
            for n in &self.nodes {
                if !reachable.contains(n.id.as_str()) {
                    warnings.push(format!(
                        "Knoten '{}' ist vom Start aus nicht erreichbar",
                        n.label
                    ));
                }
            }
        }

        if let Some(cycle) = self.find_cycle() {
            errors.push(format!("Zyklus im Flow erkannt: {cycle}"));
        }

        ValidationReport {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn adjacency(&self) -> HashMap<&str, Vec<&str>> {
        let mut m: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &self.edges {
            m.entry(e.from.as_str()).or_default().push(e.to.as_str());
        }
        m
    }

    fn reachable_from<'a>(&'a self, start: &'a str) -> HashSet<&'a str> {
        let adj = self.adjacency();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack = vec![start];
        while let Some(cur) = stack.pop() {
            if !seen.insert(cur) {
                continue;
            }
            if let Some(next) = adj.get(cur) {
                stack.extend(next.iter().copied());
            }
        }
        seen
    }

    fn find_cycle(&self) -> Option<String> {
        let adj = self.adjacency();
        let mut state: HashMap<&str, u8> = HashMap::new(); // 0 neu, 1 offen, 2 fertig
        let mut path: Vec<&str> = Vec::new();

        fn dfs<'a>(
            node: &'a str,
            adj: &HashMap<&'a str, Vec<&'a str>>,
            state: &mut HashMap<&'a str, u8>,
            path: &mut Vec<&'a str>,
        ) -> Option<String> {
            state.insert(node, 1);
            path.push(node);
            if let Some(next) = adj.get(node) {
                for n in next {
                    match state.get(n).copied().unwrap_or(0) {
                        0 => {
                            if let Some(c) = dfs(n, adj, state, path) {
                                return Some(c);
                            }
                        }
                        1 => {
                            let mut c: Vec<&str> = path.clone();
                            c.push(n);
                            return Some(c.join(" -> "));
                        }
                        _ => {}
                    }
                }
            }
            path.pop();
            state.insert(node, 2);
            None
        }

        for n in &self.nodes {
            if state.get(n.id.as_str()).copied().unwrap_or(0) == 0 {
                if let Some(c) = dfs(n.id.as_str(), &adj, &mut state, &mut path) {
                    return Some(c);
                }
            }
        }
        None
    }

    /// Simulation eines Anrufs durch den Graphen (Kapitel 2.14).
    /// `choices` beantwortet Verzweigungen, Default ist der erste Port.
    pub fn simulate(&self, choices: &HashMap<String, String>) -> Result<Vec<String>> {
        let entry = self
            .entry()
            .ok_or_else(|| Error::Validation("Kein Startknoten vorhanden".into()))?;
        let mut trace = Vec::new();
        let mut cur = entry;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 64 {
                return Err(Error::Validation(
                    "Simulation abgebrochen (zu viele Schritte)".into(),
                ));
            }
            trace.push(format!("{} [{}]", cur.label, cur.kind.as_str()));
            if cur.kind.is_terminal() {
                return Ok(trace);
            }
            let port = choices
                .get(&cur.id)
                .cloned()
                .unwrap_or_else(|| cur.kind.ports()[0].to_string());
            let edge = self
                .edges
                .iter()
                .find(|e| e.from == cur.id && e.port == port);
            let Some(edge) = edge else {
                trace.push(format!("Ende: kein Ausgang '{port}'"));
                return Ok(trace);
            };
            let Some(next) = self.nodes.iter().find(|n| n.id == edge.to) else {
                return Err(Error::Validation(format!("Zielknoten '{}' fehlt", edge.to)));
            };
            cur = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: NodeKind, r: Option<&str>) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            kind,
            ref_id: r.map(String::from),
            params: serde_json::Value::Null,
            x: 0.0,
            y: 0.0,
        }
    }

    fn refs() -> HashSet<String> {
        ["num1", "sched1", "ext1"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn accepts_a_valid_business_hours_flow() {
        let g = RouteGraph {
            nodes: vec![
                node("a", NodeKind::IncomingNumber, Some("num1")),
                node("b", NodeKind::Schedule, Some("sched1")),
                node("c", NodeKind::Extension, Some("ext1")),
                node("d", NodeKind::Voicemail, Some("ext1")),
            ],
            edges: vec![
                Edge {
                    from: "a".into(),
                    port: "next".into(),
                    to: "b".into(),
                },
                Edge {
                    from: "b".into(),
                    port: "yes".into(),
                    to: "c".into(),
                },
                Edge {
                    from: "b".into(),
                    port: "no".into(),
                    to: "d".into(),
                },
                Edge {
                    from: "c".into(),
                    port: "answered".into(),
                    to: "d".into(),
                },
                Edge {
                    from: "c".into(),
                    port: "timeout".into(),
                    to: "d".into(),
                },
            ],
        };
        let r = g.validate(&refs());
        assert!(r.valid, "{:?}", r.errors);

        let trace = g.simulate(&HashMap::new()).unwrap();
        assert!(trace.len() >= 3);
    }

    #[test]
    fn detects_dead_ends_cycles_and_bad_refs() {
        let dead = RouteGraph {
            nodes: vec![
                node("a", NodeKind::IncomingNumber, Some("num1")),
                node("b", NodeKind::Schedule, Some("sched1")),
            ],
            edges: vec![Edge {
                from: "a".into(),
                port: "next".into(),
                to: "b".into(),
            }],
        };
        assert!(!dead.validate(&refs()).valid);

        let cyclic = RouteGraph {
            nodes: vec![
                node("a", NodeKind::IncomingNumber, Some("num1")),
                node("b", NodeKind::Condition, None),
                node("c", NodeKind::Condition, None),
                node("e", NodeKind::End, None),
            ],
            edges: vec![
                Edge {
                    from: "a".into(),
                    port: "next".into(),
                    to: "b".into(),
                },
                Edge {
                    from: "b".into(),
                    port: "yes".into(),
                    to: "c".into(),
                },
                Edge {
                    from: "b".into(),
                    port: "no".into(),
                    to: "e".into(),
                },
                Edge {
                    from: "c".into(),
                    port: "yes".into(),
                    to: "b".into(),
                },
                Edge {
                    from: "c".into(),
                    port: "no".into(),
                    to: "e".into(),
                },
            ],
        };
        let r = cyclic.validate(&refs());
        assert!(!r.valid);
        assert!(r.errors.iter().any(|e| e.contains("Zyklus")));

        let badref = RouteGraph {
            nodes: vec![
                node("a", NodeKind::IncomingNumber, Some("does-not-exist")),
                node("e", NodeKind::End, None),
            ],
            edges: vec![Edge {
                from: "a".into(),
                port: "next".into(),
                to: "e".into(),
            }],
        };
        assert!(!badref.validate(&refs()).valid);
    }
}
