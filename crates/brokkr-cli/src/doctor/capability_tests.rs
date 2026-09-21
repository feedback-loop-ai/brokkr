//! Decision 0065 ruling 4's readout, proved without a live provider: the
//! tests SUPPLY which harnesses are installed, and assert the complete
//! lines. Nothing here runs a model, a search, a fetch or a server, and no
//! line claims that one was observed.

use super::*;
use serde_json::{json, Value};

fn write(root: &Path, relative: &str, value: &Value) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn adapter(provider: &str, native: Option<Value>) -> Value {
    let mut adapter = json!({
        "provider": provider, "binary": provider, "driver": [provider], "models": {},
        "model_flag": "--model", "efforts": [], "effort_flag": "unsupported",
        "tool_permissions": "unsupported", "mcp": "unsupported",
    });
    if let Some(native) = native {
        adapter["native_capabilities"] = native;
    }
    adapter
}

fn native(off: Value) -> Value {
    json!({"known": {"web-search": {
        "capability": "web-search", "tools": ["web_search"],
        "on": {"default": "measured on by default"}, "off": off,
        "restrictions": {"unsupported": "no transport"},
        "evidence": {"source": "a measurement", "scope": "cold exec on 0.154.0 only",
                     "limitations": ["a RESUMED session is unmeasured"]}
    }}})
}

fn native_dialect(name: &str, provider: &str) -> Value {
    json!({
        "schema": "brokkr.tool-dialect/v1", "name": name, "serves": "web-search",
        "kind": "provider-native", "provider": provider, "adapter_key": "web-search",
        "tools": ["web_search"],
        "sends": {"description": "a query the model composes", "seat_composed": true},
        "restrictions": {"type": "object", "additionalProperties": false, "properties": {
            "allow": {"type": "object"}}}
    })
}

/// A workspace with five harness declarations: a measured OFF switch, a
/// measured impossible one, an OFF nobody tried, an unmeasured inventory,
/// and an adapter written before the ruling.
fn workspace_with(realms: Option<Value>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (provider, declared) in [
        (
            "codex",
            Some(native(json!({"argv": ["-c", "web_search=\"disabled\""]}))),
        ),
        (
            "stuck",
            Some(native(json!({"unsupported": "9.9 has no switch for it"}))),
        ),
        (
            "untried",
            Some(native(json!({"unmeasured": "nobody has tried"}))),
        ),
        (
            "dsh",
            Some(json!({"unmeasured": "unsupported mcp proves no absence of native egress"})),
        ),
        ("older", None),
    ] {
        write(
            root,
            &format!("adapters/{provider}.json"),
            &adapter(provider, declared),
        );
    }
    write(
        root,
        "capabilities/web-search.json",
        &json!({"name": "web-search", "classes": ["reads", "egress"]}),
    );
    for (name, provider) in [("codex-native-search", "codex"), ("stuck-search", "stuck")] {
        write(
            root,
            &format!("dialects/tools/{name}.json"),
            &native_dialect(name, provider),
        );
    }
    if let Some(realms) = realms {
        write(
            root,
            "realms.json",
            &json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": realms}),
        );
    }
    dir
}

fn realm(name: &str, capabilities: Option<Value>) -> Value {
    let mut realm = json!({"name": name, "path": name, "default_branch": "main"});
    if let Some(capabilities) = capabilities {
        realm["capabilities"] = capabilities;
    }
    realm
}

fn installed(providers: &[&str]) -> Availability {
    let mut availability = Availability::unspecified();
    for provider in ["codex", "stuck", "untried", "dsh", "older"] {
        availability.record(
            provider,
            match providers.contains(&provider) {
                true => Presence::Available,
                false => Presence::Unavailable,
            },
        );
    }
    availability
}

fn lines(dir: &Path, availability: &Availability) -> (bool, Vec<String>) {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    let world = brokkr_runtime::realms::World::inspect(dir, None);
    report_capabilities(
        &mut report,
        &world,
        dir,
        &dir.join("adapters"),
        availability,
    );
    (report.healthy, report.lines)
}

const EVIDENCE: &str =
    "evidence: cold exec on 0.154.0 only · still unmeasured: a RESUMED session is unmeasured";

#[test]
fn every_realm_reads_its_own_grants_and_its_neighbours_stay_out_of_it() {
    let dir = workspace_with(Some(json!([
        realm("private", None),
        realm(
            "public",
            Some(json!({"web-search": {
            "dialect": "codex-native-search", "offices": ["researcher"]}}))
        ),
    ])));
    let (healthy, lines) = lines(dir.path(), &installed(&["codex"]));
    assert!(healthy);
    assert_eq!(
        lines,
        [
            "ok       capabilities private: grants nothing; every native capability is \
             governed by the no-grant default — switched off, or the seat is refused"
                .to_string(),
            format!(
                "warn     capabilities private native codex 'web-search': NOT granted here: \
                 every seat on codex is launched with it switched off by the adapter's \
                 declared control · {EVIDENCE}"
            ),
            "ok       capabilities public 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [web_search] · offices [researcher] \
             only · restrictions none"
                .to_string(),
            format!(
                "ok       capabilities public native codex 'web-search': granted to offices \
                 [researcher] only through dialect 'codex-native-search'; every other seat on \
                 codex is launched with it switched off · {EVIDENCE}"
            ),
        ]
    );
}

#[test]
fn an_empty_scope_never_reads_as_all_and_a_restriction_is_not_usable_authority() {
    let grant = |extra: Value| {
        let mut grant = json!({"dialect": "codex-native-search"});
        grant
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        Some(json!({"web-search": grant}))
    };
    let dir = workspace_with(Some(json!([
        realm("all", grant(json!({}))),
        realm(
            "two",
            grant(json!({"offices": ["review-security", "researcher"]}))
        ),
        realm("none", grant(json!({"offices": [], "tools": []}))),
        realm(
            "restricted",
            grant(json!({"allow": {"hosts": ["yaml.org"]}}))
        ),
    ])));
    let (_, lines) = lines(dir.path(), &installed(&[]));
    assert_eq!(
        lines,
        [
            "ok       capabilities all 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [web_search] · all requesting offices \
             · restrictions none",
            "ok       capabilities two 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [web_search] · offices \
             [review-security, researcher] only · restrictions none",
            "ok       capabilities none 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [] · no offices · restrictions none",
            "ok       capabilities restricted 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [web_search] · all requesting offices \
             · restrictions {\"allow\":{\"hosts\":[\"yaml.org\"]}} · provider 'codex' cannot \
             express restriction 'allow.hosts': a seat that requires the capability is \
             refused, one that wants it drops it with the native capability OFF, and it never \
             runs unrestricted",
        ]
    );
}

#[test]
fn installed_harnesses_are_told_apart_switchable_impossible_untried_and_unmeasured() {
    // A grant bound to ANOTHER provider covers nothing of codex's.
    let dir = workspace_with(Some(json!([realm(
        "private",
        Some(json!({"web-search": {"dialect": "stuck-search"}}))
    )])));
    let (healthy, lines) = lines(
        dir.path(),
        &installed(&["codex", "stuck", "untried", "dsh", "older"]),
    );
    assert!(healthy);
    assert_eq!(
        lines[1..],
        [
            format!(
                "warn     capabilities private native codex 'web-search': NOT granted here: \
                 every seat on codex is launched with it switched off by the adapter's \
                 declared control · {EVIDENCE}"
            ),
            "warn     capabilities private native dsh: native inventory unmeasured: \
             unsupported mcp proves no absence of native egress. Nothing is granted through \
             it and no native denial is claimed"
                .to_string(),
            "warn     capabilities private native older: native inventory unmeasured: the \
             adapter declares no native_capabilities assessment. Nothing is granted through \
             it and no native denial is claimed"
                .to_string(),
            format!(
                "ok       capabilities private native stuck 'web-search': granted to all \
                 requesting offices through dialect 'stuck-search'; every other seat on stuck \
                 is launched with it switched off · {EVIDENCE}"
            ),
            "warn     capabilities private native untried 'web-search': NOT granted here, \
             and its OFF control is unmeasured (nobody has tried); no denial is claimed"
                .to_string(),
        ]
    );
    // Ungranted, the impossible OFF predicts the compile refusal.
    let bare = workspace_with(Some(json!([realm("private", None)])));
    let (_, lines) = self::lines(bare.path(), &installed(&["stuck"]));
    assert_eq!(
        lines[1],
        "warn     capabilities private native stuck 'web-search': NOT granted here, and it \
         cannot be switched off (9.9 has no switch for it): seating stuck in this realm \
         without granting it refuses compilation (decision 0065 ruling 4)"
    );
    // An absent binary is not an installed, denied capability.
    let (_, lines) = self::lines(bare.path(), &installed(&[]));
    assert_eq!(lines.len(), 1, "{lines:?}");
}

#[test]
fn no_map_is_a_visible_denial_default_and_a_broken_map_is_unknown_authority() {
    let dir = workspace_with(None);
    let (healthy, lines) = lines(dir.path(), &installed(&[]));
    assert!(healthy);
    assert_eq!(
        lines,
        [
            "ok       capabilities <unmapped>: no realms map, so no capability grants are \
          declared; every native capability is governed by the no-grant default — switched \
          off, or the seat is refused"
        ]
    );
    std::fs::write(dir.path().join("realms.json"), "not json").unwrap();
    let (_, lines) = self::lines(dir.path(), &installed(&["dsh"]));
    assert_eq!(
        lines,
        [
            "warn     capabilities: the realms map could not be read, so what each realm \
             grants is UNKNOWN; nothing is assumed granted",
            "warn     capabilities <unknown> native dsh: native inventory unmeasured: \
             unsupported mcp proves no absence of native egress. Nothing is granted through \
             it and no native denial is claimed",
        ]
    );
    // Adapters that do not load were already reported by the provider
    // probe; the capability lines simply name no harness.
    std::fs::write(dir.path().join("adapters/codex.json"), "not json").unwrap();
    std::fs::remove_file(dir.path().join("realms.json")).unwrap();
    assert_eq!(self::lines(dir.path(), &installed(&["codex"])).1.len(), 1);
}

#[test]
fn an_unbuilt_or_invalid_grant_is_one_failing_line_and_the_rest_still_prints() {
    let dir = workspace_with(Some(json!([
        realm(
            "broken",
            Some(json!({"library-docs": {"dialect": "docs-mcp"}}))
        ),
        realm(
            "conflicting",
            Some(json!({"web-search": {"dialect": "reads-only"}}))
        ),
        realm("sound", None),
        realm(
            "unbuilt",
            Some(json!({"library-docs": {"dialect": "docs-mcp"}}))
        ),
    ])));
    let (healthy, first) = lines(dir.path(), &installed(&["dsh"]));
    assert!(!healthy);
    assert_eq!(
        first[0],
        "MISSING  capabilities broken: realm 'broken': capability 'library-docs' has no \
         abstract definition at 'capabilities/library-docs.json' in the operator \
         configuration; declare its classes before granting it"
    );
    // A definition is not a grant, and an MCP declaration is not a server.
    write(
        dir.path(),
        "capabilities/library-docs.json",
        &json!({"name": "library-docs", "classes": ["reads", "egress"]}),
    );
    write(
        dir.path(),
        "dialects/tools/docs-mcp.json",
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp",
                "serves": "library-docs", "kind": "mcp",
                "connection": {"argv": ["/nonexistent/docs-mcp"]}, "version": "1.0.0",
                "secrets": [], "tools": ["read"],
                "sends": {"description": "a library name", "seat_composed": true}}),
    );
    let mut reads_only = native_dialect("reads-only", "codex");
    reads_only["classes"] = json!(["reads"]);
    write(dir.path(), "dialects/tools/reads-only.json", &reads_only);
    let (_, second) = lines(dir.path(), &installed(&["dsh"]));
    let dsh = |realm: &str| {
        format!(
            "warn     capabilities {realm} native dsh: native inventory unmeasured: \
             unsupported mcp proves no absence of native egress. Nothing is granted through \
             it and no native denial is claimed"
        )
    };
    assert_eq!(
        second,
        [
            "MISSING  capabilities broken: realm 'broken' grants capability 'library-docs' \
             through dialect 'docs-mcp' of kind 'mcp', whose broker support is not \
             implemented until decision 0065 slice two"
                .to_string(),
            dsh("broken"),
            "MISSING  capabilities conflicting: realm 'conflicting': capability 'web-search' \
             in dialect 'reads-only' declares classes [reads], conflicting with abstract \
             definition 'capabilities/web-search.json' classes [reads, egress]"
                .to_string(),
            dsh("conflicting"),
            "ok       capabilities sound: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused"
                .to_string(),
            dsh("sound"),
            "MISSING  capabilities unbuilt: realm 'unbuilt' grants capability 'library-docs' \
             through dialect 'docs-mcp' of kind 'mcp', whose broker support is not \
             implemented until decision 0065 slice two"
                .to_string(),
            dsh("unbuilt"),
        ]
    );
}

/// The shipped repository, read as doctor reads it: the realm grants
/// nothing, Codex's search is named with its cold-0.154.0 scope and its
/// resumed gap, Claude's two tools are named as adapter data, and DSH and
/// LaneTally stay unmeasured on their own terms.
#[test]
fn the_shipped_realm_grants_nothing_and_names_every_native_power_it_denies() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut availability = Availability::unspecified();
    for provider in ["claude", "codex", "dsh", "lanetally"] {
        availability.record(provider, Presence::Available);
    }
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    let world = brokkr_runtime::realms::World::inspect(&workspace, None);
    report_capabilities(
        &mut report,
        &world,
        &workspace,
        &workspace.join("adapters"),
        &availability,
    );
    assert!(report.healthy);
    let rendered = report.render();
    assert!(
        rendered.starts_with("ok       capabilities brokkr: grants nothing;"),
        "{rendered}"
    );
    for expected in [
        "warn     capabilities brokkr native codex 'web-search': NOT granted here: every seat \
         on codex is launched with it switched off by the adapter's declared control · \
         evidence: codex-cli 0.154.0, cold `codex exec` only",
        "whether the OFF switch holds on a RESUMED codex session is unmeasured",
        "warn     capabilities brokkr native claude 'web-fetch': NOT granted here",
        "warn     capabilities brokkr native claude 'web-search': NOT granted here",
        "no live denial or enablement of WebSearch has been measured",
        "warn     capabilities brokkr native dsh: native inventory unmeasured: dsh declares \
         mcp and tool_permissions unsupported",
        "warn     capabilities brokkr native lanetally: native inventory unmeasured: the \
         LaneTally wrapper forwards argv to claude",
    ] {
        assert!(rendered.contains(expected), "{expected}\n---\n{rendered}");
    }
}
