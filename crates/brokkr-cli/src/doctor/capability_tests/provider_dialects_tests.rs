use super::*;

fn doctor_over_a_v9_grant(root: &Path, grant: Value, providers: &[&str]) -> (bool, Vec<String>) {
    let private = realm("private", Some(json!({"web-search": grant})));
    write(
        root,
        "realms.json",
        &json!({"schema": "forge.realms/v9", "journal": "forge.db", "realms": [private]}),
    );
    lines(root, &installed(providers))
}

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
    let (healthy, lines) = doctor_over_a_v9_grant(
        root,
        json!({
            "dialects": {"codex": "codex-native-search", "claude": "claude-search"},
            "offices": ["researcher", "reviewer"]
        }),
        &["claude", "codex"],
    );
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

#[test]
fn doctor_reports_the_names_and_refusal_of_an_invalid_provider_grant() {
    let dir = workspace_with(None);
    let root = dir.path();
    let (healthy, lines) = doctor_over_a_v9_grant(
        root,
        json!({
            "dialects": {"codex": "codex-native-search", "claude": "absent"},
            "offices": ["researcher"]
        }),
        &[],
    );
    assert!(!healthy, "{lines:#?}");
    assert_eq!(
        lines.iter().find(|line| line.contains("'web-search':")).map(String::as_str),
        Some("MISSING  capabilities private 'web-search': provider 'claude' through dialect 'absent', \
              provider 'codex' through dialect 'codex-native-search' · offices [researcher] only · \
              realm 'private': capability 'web-search': tool dialect 'absent' is not at \
              'dialects/tools/absent.json' in the operator configuration")
    );
}

#[test]
fn doctor_notes_unmeasured_inventory_on_the_provider_grant() {
    let dir = workspace_with(None);
    let root = dir.path();
    write(
        root,
        "dialects/tools/dsh-search.json",
        &native_dialect("dsh-search", "dsh"),
    );
    let (healthy, lines) = doctor_over_a_v9_grant(
        root,
        json!({
            "dialects": {"codex": "codex-native-search", "dsh": "dsh-search"},
            "offices": ["researcher"]
        }),
        &[],
    );
    assert!(healthy, "{lines:#?}");
    assert_eq!(
        lines
            .iter()
            .find(|line| line.contains("'web-search':"))
            .map(String::as_str),
        Some(
            "ok       capabilities private 'web-search': provider 'codex' through dialect \
              'codex-native-search', provider 'dsh' through dialect 'dsh-search' · offices \
              [researcher] only · provider 'dsh' declares its native capabilities unmeasured \
              (unsupported mcp proves no absence of native egress): no seat can hold the \
              capability through this grant, and no native denial is claimed"
        )
    );
}
