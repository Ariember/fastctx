//! Modified for CodeyX: validate the native file-tool definitions.

use fastctx::server::FastCtxServer;
use serde_json::Value;

#[test]
fn file_tools_publish_explicit_three_hint_annotations() {
    let tools = FastCtxServer::new().tool_definitions();
    let mut names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "create_file",
            "delete",
            "glob",
            "grep",
            "inspect_local_file",
            "move",
            "replace"
        ]
    );
    for tool in &tools {
        let annotations = tool.annotations.as_ref().expect("annotations");
        assert_eq!(
            annotations.read_only_hint,
            Some(matches!(
                tool.name.as_ref(),
                "glob" | "grep" | "inspect_local_file"
            )),
            "{}",
            tool.name
        );
        assert_eq!(
            annotations.destructive_hint,
            Some(matches!(tool.name.as_ref(), "delete" | "move")),
            "{}",
            tool.name
        );
        assert_eq!(annotations.open_world_hint, Some(false), "{}", tool.name);
    }
}

/// Constructs that make a provider reject the whole tool declaration, each paired with
/// the failure it causes so a reintroduction is diagnosed rather than merely flagged.
///
/// These are not style preferences. A tool declaration is validated before the request
/// is submitted, so one rejected keyword returns 400 for the entire turn and takes every
/// FastCtx tool down with it — the symptom is "FastCtx does not work on this provider",
/// not "one parameter behaves oddly".
const REJECTED_SCHEMA_KEYWORDS: [(&str, &str); 9] = [
    (
        "$ref",
        "the Gemini API Schema type has no $ref field, and Codex rewrites every local \
         $ref to {} once a tool schema passes its compaction budget, silently erasing \
         an enum parameter's accepted values",
    ),
    (
        "$defs",
        "a definition table is unreachable once $ref is gone, and providers that do not \
         know the key reject it",
    ),
    (
        "oneOf",
        "absent from the Gemini, OpenAI and Anthropic keyword sets; a fieldless enum \
         belongs in type: \"string\" plus enum",
    ),
    (
        "anyOf",
        "Gemini rejects an anyOf node that carries any sibling key, and every parameter \
         here carries a description; express the union at the Rust type instead",
    ),
    (
        "allOf",
        "OpenAI lists allOf as unsupported and the Gemini Schema type has no such field",
    ),
    (
        "const",
        "no provider subset accepts it; a string const belongs in enum",
    ),
    (
        "$schema",
        "metadata no consumer reads, and every known host transform strips it",
    ),
    (
        "additionalProperties",
        "absent from the Gemini API Schema type, where an unrecognized key is an \
         Unknown name 400; serde(deny_unknown_fields) still enforces this at call time",
    ),
    (
        "format",
        "the derived numeric widths (uint, uint64, int64) are outside every provider's \
         accepted format set, and minimum/maximum already carry the bound",
    ),
];

/// Keywords a published input schema may use.
///
/// Deliberately an allow-list. schemars gains keywords over time and providers reject
/// what they do not recognize, so a newly emitted keyword must fail here and be argued
/// for rather than ship unnoticed. Every entry appears in the Gemini API Schema type,
/// OpenAI's supported-keyword set, and Anthropic's.
const PORTABLE_SCHEMA_KEYWORDS: [&str; 11] = [
    "type",
    "description",
    "properties",
    "required",
    "items",
    "enum",
    "default",
    "minimum",
    "maximum",
    "minItems",
    "maxItems",
];

/// Scalar type names a node may carry. "null" is absent on purpose: optionality is
/// carried by `required`, and Gemini's schema type is a single scalar with no way to
/// spell a nullable union.
const PORTABLE_SCHEMA_TYPES: [&str; 6] =
    ["object", "array", "string", "integer", "number", "boolean"];

/// Freezes the portable subset across every published tool rather than a named list of
/// parameters, so a new tool, a new parameter, or a new schemars version cannot
/// reintroduce a construct that costs us a whole provider. `src/tool_schema.rs` states
/// the subset and the per-keyword reasoning; read it before widening either list above.
#[test]
fn published_tool_schemas_stay_inside_the_portable_subset() {
    for tool in FastCtxServer::new().tool_definitions() {
        let schema = Value::Object((*tool.input_schema).clone());
        assert_eq!(
            schema["type"], "object",
            "{}: a tool's parameters must be an object",
            tool.name
        );
        assert_portable_schema_node(&schema, &tool.name);
    }
}

fn assert_portable_schema_node(node: &Value, path: &str) {
    let map = node
        .as_object()
        .unwrap_or_else(|| panic!("{path}: every published schema node must be an object: {node}"));
    for (keyword, reason) in REJECTED_SCHEMA_KEYWORDS {
        assert!(
            !map.contains_key(keyword),
            "{path}: `{keyword}` reappeared in a published tool schema — {reason}"
        );
    }
    for key in map.keys() {
        assert!(
            PORTABLE_SCHEMA_KEYWORDS.contains(&key.as_str()),
            "{path}: `{key}` is outside the portable keyword set; \
             adding it means answering why every provider accepts it"
        );
    }
    let declared = map.get("type").and_then(Value::as_str).unwrap_or_else(|| {
        panic!(
            "{path}: needs one scalar `type`; a missing or list-valued type is exactly \
             what strict providers reject: {node}"
        )
    });
    assert!(
        PORTABLE_SCHEMA_TYPES.contains(&declared),
        "{path}: unsupported type `{declared}`"
    );
    if let Some(properties) = map.get("properties").and_then(Value::as_object) {
        for (name, property) in properties {
            assert_portable_schema_node(property, &format!("{path}.{name}"));
        }
    }
    if let Some(items) = map.get("items") {
        assert_portable_schema_node(items, &format!("{path}[]"));
    }
}

/// The glob-pattern parameters advertise a plain string array while their Rust type
/// still accepts a bare string. Both forms must keep parsing: the array is what models
/// send once they read the published schema, the bare string is what every already
/// deployed caller sends.
#[test]
fn glob_pattern_parameters_accept_both_the_advertised_array_and_a_bare_string() {
    for pattern in [
        serde_json::json!(["**/*.rs", "!tests/**"]),
        "**/*.rs".into(),
    ] {
        serde_json::from_value::<fastctx::glob_tool::GlobRequest>(serde_json::json!({
            "pattern": pattern,
        }))
        .unwrap_or_else(|error| panic!("glob.pattern rejected {pattern}: {error}"));
        serde_json::from_value::<fastctx::grep_tool::GrepRequest>(serde_json::json!({
            "pattern": "needle",
            "glob": pattern,
        }))
        .unwrap_or_else(|error| panic!("grep.glob rejected {pattern}: {error}"));
        serde_json::from_value::<fastctx::edit::ReplaceRequest>(serde_json::json!({
            "pattern": "needle",
            "replacement": "thread",
            "path": "/tmp/file.txt",
            "glob": pattern,
        }))
        .unwrap_or_else(|error| panic!("replace.glob rejected {pattern}: {error}"));
    }
}

#[test]
fn replace_description_and_schema_preserve_the_file_editing_contract() {
    let tools = FastCtxServer::new().tool_definitions();
    let replace = tools.iter().find(|tool| tool.name == "replace").unwrap();
    assert_eq!(
        replace.description.as_deref(),
        Some(concat!(
            "Batch find-and-replace across a file or directory (Rust regex, same engine\n",
            "as grep; no lookaround). A reference to an undefined capture group is\n",
            "rejected before any write. To delete whole lines, include \\n in the\n",
            "pattern. Matching is leftmost-first and non-overlapping; unlike grep,\n",
            "`^`/`$` anchor the whole file by default — use (?m) for per-line anchors.\n",
            "Respects .gitignore; skips .git and binaries; files whose encoding cannot\n",
            "be determined are skipped and listed. Each file is written atomically with\n",
            "a concurrent-modification check, preserving its original encoding, BOM, and\n",
            "line endings. The last line states Complete or Partial."
        ))
    );

    assert_eq!(
        replace.input_schema["required"],
        serde_json::json!(["pattern", "replacement", "path"])
    );
    for property in [
        "glob",
        "literal",
        "case_insensitive",
        "dot_all",
        "max_replacements",
        "dry_run",
        "encoding",
        "fallback_encoding",
    ] {
        assert!(
            replace.input_schema["properties"].get(property).is_some(),
            "{property}"
        );
    }
    assert_positive_local_path_schema(&replace.input_schema["properties"]["path"]);
}

#[test]
fn filesystem_tools_publish_explicit_mutation_parameters() {
    let tools = FastCtxServer::new().tool_definitions();
    for (name, required, paths) in [
        ("delete", vec!["paths"], vec!["paths"]),
        (
            "move",
            vec!["source", "destination"],
            vec!["source", "destination"],
        ),
        ("create_file", vec!["path", "content"], vec!["path"]),
    ] {
        let tool = tools.iter().find(|tool| tool.name == name).unwrap();
        assert_eq!(tool.input_schema["required"], serde_json::json!(required));
        for path in paths {
            assert_positive_local_path_schema(&tool.input_schema["properties"][path]);
        }
    }
    let delete = tools.iter().find(|tool| tool.name == "delete").unwrap();
    assert_eq!(delete.input_schema["properties"]["paths"]["type"], "array");
    assert_eq!(
        delete.input_schema["properties"]["paths"]["items"]["type"],
        "string"
    );
    assert!(
        delete
            .description
            .as_deref()
            .unwrap()
            .contains("dry_run=true")
    );
}

fn assert_positive_local_path_schema(schema: &Value) {
    let description = schema["description"]
        .as_str()
        .expect("path schema should describe the accepted local path shape");
    for required in [
        "Plain absolute local filesystem path",
        "URI-shaped",
        "equivalent local absolute path",
    ] {
        assert!(description.contains(required), "{required}: {description}");
    }
    for forbidden in [
        "read_mcp_resource",
        "list_mcp_resources",
        "list_mcp_resource_templates",
        "MCP resources",
        "file://",
        "FastCtx is not",
    ] {
        assert!(
            !description.contains(forbidden),
            "{forbidden}: {description}"
        );
    }
}
