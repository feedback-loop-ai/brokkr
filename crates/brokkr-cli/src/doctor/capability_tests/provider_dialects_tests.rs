use super::*;

#[test]
fn doctor_names_each_provider_dialect_and_its_shared_offices() {
    let dir = workspace_with(None);
    let root = dir.path();
    write(
        root,
        "adapters/claude.json",
        &adapter("claude", Some(native(json!({"argv": ["--no-search"]})))),
    );
    write(
        root,
        "dialects/tools/claude-search.json",
        &native_dialect("claude-search", "claude"),
    );
    let private = realm(
        "private",
        Some(json!({
            "web-search": {
                "dialects": {"codex": "codex-native-search", "claude": "claude-search"},
                "offices": ["researcher", "reviewer"]
            }
        })),
    );
    write(
        root,
        "realms.json",
        &json!({"schema": "forge.realms/v9", "journal": "forge.db", "realms": [private]}),
    );
    let (healthy, lines) = lines(root, &installed(&["claude", "codex"]));
    assert!(healthy, "{lines:#?}");
    let shown: Vec<_> = lines
        .iter()
        .filter(|line| line.contains("'web-search':"))
        .cloned()
        .collect();
    let held = |provider, dialect| {
        format!(
            "ok       capabilities private native {provider} 'web-search': granted to offices \
             [researcher, reviewer] only through dialect '{dialect}'; the adapter-level plan of a \
             seat of office 'researcher' that wants it is admitted with it ON; for a seat that \
             does not hold it, {OFF} · {EVIDENCE}"
        )
    };
    assert_eq!(
        shown,
        [
            "ok       capabilities private 'web-search': provider 'claude' through dialect \
             'claude-search', provider 'codex' through dialect 'codex-native-search' · offices \
             [researcher, reviewer] only"
                .to_string(),
            held("claude", "claude-search"),
            held("codex", "codex-native-search"),
        ]
    );
}
