//! Strukturelle XML-Erzeugung (Kapitel 9).
//!
//! XML wird nie per String-Konkatenation gebaut. Werte durchlaufen immer die
//! Escaping-Funktion dieses Moduls.

#[derive(Debug, Clone)]
pub struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

#[derive(Debug, Clone)]
enum Node {
    Element(Element),
    Text(String),
    Comment(String),
}

impl Element {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(mut self, key: &str, value: impl Into<String>) -> Self {
        self.attrs.push((key.to_string(), value.into()));
        self
    }

    pub fn attr_opt(self, key: &str, value: Option<impl Into<String>>) -> Self {
        match value {
            Some(v) => self.attr(key, v),
            None => self,
        }
    }

    pub fn child(mut self, e: Element) -> Self {
        self.children.push(Node::Element(e));
        self
    }

    pub fn children(mut self, it: impl IntoIterator<Item = Element>) -> Self {
        for e in it {
            self.children.push(Node::Element(e));
        }
        self
    }

    pub fn text(mut self, t: impl Into<String>) -> Self {
        self.children.push(Node::Text(t.into()));
        self
    }

    pub fn comment(mut self, t: impl Into<String>) -> Self {
        self.children.push(Node::Comment(t.into()));
        self
    }

    /// FreeSWITCH-Kurzform `<param name=".." value=".."/>`.
    pub fn param(name: &str, value: impl Into<String>) -> Element {
        Element::new("param")
            .attr("name", name)
            .attr("value", value)
    }

    pub fn render_document(&self) -> String {
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        self.render_into(&mut out, 0);
        out.push('\n');
        out
    }

    fn render_into(&self, out: &mut String, depth: usize) {
        let pad = "  ".repeat(depth);
        out.push_str(&pad);
        out.push('<');
        out.push_str(&escape_name(&self.name));
        for (k, v) in &self.attrs {
            out.push(' ');
            out.push_str(&escape_name(k));
            out.push_str("=\"");
            out.push_str(&escape_attr(v));
            out.push('"');
        }
        if self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');

        let only_text = self.children.len() == 1 && matches!(self.children[0], Node::Text(_));
        if only_text {
            if let Node::Text(t) = &self.children[0] {
                out.push_str(&escape_text(t));
            }
        } else {
            for c in &self.children {
                out.push('\n');
                match c {
                    Node::Element(e) => e.render_into(out, depth + 1),
                    Node::Text(t) => {
                        out.push_str(&"  ".repeat(depth + 1));
                        out.push_str(&escape_text(t));
                    }
                    Node::Comment(t) => {
                        out.push_str(&"  ".repeat(depth + 1));
                        out.push_str("<!-- ");
                        out.push_str(&escape_comment(t));
                        out.push_str(" -->");
                    }
                }
            }
            out.push('\n');
            out.push_str(&pad);
        }
        out.push_str("</");
        out.push_str(&escape_name(&self.name));
        out.push('>');
    }
}

/// Element- und Attributnamen duerfen nur aus einem engen Alphabet bestehen.
/// Alles andere ist ein Programmierfehler und wird ersetzt, nicht durchgereicht.
fn escape_name(n: &str) -> String {
    n.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn escape_attr(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

pub fn escape_text(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

fn escape_comment(v: &str) -> String {
    v.replace("--", "- -").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_attribute_injection() {
        let e = Element::new("user").attr("id", "20\"/><evil x=\"1");
        let out = e.render_document();
        assert!(
            !out.contains("<evil"),
            "Injektion muss escaped werden: {out}"
        );
        assert!(out.contains("&quot;"));
    }

    #[test]
    fn renders_nested_structure() {
        let doc = Element::new("document")
            .attr("type", "freeswitch/xml")
            .child(
                Element::new("section").attr("name", "directory").child(
                    Element::new("domain")
                        .attr("name", "extelio.local")
                        .child(Element::param("dial-string", "user/${dialed_user}")),
                ),
            );
        let out = doc.render_document();
        assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(out.contains("<section name=\"directory\">"));
        assert!(out.contains("<param name=\"dial-string\" value=\"user/${dialed_user}\"/>"));
    }

    #[test]
    fn sanitises_element_names() {
        let out = Element::new("bad name<x>").render_document();
        assert!(out.contains("<bad_name_x_/>"));
    }
}
