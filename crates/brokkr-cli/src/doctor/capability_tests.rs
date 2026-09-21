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

/// A harness whose native power is switched through its own tool lists,
/// as Claude Code's are: the control is a selection, not an argv pair.
fn selecting() -> Value {
    let list = |flag: &str| json!({"flag": flag, "separator": ","});
    json!({
        "known": {"web-fetch": {
            "capability": "web-fetch", "tools": ["WebFetch"],
            "on": {"selection": {"include": ["WebFetch"], "allow": ["WebFetch"], "deny": []}},
            "off": {"selection": {"include": [], "allow": [], "deny": ["WebFetch"]}},
            "restrictions": {"unsupported": "no transport"},
            "evidence": {"source": "a measurement", "scope": "cold exec on 0.154.0 only",
                         "limitations": ["a RESUMED session is unmeasured"]}
        }},
        "selection": {"include": list("--tools"), "allow": list("--allowedTools"),
                      "deny": list("--disallowedTools")}
    })
}

/// A workspace with six harness declarations: a tool-list selection, a
/// measured OFF switch, a measured impossible one, an OFF nobody tried, an
/// unmeasured inventory, and an adapter written before the ruling.
fn workspace_with(realms: Option<Value>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (provider, declared) in [
        ("claude", Some(selecting())),
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
    for provider in ["claude", "codex", "stuck", "untried", "dsh", "older"] {
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
    // Design D8: a map that cannot be read names no realm, so no empty
    // grant set is invented for one. Every installed harness is still
    // ASSESSED — what it has, how ON and OFF are declared, the evidence
    // limits — and not one line says granted, not granted or switched off.
    std::fs::write(dir.path().join("realms.json"), "not json").unwrap();
    let (healthy, lines) = self::lines(
        dir.path(),
        &installed(&["claude", "codex", "stuck", "untried", "dsh"]),
    );
    assert!(healthy, "the map's own failing line is the realm readout's");
    let unknown = "whether any realm grants it is UNKNOWN, so neither a grant nor a denial is \
                   claimed";
    assert_eq!(
        lines,
        [
            "warn     capabilities: the realms map could not be read, so what each realm \
             grants is UNKNOWN; nothing is assumed granted and nothing is assumed denied"
                .to_string(),
            format!(
                "warn     capabilities native claude 'web-fetch': declared by the adapter: \
                 tools [WebFetch] · ON: tool lists include [WebFetch] allow [WebFetch] deny [] \
                 · OFF: tool lists include [] allow [] deny [WebFetch] · {EVIDENCE} · {unknown}"
            ),
            format!(
                "warn     capabilities native codex 'web-search': declared by the adapter: \
                 tools [web_search] · ON: the harness default (measured on by default) · OFF: \
                 argv [-c web_search=\"disabled\"] · {EVIDENCE} · {unknown}"
            ),
            "warn     capabilities native dsh: native inventory unmeasured: unsupported mcp \
             proves no absence of native egress. No native grant or denial is claimed"
                .to_string(),
            format!(
                "warn     capabilities native stuck 'web-search': declared by the adapter: \
                 tools [web_search] · ON: the harness default (measured on by default) · OFF: \
                 cannot be switched (9.9 has no switch for it) · {EVIDENCE} · {unknown}"
            ),
            format!(
                "warn     capabilities native untried 'web-search': declared by the adapter: \
                 tools [web_search] · ON: the harness default (measured on by default) · OFF: \
                 unmeasured (nobody has tried) · {EVIDENCE} · {unknown}"
            ),
        ]
    );
}

/// `Adapters::load` reads the directory whole, so one malformed file
/// takes every native line with it. That absence is SAID, with the
/// loader's reason, and the grants — which need no adapter to be read —
/// are still shown.
#[test]
fn unreadable_adapter_declarations_are_said_so_and_never_read_as_no_native_capability() {
    let dir = workspace_with(Some(json!([realm(
        "private",
        Some(json!({"web-search": {"dialect": "codex-native-search"}}))
    )])));
    std::fs::write(dir.path().join("adapters/codex.json"), "not json").unwrap();
    let adapters = dir.path().join("adapters");
    let reason = Adapters::load(&adapters).unwrap_err();
    let (healthy, lines) = lines(dir.path(), &installed(&["claude", "codex"]));
    assert!(healthy, "a warning, on the provider probe's own terms");
    assert_eq!(
        lines,
        [
            format!(
                "warn     capabilities native: the adapter declarations at {} could not be \
                 read ({reason}), so no harness's native capabilities are named below; that is \
                 NOT a finding that an installed harness has none",
                adapters.display()
            ),
            "ok       capabilities private 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [web_search] · all requesting offices \
             · restrictions none"
                .to_string(),
        ]
    );
    assert!(reason.to_string().contains("codex.json"), "{reason}");
}

/// A grant bound to a provider whose inventory is unmeasured loads — the
/// map is valid — and can be held by nobody. The declaration is shown as
/// declared, with that said beside it.
#[test]
fn a_grant_bound_to_an_unmeasured_inventory_is_shown_as_declared_and_not_as_usable() {
    let dir = workspace_with(Some(json!([realm(
        "private",
        Some(json!({"web-search": {"dialect": "dsh-search"}}))
    )])));
    write(
        dir.path(),
        "dialects/tools/dsh-search.json",
        &native_dialect("dsh-search", "dsh"),
    );
    let (healthy, lines) = lines(dir.path(), &installed(&["dsh"]));
    assert!(healthy);
    assert_eq!(
        lines,
        [
            "ok       capabilities private 'web-search': dialect 'dsh-search' (provider-native, \
             provider 'dsh') · tools [web_search] · all requesting offices · restrictions none \
             · provider 'dsh' declares its native capabilities unmeasured (unsupported mcp \
             proves no absence of native egress): no seat can hold the capability through this \
             grant, and no native denial is claimed",
            "warn     capabilities private native dsh: native inventory unmeasured: \
             unsupported mcp proves no absence of native egress. Nothing is granted through \
             it and no native denial is claimed",
        ]
    );
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
        "MISSING  capabilities broken 'library-docs': realm 'broken': capability 'library-docs' has no \
         abstract definition at 'capabilities/library-docs.json' in the operator \
         configuration; declare its classes before granting it"
    );
    // A definition is not a grant, and an MCP declaration is not a server.
    write(
        dir.path(),
        "capabilities/library-docs.json",
        &json!({"name": "library-docs", "classes": ["reads", "egress"]}),
    );
    docs_mcp(dir.path());
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
            "MISSING  capabilities broken 'library-docs': realm 'broken' grants capability \
             'library-docs' through dialect 'docs-mcp' of kind 'mcp', whose broker support is \
             not implemented until decision 0065 slice two"
                .to_string(),
            dsh("broken"),
            "MISSING  capabilities conflicting 'web-search': realm 'conflicting': capability \
             'web-search' in dialect 'reads-only' declares classes [reads], conflicting with \
             abstract definition 'capabilities/web-search.json' classes [reads, egress]"
                .to_string(),
            dsh("conflicting"),
            "ok       capabilities sound: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused"
                .to_string(),
            dsh("sound"),
            "MISSING  capabilities unbuilt 'library-docs': realm 'unbuilt' grants capability \
             'library-docs' through dialect 'docs-mcp' of kind 'mcp', whose broker support is \
             not implemented until decision 0065 slice two"
                .to_string(),
            dsh("unbuilt"),
        ]
    );
}

fn docs_mcp(root: &Path) {
    write(
        root,
        "dialects/tools/docs-mcp.json",
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp",
                "serves": "library-docs", "kind": "mcp",
                "connection": {"argv": ["/nonexistent/docs-mcp"]}, "version": "1.0.0",
                "secrets": [], "tools": ["read"],
                "sends": {"description": "a library name", "seat_composed": true}}),
    );
}

/// One realm, three grants, two of them refused: each refusal is its own
/// failing line with the compiler's whole reason, the grant that validates
/// is still shown and still covers Codex's search, and Claude's fetch —
/// whose grant is the one that could not be read — is UNKNOWN there, never
/// "not granted" and never "switched off".
#[test]
fn a_failing_grant_takes_no_neighbour_with_it_and_fabricates_no_denial() {
    let dir = workspace_with(Some(json!([
        realm(
            "mixed",
            Some(json!({
                "library-docs": {"dialect": "docs-mcp"},
                "web-fetch": {"dialect": "absent-fetch"},
                "web-search": {"dialect": "codex-native-search", "offices": ["researcher"]},
            }))
        ),
        realm("sound", None),
    ])));
    for name in ["library-docs", "web-fetch"] {
        write(
            dir.path(),
            &format!("capabilities/{name}.json"),
            &json!({"name": name, "classes": ["reads", "egress"]}),
        );
    }
    docs_mcp(dir.path());
    let (healthy, lines) = lines(dir.path(), &installed(&["claude", "codex"]));
    assert!(!healthy);
    assert_eq!(
        lines,
        [
            "MISSING  capabilities mixed 'library-docs': realm 'mixed' grants capability \
             'library-docs' through dialect 'docs-mcp' of kind 'mcp', whose broker support is \
             not implemented until decision 0065 slice two"
                .to_string(),
            "MISSING  capabilities mixed 'web-fetch': realm 'mixed': capability 'web-fetch': \
             tool dialect 'absent-fetch' is not at 'dialects/tools/absent-fetch.json' in the \
             operator configuration"
                .to_string(),
            "ok       capabilities mixed 'web-search': dialect 'codex-native-search' \
             (provider-native, provider 'codex') · tools [web_search] · offices [researcher] \
             only · restrictions none"
                .to_string(),
            format!(
                "warn     capabilities mixed native claude 'web-fetch': UNKNOWN here: this \
                 realm's grant of 'web-fetch' did not validate (its failing line is above), so \
                 neither a grant nor a denial is claimed, and no seat compiles in this realm \
                 until it is repaired · {EVIDENCE}"
            ),
            format!(
                "ok       capabilities mixed native codex 'web-search': granted to offices \
                 [researcher] only through dialect 'codex-native-search'; every other seat on \
                 codex is launched with it switched off · {EVIDENCE}"
            ),
            "ok       capabilities sound: grants nothing; every native capability is governed \
             by the no-grant default — switched off, or the seat is refused"
                .to_string(),
            format!(
                "warn     capabilities sound native claude 'web-fetch': NOT granted here: \
                 every seat on claude is launched with it switched off by the adapter's \
                 declared control · {EVIDENCE}"
            ),
            format!(
                "warn     capabilities sound native codex 'web-search': NOT granted here: \
                 every seat on codex is launched with it switched off by the adapter's \
                 declared control · {EVIDENCE}"
            ),
        ]
    );
}

/// Malformed operator JSON is its own failing line, carrying the loader's
/// reason, and everything that does not depend on it is still printed: a
/// definition file refuses every compile and is said once; a dialect file
/// fails only the grant that selected it.
#[test]
fn malformed_definition_and_dialect_json_are_their_own_failing_lines() {
    use brokkr_runtime::capabilities::{Definitions, ToolDialect};
    let realms = || {
        Some(json!([
            realm("private", None),
            realm(
                "public",
                Some(json!({"web-search": {"dialect": "codex-native-search"}}))
            ),
        ]))
    };
    let private = [
        "ok       capabilities private: grants nothing; every native capability is governed \
         by the no-grant default — switched off, or the seat is refused"
            .to_string(),
        format!(
            "warn     capabilities private native codex 'web-search': NOT granted here: every \
             seat on codex is launched with it switched off by the adapter's declared control \
             · {EVIDENCE}"
        ),
    ];
    let unknown = format!(
        "warn     capabilities public native codex 'web-search': UNKNOWN here: this realm's \
         grant of 'web-search' did not validate (its failing line is above), so neither a \
         grant nor a denial is claimed, and no seat compiles in this realm until it is \
         repaired · {EVIDENCE}"
    );

    let dir = workspace_with(realms());
    std::fs::write(dir.path().join("capabilities/web-search.json"), "not json").unwrap();
    let reason = Definitions::load(dir.path()).unwrap_err();
    assert!(reason.contains("capabilities/web-search.json"), "{reason}");
    let (healthy, lines) = self::lines(dir.path(), &installed(&["codex"]));
    assert!(!healthy);
    assert_eq!(
        lines,
        [
            format!(
                "MISSING  capabilities definitions: {reason}; no grant can be validated and \
                 every compile under this configuration refuses until it is repaired"
            ),
            private[0].clone(),
            private[1].clone(),
            format!("MISSING  capabilities public 'web-search': {reason}"),
            unknown.clone(),
        ]
    );

    let dir = workspace_with(realms());
    std::fs::write(
        dir.path().join("dialects/tools/codex-native-search.json"),
        "not json",
    )
    .unwrap();
    let reason = ToolDialect::load(dir.path(), "codex-native-search").unwrap_err();
    assert!(reason.contains("codex-native-search.json"), "{reason}");
    let (healthy, lines) = self::lines(dir.path(), &installed(&["codex"]));
    assert!(!healthy);
    assert_eq!(
        lines,
        [
            private[0].clone(),
            private[1].clone(),
            format!(
                "MISSING  capabilities public 'web-search': realm 'public': capability \
                 'web-search': {reason}"
            ),
            unknown,
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
    // `exec` is installed wherever a POSIX `sh` is, which the provider
    // probe records like any other binary: it is named on the same terms.
    for provider in ["claude", "codex", "dsh", "exec", "lanetally"] {
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
    // Design D6: generic exec dispatch cannot certify an arbitrary child
    // program's native inventory, and the readout says so in full.
    let exec = "warn     capabilities brokkr native exec: native inventory unmeasured: exec runs \
                whatever command the bundle names; the engine cannot certify what an arbitrary \
                child program reaches on its own, so its native inventory is unmeasured. Its \
                hands, boundary and command authority are decisions 0043 and 0046's, unchanged. \
                Nothing is granted through it and no native denial is claimed";
    assert_eq!(
        report
            .lines
            .iter()
            .filter(|line| line.contains(" native exec"))
            .collect::<Vec<_>>(),
        [exec]
    );
}
