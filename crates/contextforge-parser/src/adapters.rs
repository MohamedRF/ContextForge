use crate::Error;
use std::path::Path;
use tree_sitter::{Language, Node};

pub(crate) fn language(path: &Path) -> Result<(&'static str, Language), Error> {
    Ok(match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => ("rust", tree_sitter_rust::LANGUAGE.into()),
        Some("py") => ("python", tree_sitter_python::LANGUAGE.into()),
        Some("ts") => (
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        ),
        Some("tsx") => ("typescript", tree_sitter_typescript::LANGUAGE_TSX.into()),
        Some("js" | "jsx" | "mjs" | "cjs") => {
            ("javascript", tree_sitter_javascript::LANGUAGE.into())
        }
        _ => return Err(Error::Unsupported),
    })
}

pub(crate) fn definition<'a>(node: Node<'a>, source: &'a str) -> Option<(&'a str, &'static str)> {
    let kind = match node.kind() {
        "function_item" | "function_signature_item" => {
            if node
                .parent()
                .and_then(|p| p.parent())
                .is_some_and(|p| matches!(p.kind(), "impl_item" | "trait_item"))
            {
                "method"
            } else {
                "function"
            }
        }
        "function_definition" => {
            // Decorated Python methods have a wrapper between definition and class block.
            let mut parent = node.parent();
            if parent.is_some_and(|p| p.kind() == "decorated_definition") {
                parent = parent.and_then(|p| p.parent());
            }
            if parent
                .and_then(|p| p.parent())
                .is_some_and(|p| p.kind() == "class_definition")
            {
                "method"
            } else {
                "function"
            }
        }
        "function_declaration" | "generator_function_declaration" => "function",
        "method_definition" | "method_signature" | "abstract_method_signature" => "method",
        "struct_item" => "struct",
        "enum_item" | "enum_declaration" => "enum",
        "trait_item" => "trait",
        "class_definition" | "class_declaration" | "abstract_class_declaration" => "class",
        "interface_declaration" => "interface",
        "type_item" | "type_alias_declaration" => "type",
        "mod_item" | "internal_module" => "module",
        "const_item" | "static_item" => "constant",
        "impl_item" => {
            let name = node.child_by_field_name("type")?;
            return Some((&source[name.byte_range()], "implementation"));
        }
        "variable_declarator" => {
            let value = node.child_by_field_name("value")?;
            if matches!(
                value.kind(),
                "arrow_function" | "function_expression" | "generator_function"
            ) {
                "function"
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let name = node.child_by_field_name("name")?;
    if name.is_missing() || name.start_byte() == name.end_byte() {
        return None;
    }
    // Destructuring does not define one named callable.
    if node.kind() == "variable_declarator" && name.kind() != "identifier" {
        return None;
    }
    Some((&source[name.byte_range()], kind))
}

pub(crate) fn occurrence(node: Node<'_>) -> Option<&'static str> {
    match node.kind() {
        "call_expression" | "call" | "new_expression" => Some("call"),
        "use_declaration"
        | "extern_crate_declaration"
        | "import_statement"
        | "import_from_statement"
        | "import_alias" => Some("import"),
        "export_statement" => Some("export"),
        _ => None,
    }
}

pub(crate) fn visibility(node: Node<'_>, language: &str, source: &str) -> &'static str {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "visibility_modifier" => {
                return if &source[child.byte_range()] == "pub" {
                    "public"
                } else {
                    "restricted"
                };
            }
            "accessibility_modifier" => {
                return match &source[child.byte_range()] {
                    "public" => "public",
                    "private" => "private",
                    "protected" => "protected",
                    _ => "unspecified",
                };
            }
            _ => {}
        }
    }
    if language == "rust" {
        "private"
    } else {
        "unspecified"
    }
}
