use crate::Node;
use std::collections::BTreeMap;

pub(crate) struct NameIndex {
    names: BTreeMap<(String, String), Vec<String>>,
}
impl NameIndex {
    pub fn new(nodes: &BTreeMap<String, Node>) -> Self {
        let mut names: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for node in nodes.values().filter(|node| {
            !node.contains_errors && matches!(node.kind.as_str(), "function" | "method")
        }) {
            if node.name.chars().count() >= 256 {
                continue;
            }
            // Receiver expressions never fall back to matching just the final method name.
            if node.kind == "function" {
                names
                    .entry((node.language.clone(), node.name.clone()))
                    .or_default()
                    .push(node.id.clone());
            }
            if node.qualified_name.contains("::") {
                names
                    .entry((node.language.clone(), node.qualified_name.clone()))
                    .or_default()
                    .push(node.id.clone());
            }
        }
        for ids in names.values_mut() {
            ids.sort();
            ids.dedup();
        }
        Self { names }
    }
    pub fn call_target(&self, language: &str, expression: &str) -> Option<String> {
        if !expression
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '.'))
        {
            return None;
        }
        let normalized = expression.replace('.', "::");
        let targets = self.names.get(&(language.to_owned(), normalized))?;
        if targets.len() == 1 {
            targets.first().cloned()
        } else {
            None
        }
    }
}

pub(crate) fn import_target(
    path: &str,
    language: &str,
    text: &str,
    nodes: &BTreeMap<String, Node>,
) -> Option<String> {
    let module = match language {
        "typescript" | "javascript" => {
            let quote = text.find(['\'', '"'])?;
            let delimiter = text.as_bytes()[quote] as char;
            let rest = &text[quote + 1..];
            let module = &rest[..rest.find(delimiter)?];
            if !module.starts_with('.') {
                return None;
            }
            let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
            format!("{parent}/{module}")
        }
        "python" => {
            let module = text.strip_prefix("from ")?.split_whitespace().next()?;
            if module.starts_with('.') {
                return None;
            }
            module.replace('.', "/")
        }
        "rust" => text
            .strip_prefix("use crate::")?
            .split([';', '{'])
            .next()?
            .trim_end_matches("::")
            .replace("::", "/"),
        _ => return None,
    };
    let mut parts = Vec::new();
    for part in module.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    let normalized = parts.join("/");
    let mut candidates = Vec::new();
    for suffix in [
        "",
        ".ts",
        ".tsx",
        ".js",
        ".jsx",
        ".py",
        ".rs",
        "/index.ts",
        "/index.js",
        "/__init__.py",
        "/mod.rs",
    ] {
        let id = format!("file:{normalized}{suffix}");
        if nodes.get(&id).is_some_and(|node| node.language == language) {
            candidates.push(id);
        }
        if language == "rust" {
            let id = format!("file:src/{normalized}{suffix}");
            if nodes.contains_key(&id) {
                candidates.push(id);
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    if candidates.len() == 1 {
        candidates.pop()
    } else {
        None
    }
}
