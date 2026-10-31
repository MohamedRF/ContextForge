#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_parser::parse_source;

#[test]
fn deterministic_generated_malformed_unicode_inputs_preserve_span_and_output_invariants() {
    let atoms = [
        "fn café() {",
        "}",
        "\n",
        "\r\n",
        "🌱",
        "\"",
        "/*",
        "*/",
        "def f():",
        "export function f()",
        "f();",
        "[",
        "# comment",
        "\\",
        "\t",
    ];
    let mut state = 0x1234_5678_u64;
    for path in ["case.rs", "case.py", "case.ts", "case.jsx"] {
        for case in 0..64 {
            let mut source = String::new();
            for _ in 0..case {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                source.push_str(atoms[(state >> 32) as usize % atoms.len()]);
            }
            let limit = case % 12 + 1;
            let outline = parse_source("generated", path, &source, limit).unwrap();
            assert!(outline.symbols.len() + outline.occurrences.len() <= limit);
            for span in outline
                .symbols
                .iter()
                .map(|s| &s.span)
                .chain(outline.occurrences.iter().map(|o| &o.span))
            {
                assert!(span.start_byte < span.end_byte);
                assert!(span.end_byte <= source.len());
                assert!(source.is_char_boundary(span.start_byte));
                assert!(source.is_char_boundary(span.end_byte));
                let line = |offset| {
                    source.as_bytes()[..offset]
                        .iter()
                        .filter(|b| **b == b'\n')
                        .count()
                        + 1
                };
                assert_eq!(span.start_line, line(span.start_byte));
                assert_eq!(span.end_line, line(span.end_byte - 1));
            }
            assert_eq!(
                serde_json::to_value(&outline).unwrap(),
                serde_json::to_value(parse_source("generated", path, &source, limit).unwrap())
                    .unwrap()
            );
        }
        assert!(parse_source("generated", path, "\0", 1).is_err());
    }
}
